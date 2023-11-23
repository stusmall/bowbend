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
use tokio::{
    sync::{
        mpsc::{channel, Receiver},
        Semaphore,
    },
    task,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

use crate::{
    icmp::{
        icmp_listener::{listen_for_icmp, ReceivedIcmpPacket},
        icmp_writer::{send_ping, PingSentSummary, PingWriteError},
    },
    target::TargetInstance,
    utils::{reactor, reactor::reactor},
    PortscanErr,
};

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

impl reactor::Conclusion for PingResult {}

impl reactor::Reply for ReceivedIcmpPacket {}

impl reactor::Index for u16 {}

/// Details on an ICMP reply we received.  Right now we are just holding onto
/// the time received.
#[derive(Debug)]
pub struct IcmpSummary {
    /// The time the reply was received.
    pub time_received: SystemTime,
}

impl reactor::Context for (TargetInstance, PingSentSummary) {
    type Reply = ReceivedIcmpPacket;
    type Conclusion = PingResult;

    fn start_time(&self) -> SystemTime {
        self.1.time_sent
    }

    fn create_timeout_conclusion(&self) -> Self::Conclusion {
        PingResult {
            ping_sent: self.1.time_sent,
            result_type: PingResultType::Timeout,
        }
    }

    fn create_conclusion(&self, reply: Self::Reply) -> Self::Conclusion {
        PingResult {
            ping_sent: self.1.time_sent,
            result_type: PingResultType::Reply(IcmpSummary {
                time_received: reply.time_received,
            }),
        }
    }
}

#[tracing::instrument(skip(target_stream))]
pub(crate) async fn icmp_sweep(
    target_stream: impl Stream<Item = TargetInstance> + 'static + Send + Unpin,
    semaphore: Arc<Semaphore>,
) -> Result<impl Stream<Item = (TargetInstance, Option<PingResult>)>, PortscanErr> {
    let recieved_packet_rx = start_icmp_listener_task().await?;
    let (ping_sent_rx, ping_sending_error_rx) =
        start_ping_sender_task(target_stream, semaphore).await?;

    let context_stream = ReceiverStream::new(ping_sent_rx).map(|x| (x.1.icmp_identity, x));

    let result_stream = ReceiverStream::new(recieved_packet_rx).map(|x| (x.identity, x));
    let reactor_stream = reactor(context_stream, result_stream, Duration::from_secs(10));

    let _ = ReceiverStream::new(ping_sending_error_rx).map(|_x| {
       unimplemented!();
    });

    Ok(reactor_stream.map(|x| (x.0 .0, Some(x.1))))
}

async fn start_ping_sender_task(
    mut target_stream: impl Stream<Item = TargetInstance> + 'static + Send + Unpin,
    semaphore: Arc<Semaphore>,
) -> Result<
    (
        Receiver<(TargetInstance, PingSentSummary)>,
        Receiver<PingWriteError>,
    ),
    PortscanErr,
> {
    let icmpv4_sender = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .map_err(socket_open_error)?;
    let icmpv6_sender = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))
        .map_err(socket_open_error)?;
    let sent_ping_channel =
        channel::<(TargetInstance, PingSentSummary)>(crate::consts::CHANNEL_SIZE);
    let error_channel = channel::<PingWriteError>(crate::consts::CHANNEL_SIZE);
    let _write_ping_task = task::spawn(async move {
        while let Some(target) = target_stream.next().await {
            let dest = SocketAddr::new(target.get_ip(), 0).into();
            let sender = match target.get_ip() {
                IpAddr::V4(_) => &icmpv4_sender,
                IpAddr::V6(_) => &icmpv6_sender,
            };
            match send_ping(
                target.clone(),
                sender,
                dest,
                random(),
                random(),
                semaphore.clone(),
            )
            .await
            {
                Ok(x) => sent_ping_channel.0.send((target, x)).await.unwrap(),
                Err(e) => error_channel.0.send(e).await.unwrap(),
            };
        }
    });

    Ok((sent_ping_channel.1, error_channel.1))
}

async fn start_icmp_listener_task() -> Result<Receiver<ReceivedIcmpPacket>, PortscanErr> {
    // TODO: This is pretty wonky.  We should move the actual listening into their
    // own tasks.  Instead it has some of my old stream heavy code and puts a
    // facade of a task at the end so it fits in. this will cause unneeded wakes

    let icmpv4_listener_socket = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .map_err(socket_open_error)?;
    let icmpv6_listener_socket = Socket::new_raw(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))
        .map_err(socket_open_error)?;
    let icmpv4_listener = listen_for_icmp(icmpv4_listener_socket).boxed();
    let icmpv6_listener = listen_for_icmp(icmpv6_listener_socket).boxed();
    let mut merged_stream = combine(icmpv4_listener, icmpv6_listener);
    let (tx, rx) = channel::<ReceivedIcmpPacket>(crate::consts::CHANNEL_SIZE);

    let _icmp_listener_task = task::spawn(async move {
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

#[instrument(skip(target_stream))]
pub(crate) async fn skip_icmp(
    target_stream: impl Stream<Item = TargetInstance> + 'static + Send,
) -> Result<impl Stream<Item = (TargetInstance, Option<PingResult>)>, PortscanErr> {
    Ok(target_stream.map(|target| (target, None)))
}
