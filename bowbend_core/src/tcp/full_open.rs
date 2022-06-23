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
};

#[instrument(level = "trace", skip(input_stream))]
pub(crate) async fn full_open_port_scan(
    mut input_stream: impl Stream<Item = PingResult> + Unpin,
    port_list: Vec<u16>,
) -> impl Stream<Item = Report> {
    stream! {
        while let Some(ping_result) = input_stream.next().await {
            match ping_result.result_type {
                PingResultType::Reply(_) | PingResultType::Skipped => {
                    let result = scan_host(ping_result, port_list.clone()).await;
                    yield result;
                }
                PingResultType::Timeout => {
                    yield Report {
                        target: ping_result.destination.clone().into(),
                        instance: Some(ping_result.destination.get_ip()),
                        contents: Ok(ReportContents {
                            icmp: Some(ping_result),
                            ports: None,
                        }),
                    };
                }
            }
        }
    }
}

#[instrument(level = "trace")]
async fn scan_host(ping_result: PingResult, mut ports: Vec<u16>) -> Report {
    let mut connection_futures = vec![];
    let ip = ping_result.destination.get_ip();
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
        target: ping_result.destination.clone().into(),
        instance: Some(ping_result.destination.get_ip()),
        contents: Ok(ReportContents {
            icmp: Some(ping_result),
            ports: Some(ports),
        }),
    }
}
