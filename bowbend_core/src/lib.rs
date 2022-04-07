#![warn(missing_docs)]
//! This is the cross language core of the portscanner.  All of the real logic
//! and functionality of the project should be contained here.  It's only
//! consumer is the FFI crate which provides a C API to be consumed by each
//! language SDK.  The APIs of this crate are not public and will not be
//! kept stable

use std::ops::Range;

use futures::{stream, Stream};

use crate::{
    icmp::icmp_sweep,
    report::Report,
    target::{targets_to_instance_stream, Target},
    tcp::full_open::full_open_port_scan,
    utils::throttle_stream::throttle_stream,
};

mod err;
mod icmp;
pub mod report;
pub mod target;
mod tcp;
pub(crate) mod utils;

/// The entry point to kick off a batch of portscans.  It will return a stream
/// of updates as events happens
#[tracing::instrument]
pub async fn entry_point(
    targets: Vec<Target>,
    port_list: Vec<u16>,
    throttle_range: Option<Range<u64>>,
) -> impl Stream<Item = Report> {
    let (target_stream, failed) = targets_to_instance_stream(targets);
    let throttled_stream = if let Some(range) = throttle_range {
        throttle_stream(range, target_stream)
    } else {
        throttle_stream(Range::default(), target_stream)
    };
    let ping_result_stream = icmp_sweep(throttled_stream).await.unwrap();

    let results = full_open_port_scan(Box::pin(ping_result_stream), port_list).await;
    println!("Our results {:#?}", results);

    // TODO: This is just here to make the compiler happier
    stream::iter(failed)
}
