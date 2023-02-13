//! The most basic of the TCP scan strategies.  It attempts to open a connection
//! on the targeted port by fully completely the TCP handshake.  This has a
//! couple downsides.  It is slower and it can generate noise in service's logs.
//! The big advantage is that it doesn't need privileged access to open raw
//! sockets, so can be run as normal user

use std::{collections::HashMap, io, net::SocketAddr, ops::Range, sync::Arc, time::Duration};

use futures::{future::join_all, FutureExt, Stream, StreamExt};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use tokio::{
    net::TcpStream,
    sync::Semaphore,
    task,
    time::{error::Elapsed, sleep, timeout},
};
use tracing::instrument;

use crate::{
    icmp::{PingResult, PingResultType},
    report::{PortReport, PortStatus, Report, ReportContents},
    stream,
    stream::FuturesUnordered,
    target::TargetInstance,
};

#[instrument(level = "trace", skip(input_stream))]
pub(crate) async fn full_open_port_scan(
    mut input_stream: impl Stream<Item = (TargetInstance, Option<PingResult>)> + Unpin,
    port_list: Vec<u16>,
    semaphore: Arc<Semaphore>,
    throttle_range: Option<Range<u64>>,
) -> impl Stream<Item = Report> {
    let futures = FuturesUnordered::new();
    let mut skipped = Vec::new();
    while let Some(target) = input_stream.next().await {
        let should_scan = target
            .1
            .as_ref()
            .map(|x| !matches!(x.result_type, PingResultType::Error(_)))
            .unwrap_or(true);
        if should_scan {
            let port_list = port_list.clone();
            let throttle_range = throttle_range.clone();
            let semaphore = semaphore.clone();
            let handle = task::spawn(async move {
                let _permit = Arc::clone(&semaphore).acquire_owned().await;
                scan_host(target.0, target.1, port_list, throttle_range).await
            });
            futures.push(handle);
        } else {
            skipped.push(Report {
                target: target.0.clone().into(),
                instance: Some(target.0),
                contents: Ok(ReportContents {
                    icmp: target.1,
                    ports: None,
                }),
            })
        }
    }

    let results = futures.map(|x| {
        // Right now if there is an error, we don't know what it is, why or what address
        // it is related to.  We need to find out if there is some way to tag
        // metadata along with the future in FuturesUnordered.  This should really only
        // come up if futures are canceled or failing to complete.  If we start
        // hitting it for other reasons this can get cleaned up.
        x.unwrap()
    });

    stream::select(results, stream::iter(skipped))
}

#[instrument(level = "trace")]
async fn scan_host(
    target: TargetInstance,
    ping_result: Option<PingResult>,
    mut ports: Vec<u16>,
    throttle_range: Option<Range<u64>>,
) -> Report {
    let mut rng = StdRng::from_entropy();
    let mut connection_futures = vec![];
    let ip = target.get_ip();
    ports.shuffle(&mut rng);
    for port in ports {
        let socket_addr = SocketAddr::new(ip, port);
        let connect_future = TcpStream::connect(socket_addr);
        let timout_future =
            timeout(Duration::from_millis(500), connect_future).map(move |result| (port, result));
        connection_futures.push(timout_future);
        if let Some(range) = throttle_range.clone() {
            sleep(Duration::from_millis(rng.gen_range(range))).await;
        }
    }
    let results: Vec<(u16, Result<io::Result<_>, Elapsed>)> = join_all(connection_futures).await;
    let ports: HashMap<u16, PortReport> =
        results
            .iter()
            .fold(HashMap::new(), |mut map, (port, result)| match result {
                Ok(Ok(_)) => {
                    map.insert(
                        *port,
                        PortReport {
                            port: *port,
                            status: PortStatus::Open,
                            service_detection_conclusions: None,
                        },
                    );
                    map
                }
                _ => {
                    map.insert(
                        *port,
                        PortReport {
                            port: *port,
                            status: PortStatus::Closed,
                            service_detection_conclusions: None,
                        },
                    );
                    map
                }
            });
    Report {
        target: target.clone().into(),
        instance: Some(target),
        contents: Ok(ReportContents {
            icmp: ping_result,
            ports: Some(ports),
        }),
    }
}
