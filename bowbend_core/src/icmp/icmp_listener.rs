use std::{
    io,
    mem::MaybeUninit,
    net::{IpAddr, SocketAddr},
    os::unix::io::AsRawFd,
    time::Instant,
};

use async_stream::try_stream;
use byteorder::{BigEndian, ByteOrder};
use futures::Stream;
use pnet::packet::{icmp::IcmpPacket, ipv4::Ipv4Packet, ipv6::Ipv6Packet, Packet};
use socket2::{SockAddr, Socket};
use tokio::io::unix::AsyncFd;
use tracing::{info, instrument, warn};
//TODO: We should differentiate between replies and Destination Unreachable
// etc.  Right now this is does what we need.  The difference between a timeout
// and host unreachable is kind of semantics.  We still won't continue the scan
#[derive(Debug)]
pub(crate) struct ReceivedIcmpPacket {
    pub source: IpAddr,
    pub identity: u16,
    pub time_received: Instant,
}

#[instrument(level = "trace")]
pub(crate) fn listen_for_icmp(
    mut socket: Socket,
) -> impl Stream<Item = io::Result<ReceivedIcmpPacket>> {
    try_stream! {
        socket.set_nonblocking(true)?;
        // TODO:  I think I can make this a little smaller but not positive.  Going big just to be safe.  Also might want to move it off the stack
        let mut buffer = [0u8; 65535];
        let fd = socket.as_raw_fd();
        let async_fd = AsyncFd::new(fd)?;
        loop {
            let (bytes_read, source) = internal_read(&async_fd, &mut socket, cast_as_maybe(&mut buffer)).await?;
            if let Some(std_src) = source.as_socket(){
                info!("We got a ping with {} bytes from {:?}", bytes_read, std_src);
                if let Some(to_ret) = parse_packet(std_src, &buffer, bytes_read){
                    yield to_ret;
                }
            } else {
                warn!("We read in {} bytes but didn't have a source.", bytes_read);
            }
        }
    }
}

#[tracing::instrument(level = "trace", skip(buffer))]
async fn internal_read(
    async_fd: &AsyncFd<i32>,
    socket: &mut Socket,
    buffer: &mut [MaybeUninit<u8>],
) -> Result<(usize, SockAddr), io::Error> {
    loop {
        let mut read_guard = async_fd.readable().await?;
        match read_guard.try_io(|_| socket.recv_from(buffer)) {
            Ok(result) => return result,
            Err(_would_block) => continue,
        }
    }
}

#[instrument(level = "trace", skip(buffer))]
fn parse_packet(
    source: SocketAddr,
    buffer: &[u8],
    bytes_read: usize,
) -> Option<ReceivedIcmpPacket> {
    match &source {
        SocketAddr::V4(_) => {
            if let Some(ip_packet) = Ipv4Packet::new(&buffer[..bytes_read]) {
                parse_icmp(source, ip_packet.payload())
            } else {
                info!("Failed to parse IPv4 packet");
                None
            }
        }
        SocketAddr::V6(_) => {
            if let Some(ip_packet) = Ipv6Packet::new(&buffer[..bytes_read]) {
                parse_icmp(source, ip_packet.payload())
            } else {
                info!("Failed to parse IPv6 packet");
                None
            }
        }
    }
}
//TODO: replace pnet's implementation with my own.  It's already there just
// need to drop it in and test it
fn parse_icmp(source: SocketAddr, ip_payload: &[u8]) -> Option<ReceivedIcmpPacket> {
    let to_ret = IcmpPacket::new(ip_payload).map(|icmp_packet| {
        let identity = BigEndian::read_u16(icmp_packet.payload());
        ReceivedIcmpPacket {
            source: source.ip(),
            identity,
            time_received: Instant::now(),
        }
    });
    if to_ret.is_none() {
        info!("Failed to parse ICMP packet")
    }
    to_ret
}

fn cast_as_maybe(buf: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    // Here is documentation on why this extremely unsafe looking thing is actually
    // safe: https://docs.rs/socket2/0.4.1/socket2/struct.Socket.html#safety
    unsafe { &mut *(buf as *mut [u8] as *mut [MaybeUninit<u8>]) }
}
