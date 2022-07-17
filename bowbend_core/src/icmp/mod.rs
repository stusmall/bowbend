use std::{
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
use std::time::SystemTime;

use async_stream::stream;
use futures::{select, stream::select as combine, FutureExt, Stream, StreamExt};
use rand::random;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::time::sleep;
use tracing::{debug, error, instrument};

use crate::{
    icmp::{
        icmp_listener::{listen_for_icmp, ReceivedIcmpPacket},
        icmp_writer::{send_ping, PingSentSummary},
    },
    target::TargetInstance,
    utils::batch_stream::batch_stream,
    PortscanErr,
};

pub(crate) mod icmp_listener;
pub mod icmp_writer;
mod packet;

/// The results of an send ICMP hello if sent.  A valid
#[derive(Debug)]
pub struct PingResult {
    pub destination: TargetInstance,
    pub ping_sent: Option<SystemTime>,
    pub result_type: PingResultType,
}

/// The result from our ICMP stage.
#[derive(Debug)]
pub enum PingResultType {
    /// We sent an ICMP hello but timeout waiting for a reply
    Timeout,
    /// We received a reply
    Reply(IcmpSummary),
    /// We've deiced to skip ping.
    Skipped,
}

#[derive(Debug)]
pub struct IcmpSummary {
    pub time_received: Instant,
}

#[tracing::instrument(skip(target_stream))]
pub(crate) async fn icmp_sweep(
    target_stream: impl Stream<Item = TargetInstance> + 'static + Send,
) -> Result<impl Stream<Item = PingResult>, PortscanErr> {
    #[instrument(level = "error")]
    fn socket_open_error(_: io::Error) -> PortscanErr {
        PortscanErr::InsufficientPermission
    }

    let icmpv4_sender = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .map_err(socket_open_error)?;
    let icmpv6_sender = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))
        .map_err(socket_open_error)?;
    let icmpv4_listener_socket = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .map_err(socket_open_error)?;
    let icmpv6_listener_socket = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))
        .map_err(socket_open_error)?;
    let icmpv4_listener = listen_for_icmp(icmpv4_listener_socket).boxed();
    let icmpv6_listener = listen_for_icmp(icmpv6_listener_socket).boxed();
    let mut sent_pings = HashMap::new();

    let mut target_stream = Box::pin(batch_stream(100, target_stream));

    while let Some(targets) = target_stream.next().await {
        for target in targets {
            let dest = SocketAddr::new(target.get_ip(), 0).into();
            match target.get_ip() {
                IpAddr::V4(_) => {
                    let future = send_ping(&icmpv4_sender, dest, random(), random());
                    sent_pings.insert(target, future);
                }
                IpAddr::V6(_) => {
                    let future = send_ping(&icmpv6_sender, dest, random(), random());
                    sent_pings.insert(target, future);
                }
            }
        }
    }

    let mut targets = HashMap::new();
    for (target, output) in sent_pings {
        let identity = output.await.unwrap(); //TODO: We should include the target with the error
        targets.insert(target.get_ip(), (target.to_owned(), identity));
    }
    let merged_stream = combine(icmpv4_listener, icmpv6_listener);
    Ok(await_results(targets, merged_stream))
}

#[instrument(skip(targets, icmp_listener))]
fn await_results(
    mut targets: HashMap<IpAddr, (TargetInstance, PingSentSummary)>,
    mut icmp_listener: impl Stream<Item = io::Result<ReceivedIcmpPacket>> + Unpin,
) -> impl Stream<Item = PingResult> {
    stream! {
        let mut ping_timeout = sleep(Duration::from_millis(500)).boxed().fuse();
        let mut icmp_stream_future = icmp_listener.next().fuse();
        loop {
            select!{
                () = ping_timeout => {
                    // We've hit our timeout.  Anything left in the target list will get
                    for entry in targets {
                        yield PingResult {
                            destination: entry.1.0,
                            ping_sent: Some(entry.1.1.time_sent),
                            result_type: PingResultType::Timeout
                        };
                    }
                    break;
                }
                result = icmp_stream_future => {
                    icmp_stream_future = icmp_listener.next().fuse();
                    match result {
                        Some(Ok(packet)) => {
                            if let Some(entry) = targets.remove(&packet.source){
                                //TODO:  We shouldn't remove this if the IDs don't match
                                if packet.identity == entry.1.icmp_identity {
                                    debug!("We got a match!  Yielding");
                                    // This looks to match!  Yield it for the next step in the scan
                                    yield PingResult{
                                        destination: entry.0,
                                        ping_sent: Some(entry.1.time_sent),
                                        result_type: PingResultType::Reply(IcmpSummary{
                                            time_received: packet.time_received
                                        })
                                    };
                                } else {
                                    debug!("We got a message for {:?} which is a target but the identity doesn't match. {} {}", packet.source, packet.identity, entry.1.icmp_identity);
                                }
                            } else {
                                debug!("We got an ICMP message for {:?} which isn't one of our targets.  Dropping it", packet.source);
                            }
                        }
                        Some(Err(e)) => {
                            error!("Found an error when reading icmp message {:?}", e);
                        }
                        None => {
                            debug!("The stream is done");
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[instrument(skip(target_stream))]
pub(crate) async fn skip_icmp(
    target_stream: impl Stream<Item = TargetInstance> + 'static + Send,
) -> Result<impl Stream<Item = PingResult>, PortscanErr> {
    Ok(target_stream.map(|target| PingResult {
        destination: target,
        ping_sent: None,
        result_type: PingResultType::Skipped,
    }))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io, net::IpAddr, time::Instant};
    use std::time::SystemTime;

    use futures::{stream, StreamExt};

    use crate::{
        icmp::{
            await_results, icmp_listener::ReceivedIcmpPacket, icmp_writer::PingSentSummary,
            PingResult,
        },
        target::TargetInstance,
    };

    fn build_received(
        dest_least_significant_byte: u8,
        identity: u16,
    ) -> io::Result<ReceivedIcmpPacket> {
        Ok(ReceivedIcmpPacket {
            source: IpAddr::from([0, 0, 0, dest_least_significant_byte]),
            identity,
            time_received: Instant::now(),
        })
    }

    fn build_targets(
        dest_least_significant_byte: u8,
        icmp_identity: u16,
    ) -> (IpAddr, (TargetInstance, PingSentSummary)) {
        let ip = IpAddr::from([0, 0, 0, dest_least_significant_byte]);
        (
            ip,
            (
                TargetInstance::IP(ip),
                PingSentSummary {
                    icmp_identity,
                    time_sent: SystemTime::now(),
                },
            ),
        )
    }

    #[tokio::test]
    async fn test_basic_awaiting_results() {
        let mut target = HashMap::new();
        for number in 0..10u16 {
            let x = build_targets(number as u8, number);
            target.insert(x.0, x.1);
        }
        let received_pings: Vec<io::Result<ReceivedIcmpPacket>> = (0..10u16)
            .map(|number| build_received(number as u8, number))
            .collect();

        let ping_results: Vec<PingResult> = await_results(target, stream::iter(received_pings))
            .collect()
            .await;
        assert_eq!(ping_results.len(), 10);
    }
}
