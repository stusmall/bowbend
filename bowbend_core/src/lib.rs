#![warn(missing_docs)]
//! This is the cross language core of the portscanner.  All of the real logic
//! and functionality of the project should be contained here.  It's only
//! consumer is the FFI crate which provides a C API to be consumed by each
//! language SDK.  The APIs of this crate are not public and will not be
//! kept stable

use std::{ops::Range, sync::Arc};

use futures::{stream, Stream};
use tokio::sync::Semaphore;
use tracing::{instrument, Level};
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

use crate::{
    err::PortscanErr,
    icmp::{icmp_sweep, skip_icmp},
    report::Report,
    stream::StreamExt,
    target::{targets_to_instance_stream, Target},
    tcp::full_open::full_open_port_scan,
    utils::throttle_stream::throttle_stream,
};

pub mod err;
pub mod icmp;
pub mod report;
pub mod target;
mod tcp;
pub(crate) mod utils;

/// The entry point to kick off a batch of portscans.  It will return a stream
/// of updates as events happens
#[instrument(level = "trace")]
pub async fn entry_point(
    targets: Vec<Target>,
    port_list: Vec<u16>,
    throttle_range: Option<Range<u64>>,
    ping: bool,
    max_in_flight: u32,
) -> Result<impl Stream<Item = Report>, PortscanErr> {
    let semaphore = Arc::new(Semaphore::new(max_in_flight as usize));
    let (target_stream, failed) = targets_to_instance_stream(targets);
    let throttled_stream = if let Some(ref range) = throttle_range {
        throttle_stream(range.clone(), target_stream).boxed()
    } else {
        target_stream.boxed()
    };
    let ping_result_stream = if ping {
        icmp_sweep(throttled_stream, semaphore.clone())
            .await?
            .boxed()
    } else {
        skip_icmp(throttled_stream).await?.boxed()
    };
    tracing::trace!("We have ping results back");
    let results = full_open_port_scan(
        Box::pin(ping_result_stream),
        port_list,
        semaphore,
        throttle_range,
    )
    .await;
    tracing::trace!("We finished a full open port scan");

    Ok(stream::iter(failed).chain(results.boxed()).boxed())
}

/// Set up the tracing module.  This dumps out detailed traces of the exact
/// code path to stdout.  This is only useful for internal development and not
/// to a consumer of the library.
pub fn setup_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::FULL)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
