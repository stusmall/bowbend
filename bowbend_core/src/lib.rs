#![warn(missing_docs)]
//! This is the cross language core of the portscanner.  All of the real logic
//! and functionality of the project should be contained here.  It's only
//! consumer is the FFI crate which provides a C API to be consumed by each
//! language SDK.  The APIs of this crate are not public and will not be
//! kept stable

use std::ops::Range;

use futures::{stream, Stream};
use tracing::Level;
use tracing::instrument;
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

use crate::{
    icmp::icmp_sweep,
    report::Report,
    target::{targets_to_instance_stream, Target},
    tcp::full_open::full_open_port_scan,
    utils::throttle_stream::throttle_stream,
};

pub mod err;
mod icmp;
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
    ping: bool
) -> impl Stream<Item = Report> {
    let (target_stream, failed) = targets_to_instance_stream(targets);
    let throttled_stream = if let Some(range) = throttle_range {
        throttle_stream(range, target_stream)
    } else {
        throttle_stream(Range::default(), target_stream)
    };



    if let Ok(ping_result_stream) = icmp_sweep(throttled_stream).await {
        tracing::trace!("We have ping results back");
        let results = full_open_port_scan(Box::pin(ping_result_stream), port_list).await;
        tracing::trace!("We finished a full open port scan");

        // TODO: This is just here to make the compiler happier
        stream::iter(failed)
    } else {
        // TODO: This needs to be an actual report instead
        tracing::error!("This failed and is just aborting.  We need to send something back");
        panic!();
    }
}


pub fn setup_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::FULL)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}


