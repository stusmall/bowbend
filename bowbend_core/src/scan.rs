use std::sync::Arc;

use futures::{stream, Stream, StreamExt};
use tokio::sync::Semaphore;
use tracing::trace;

use crate::{
    icmp::{icmp_sweep, skip_icmp},
    logging::setup_tracing,
    service_detection::run_service_detection_on_target,
    target::targets_to_instance_stream,
    tcp::full_open::full_open_port_scan,
    utils::throttle_stream::throttle_stream,
    ConfigBuilder, PortscanErr, Report,
};

/// The entry point to kick off a batch of portscans.  It will return a stream
/// of updates as events happens
pub async fn start_scan(
    config_builder: ConfigBuilder,
) -> Result<impl Stream<Item = Report>, PortscanErr> {
    if config_builder.tracing {
        setup_tracing()
    }
    let semaphore = Arc::new(Semaphore::new(config_builder.max_in_flight as usize));
    let (target_stream, failed) = targets_to_instance_stream(config_builder.targets);
    let throttled_stream = if let Some(ref range) = config_builder.throttle_range {
        throttle_stream(range.clone(), target_stream).boxed()
    } else {
        target_stream.boxed()
    };
    let ping_result_stream = if config_builder.ping {
        icmp_sweep(throttled_stream, semaphore.clone())
            .await
            .boxed()
    } else {
        skip_icmp(throttled_stream).await?.boxed()
    };
    trace!("We have ping results back");
    let results = full_open_port_scan(
        Box::pin(ping_result_stream),
        config_builder.ports,
        semaphore.clone(),
        config_builder.throttle_range.clone(),
    )
    .await;
    trace!("We finished a full open port scan");

    let results = if config_builder.run_service_detection {
        run_service_detection_on_target(results, semaphore.clone(), config_builder.throttle_range)
            .await
            .boxed()
    } else {
        results.boxed()
    };

    Ok(stream::iter(failed).chain(results).boxed())
}
