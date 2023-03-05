//! This manages everything needed for the initial ICMP sweep to see if hosts
//! are up.

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_stream::stream;
use futures::{select, stream::select as combine, FutureExt, Stream, StreamExt};
use rand::random;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{sync::Semaphore, time::sleep};
use tracing::{debug, error, instrument};

use crate::{icmp::{
    icmp_listener::{listen_for_icmp, ReceivedIcmpPacket},
    icmp_writer::{send_ping, PingSentSummary},
}, stream::iter, target::TargetInstance, PortscanErr, Target};
use crate::utils::reactor::Reactor;

pub(crate) mod icmp_listener;
pub(crate) mod icmp_writer;
mod packet;

/// The results of an send ICMP hello if sent.
#[derive(Debug)]
pub struct PingResult {
    /// The time when either the ping was sent or attempted to be sent
    pub ping_sent: SystemTime,
    /// The result of the ping.  Either we got a reply, we timed out or hit some
    /// type of IO failure
    pub result_type: PingResultType,
}

/// The result from our ICMP stage.
#[derive(Debug)]
pub enum PingResultType {
    /// We hit an IO error when trying to send the ping
    Error(io::Error),
    /// We sent an ICMP hello but timeout waiting for a reply
    Timeout,
    /// We received a reply
    Reply(IcmpSummary),
}

/// Details on an ICMP reply we received.  Right now we are just holding onto
/// the time received.
#[derive(Debug)]
pub struct IcmpSummary {
    /// The time the reply was received.
    pub time_received: SystemTime,
}

#[tracing::instrument(skip(target_stream))]
pub(crate) async fn icmp_sweep(
    mut target_stream: impl Stream<Item = TargetInstance> + 'static + Send + Unpin,
    semaphore: Arc<Semaphore>,
) -> Result<impl Stream<Item = (TargetInstance, Option<PingResult>)>, PortscanErr> {
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

    while let Some(target) = target_stream.next().await {
        let dest = SocketAddr::new(target.get_ip(), 0).into();
        match target.get_ip() {
            IpAddr::V4(_) => {
                let future = send_ping(
                    target.clone(),
                    &icmpv4_sender,
                    dest,
                    random(),
                    random(),
                    semaphore.clone(),
                );
                sent_pings.insert(target, future);
            }
            IpAddr::V6(_) => {
                let future = send_ping(
                    target.clone(),
                    &icmpv6_sender,
                    dest,
                    random(),
                    random(),
                    semaphore.clone(),
                );
                sent_pings.insert(target, future);
            }
        }
    }

    let mut targets = HashMap::new();
    let mut errors = Vec::new();
    for (target, output) in sent_pings {
        match output.await {
            Ok(identity) => {
                targets.insert(target.get_ip(), (target.to_owned(), identity));
            }
            Err(e) => {
                errors.push((
                    e.target_instance,
                    Some(PingResult {
                        ping_sent: SystemTime::now(),
                        result_type: PingResultType::Error(e.error),
                    }),
                ));
            }
        };
    }
    let merged_stream = combine(icmpv4_listener, icmpv6_listener);
    Ok(combine(iter(errors), await_results(targets, merged_stream)))
}
// Index is IpAddr
// Context is TargetInstance
// Result is Option<PingResult>

#[instrument(skip(targets, icmp_listener))]
fn await_results(
    mut targets: HashMap<IpAddr, (TargetInstance, PingSentSummary)>,
    mut icmp_listener: impl Stream<Item = io::Result<ReceivedIcmpPacket>> + Unpin,
) -> impl Stream<Item = (TargetInstance, Option<PingResult>)> {

    //Reactor::<IpAddr, TargetInstance, Option<PingResult>>::new();
    stream! {
        let mut ping_timeout = sleep(Duration::from_millis(500)).boxed().fuse();
        let mut icmp_stream_future = icmp_listener.next().fuse();
        loop {
            select!{
                () = ping_timeout => {
                    // We've hit our timeout.  Anything left in the target list will get
                    for entry in targets {
                        yield (entry.1.0, Some(PingResult {
                            ping_sent: entry.1.1.time_sent,
                            result_type: PingResultType::Timeout
                        }));
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
                                    yield (entry.0, Some(PingResult{
                                        ping_sent: entry.1.time_sent,
                                        result_type: PingResultType::Reply(IcmpSummary{
                                            time_received: packet.time_received
                                        })
                                    }));
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
) -> Result<impl Stream<Item = (TargetInstance, Option<PingResult>)>, PortscanErr> {
    Ok(target_stream.map(|target| (target, None)))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io, net::IpAddr, time::SystemTime};

    use futures::{stream, StreamExt};

    use crate::{
        icmp::{await_results, icmp_listener::ReceivedIcmpPacket, icmp_writer::PingSentSummary},
        target::TargetInstance,
    };

    fn build_received(
        dest_least_significant_byte: u8,
        identity: u16,
    ) -> io::Result<ReceivedIcmpPacket> {
        Ok(ReceivedIcmpPacket {
            source: IpAddr::from([0, 0, 0, dest_least_significant_byte]),
            identity,
            time_received: SystemTime::now(),
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

        let ping_results: Vec<_> = await_results(target, stream::iter(received_pings))
            .collect()
            .await;
        assert_eq!(ping_results.len(), 10);
    }
}
