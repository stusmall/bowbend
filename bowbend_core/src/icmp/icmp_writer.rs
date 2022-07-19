use std::{io, net::SocketAddr, os::unix::io::AsRawFd, time::SystemTime};

use socket2::{SockAddr, Socket};
use tokio::io::unix::AsyncFd;
use tracing::{info, instrument};

use crate::icmp::packet::{EchoRequest, IcmpV4, IcmpV6};

pub(crate) struct PingSentSummary {
    pub icmp_identity: u16,
    pub time_sent: SystemTime,
}

#[instrument(level = "trace")]
pub(crate) async fn send_ping(
    socket: &Socket,
    destination: SockAddr,
    icmp_identity: u16,
    sequence_count: u16,
) -> Result<PingSentSummary, io::Error> {
    let async_fd = AsyncFd::new(socket.as_raw_fd())?;
    let mut buffer = [0; 12];
    let payload = vec![1, 2, 3, 4];
    let request = EchoRequest {
        ident: icmp_identity,
        seq_cnt: sequence_count,
        payload: &payload,
    };

    //This unwrap is safe because we know this will always either be AF_INET or
    // AF_INET6
    match destination.as_socket().unwrap() {
        SocketAddr::V4(_) => request.encode::<IcmpV4>(&mut buffer)?,
        SocketAddr::V6(_) => request.encode::<IcmpV6>(&mut buffer)?,
    }
    let time_sent = SystemTime::now();
    internal_write(async_fd, socket, destination, &buffer).await?;
    info!("Ping successfully sent");
    Ok(PingSentSummary {
        icmp_identity,
        time_sent,
    })
}

#[instrument(level = "trace")]
async fn internal_write(
    async_fd: AsyncFd<i32>,
    socket: &Socket,
    destination: SockAddr,
    buffer: &[u8],
) -> io::Result<usize> {
    loop {
        let mut write_guard = async_fd.writable().await?;
        match write_guard.try_io(|_| socket.send_to(buffer, &destination)) {
            Ok(result) => return result,
            Err(_would_block) => continue,
        }
    }
}
