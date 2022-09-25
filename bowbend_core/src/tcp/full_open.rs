use std::{io, net::SocketAddr, time::Duration};

use async_stream::stream;
use futures::{future::join_all, FutureExt, Stream, StreamExt};
use rand::{seq::SliceRandom, thread_rng};
use tokio::{
    net::TcpStream,
    time::{error::Elapsed, timeout},
};
use tracing::instrument;

use crate::{
    icmp::{PingResult, PingResultType},
    report::{PortReport, PortStatus, Report, ReportContents},
    target::TargetInstance,
};

#[instrument(level = "trace", skip(input_stream))]
pub(crate) async fn full_open_port_scan(
    mut input_stream: impl Stream<Item = (TargetInstance, Option<PingResult>)> + Unpin,
    port_list: Vec<u16>,
) -> impl Stream<Item = Report> {
    stream! {
        while let Some(target) = input_stream.next().await {
            let should_scan = target.1.as_ref().map(|x| {if let PingResultType::Error(_) = x.result_type {
                false
            }  else {
                true
            }}).unwrap_or(true);

            if should_scan {
                let result = scan_host(target.0, target.1, port_list.clone()).await;
                    yield result;
            } else {
                yield Report {
                        target: target.0.clone().into(),
                        instance: Some(target.0.get_ip()),
                        contents: Ok(ReportContents {
                            icmp: target.1,
                            ports: None,
                        }),
                    }
            };
        }
    }
}

#[instrument(level = "trace")]
async fn scan_host(
    target: TargetInstance,
    ping_result: Option<PingResult>,
    mut ports: Vec<u16>,
) -> Report {
    let mut connection_futures = vec![];
    let ip = target.get_ip();
    ports.shuffle(&mut thread_rng());
    for port in ports {
        let socket_addr = SocketAddr::new(ip, port);
        let connect_future = TcpStream::connect(socket_addr);
        let timout_future =
            timeout(Duration::from_millis(500), connect_future).map(move |result| (port, result));
        connection_futures.push(timout_future);
    }
    let results: Vec<(u16, Result<io::Result<_>, Elapsed>)> = join_all(connection_futures).await;
    let ports: Vec<PortReport> = results
        .iter()
        .map(|(port, result)| match result {
            Ok(Ok(_)) => PortReport {
                port: *port,
                status: PortStatus::Open,
            },
            _ => PortReport {
                port: *port,
                status: PortStatus::Closed,
            },
        })
        .collect();
    Report {
        target: target.into(),
        instance: Some(ip),
        contents: Ok(ReportContents {
            icmp: ping_result,
            ports: Some(ports),
        }),
    }
}
