use std::{
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use async_stream::stream;
use futures::{select, stream::select as combine, FutureExt, Stream, StreamExt};
use rand::random;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::time::sleep;

use crate::{
    icmp::{
        icmp_listener::{listen_for_icmp, ReceivedIcmpPacket},
        icmp_writer::{send_ping, PingSentSummary},
    },
    target::PortscanTargetInstance,
    utils::batch_stream::batch_stream,
};

pub(crate) mod icmp_listener;
pub mod icmp_writer;
mod packet;

/// The results of an send ICMP hello if sent.  A valid
#[derive(Debug)]
pub struct PingResult {
    pub destination: PortscanTargetInstance,
    pub ping_sent: Option<Instant>,
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

pub(crate) async fn icmp_sweep(
    target_stream: impl Stream<Item = PortscanTargetInstance> + 'static + Send,
) -> io::Result<impl Stream<Item = PingResult>> {
    let icmpv4_sender = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    let icmpv6_sender = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?;
    let icmpv4_listener_socket = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    let icmpv6_listener_socket = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?;
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
        let identity = output.await?; // We should include the target with the error
        targets.insert(target.get_ip(), (target.to_owned(), identity));
    }
    let merged_stream = combine(icmpv4_listener, icmpv6_listener);
    Ok(await_results(targets, merged_stream))
}

fn await_results(
    mut targets: HashMap<IpAddr, (PortscanTargetInstance, PingSentSummary)>,
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
                                    tracing::debug!("We got a match!  Yielding");
                                    // This looks to match!  Yield it for the next step in the scan
                                    yield PingResult{
                                        destination: entry.0,
                                        ping_sent: Some(entry.1.time_sent),
                                        result_type: PingResultType::Reply(IcmpSummary{
                                            time_received: packet.time_received
                                        })
                                    };
                                } else {
                                    tracing::debug!("We got a message for {:?} which is a target but the identity doesn't match. {} {}", packet.source, packet.identity, entry.1.icmp_identity);
                                }
                            } else {
                                tracing::debug!("We got an ICMP message for {:?} which isn't one of our targets.  Dropping it", packet.source);
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("Found an error when reading icmp message {:?}", e);
                        }
                        None => {
                            tracing::debug!("The stream is done");
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io, net::IpAddr, time::Instant};

    use futures::{stream, StreamExt};

    use crate::{
        icmp::{
            await_results, icmp_listener::ReceivedIcmpPacket, icmp_writer::PingSentSummary,
            PingResult,
        },
        target::PortscanTargetInstance,
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
    ) -> (IpAddr, (PortscanTargetInstance, PingSentSummary)) {
        let ip = IpAddr::from([0, 0, 0, dest_least_significant_byte]);
        (
            ip,
            (
                PortscanTargetInstance::IP(ip),
                PingSentSummary {
                    icmp_identity,
                    time_sent: Instant::now(),
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
