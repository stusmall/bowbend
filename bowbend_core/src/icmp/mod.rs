//! This manages everything needed for the initial ICMP sweep to see if hosts
//! are up.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime},
};

use futures::{stream::select as combine, Stream, StreamExt};
use rand::random;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{sync::Semaphore, task};
use tokio::sync::mpsc::{channel, Receiver};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

use crate::{icmp::{
    icmp_listener::{listen_for_icmp, ReceivedIcmpPacket},
    icmp_writer::{send_ping, PingSentSummary},
}, stream::iter, target::TargetInstance, PortscanErr, utils};
use crate::icmp::icmp_writer::PingWriteError;
use crate::utils::reactor::reactor;

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

impl utils::reactor::Result for PingResult {

}

/// Details on an ICMP reply we received.  Right now we are just holding onto
/// the time received.
#[derive(Debug)]
pub struct IcmpSummary {
    /// The time the reply was received.
    pub time_received: SystemTime,
}

impl crate::utils::reactor::Context for PingSentSummary {
    type Result = PingResult;

    fn start_time(&self) -> SystemTime {
        self.time_sent
    }

    fn create_timeout_result(&self) -> Self::Result {
        PingResult {
            ping_sent: self.time_sent,
            result_type: PingResultType::Timeout,
        }
    }
}

#[tracing::instrument(skip(target_stream))]
pub(crate) async fn icmp_sweep(
    mut target_stream: impl Stream<Item = TargetInstance> + 'static + Send + Unpin,
    semaphore: Arc<Semaphore>,
) -> Result<impl Stream<Item = (TargetInstance, Option<PingResult>)>, PortscanErr> {

    let recieved_packet_rx = start_icmp_listener_task().await?;
    let (ping_sent_rx, ping_sending_error_rx) = start_ping_sender_task(target_stream, semaphore).await?;

    let context_stream = ReceiverStream::new(ping_sent_rx)
        .map(|x| {
            (x.icmp_identity, x)
        });

    let result_stream =  ReceiverStream::new(recieved_packet_rx)
        .map(|x|{
            (x.identity, x)
        });
    // let r = reactor(context_stream, result_stream, Duration::from_secs(10));
    Ok(iter(vec![]))
}


async fn start_ping_sender_task(mut target_stream: impl Stream<Item = TargetInstance> + 'static + Send + Unpin,
                                semaphore: Arc<Semaphore>) -> Result<(Receiver<PingSentSummary>, Receiver<PingWriteError>), PortscanErr>{
    let icmpv4_sender = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .map_err(socket_open_error)?;
    let icmpv6_sender = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))
        .map_err(socket_open_error)?;
    let sent_ping_channel = channel::<PingSentSummary>(crate::consts::CHANNEL_SIZE);
    let error_channel = channel::<PingWriteError>(crate::consts::CHANNEL_SIZE);
    let _write_ping_task = tokio::task::spawn( async move {
        while let Some(target) = target_stream.next().await {
            let dest = SocketAddr::new(target.get_ip(), 0).into();
            let sender = match target.get_ip() {
                IpAddr::V4(_) => {
                    &icmpv4_sender
                }
                IpAddr::V6(_) => {
                    &icmpv6_sender
                }
            };
            match send_ping(
                target.clone(),
                sender,
                dest,
                random(),
                random(),
                semaphore.clone(),
            ).await {
                Ok(x) => sent_ping_channel.0.send(x).await.unwrap(),
                Err(e) => error_channel.0.send(e).await.unwrap(),
            };
        }
    });

    Ok((sent_ping_channel.1, error_channel.1))
}

async fn start_icmp_listener_task() -> Result<Receiver<ReceivedIcmpPacket>, PortscanErr>{

    // TODO: This is pretty wonky.  We should move the actual listening into their own tasks.  Instead
    // it has some of my old stream heavy code and puts a facade of a task at the end so it fits in.
    // this will cause unneeded wakes

    let icmpv4_listener_socket = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .map_err(socket_open_error)?;
    let icmpv6_listener_socket = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))
        .map_err(socket_open_error)?;
    let icmpv4_listener = listen_for_icmp(icmpv4_listener_socket).boxed();
    let icmpv6_listener = listen_for_icmp(icmpv6_listener_socket).boxed();
    let mut merged_stream = combine(icmpv4_listener, icmpv6_listener);
    let (tx, rx) = channel::<ReceivedIcmpPacket>(crate::consts::CHANNEL_SIZE);
    task::spawn(async move {
        while let Some(Ok(packet)) = merged_stream.next().await {
            tx.send(packet).await.unwrap();
        }
    });
    Ok(rx)
}

#[instrument(level = "error")]
fn socket_open_error(_: io::Error) -> PortscanErr {
    PortscanErr::InsufficientPermission
}

/*
#[instrument(skip(targets, icmp_listener))]
fn await_results(
    mut targets: HashMap<IpAddr, (TargetInstance, PingSentSummary)>,
    mut icmp_listener: impl Stream<Item = io::Result<ReceivedIcmpPacket>> + Unpin,
) -> impl Stream<Item = (TargetInstance, Option<PingResult>)> {
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
}*/

#[instrument(skip(target_stream))]
pub(crate) async fn skip_icmp(
    target_stream: impl Stream<Item = TargetInstance> + 'static + Send,
) -> Result<impl Stream<Item = (TargetInstance, Option<PingResult>)>, PortscanErr> {
    Ok(target_stream.map(|target| (target, None)))
}

#[cfg(test)]
mod tests {
    use std::{io, net::IpAddr, time::SystemTime};


    use crate::{
        icmp::{icmp_listener::ReceivedIcmpPacket, icmp_writer::PingSentSummary},
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

    // #[tokio::test]
    // async fn test_basic_awaiting_results() {
    //     let mut target = HashMap::new();
    //     for number in 0..10u16 {
    //         let x = build_targets(number as u8, number);
    //         target.insert(x.0, x.1);
    //     }
    //     let received_pings: Vec<io::Result<ReceivedIcmpPacket>> = (0..10u16)
    //         .map(|number| build_received(number as u8, number))
    //         .collect();
    //
    //     let ping_results: Vec<_> = crate::icmp::await_results(target, stream::iter(received_pings))
    //         .collect()
    //         .await;
    //     assert_eq!(ping_results.len(), 10);
    // }
}
