use std::{io, net::SocketAddr, os::unix::io::AsRawFd, sync::Arc, time::SystemTime};

use socket2::{SockAddr, Socket};
use tokio::{io::unix::AsyncFd, sync::Semaphore};
use tracing::{info, instrument};

use crate::{
    icmp::packet::{EchoRequest, IcmpV4, IcmpV6},
    target::TargetInstance,
};

pub(crate) struct PingSentSummary {
    pub icmp_identity: u16,
    pub time_sent: SystemTime,
}

#[derive(Debug)]
pub struct PingWriteError {
    pub target_instance: TargetInstance,
    pub time_attempted: SystemTime,
    pub error: io::Error,
}

impl PingWriteError {
    fn new(target_instance: TargetInstance, error: io::Error) -> Self {
        Self {
            target_instance,
            time_attempted: SystemTime::now(),
            error,
        }
    }
}

#[instrument(level = "trace")]
pub(crate) async fn send_ping(
    target_instance: TargetInstance,
    socket: &Socket,
    destination: SockAddr,
    icmp_identity: u16,
    sequence_count: u16,
    semaphore: Arc<Semaphore>,
) -> Result<PingSentSummary, PingWriteError> {
    let _permit = semaphore.acquire_owned().await;
    let mut buffer = [0; 12];
    let payload = vec![1, 2, 3, 4];
    let request = EchoRequest {
        ident: icmp_identity,
        seq_cnt: sequence_count,
        payload: &payload,
    };
    let async_fd = AsyncFd::new(socket.as_raw_fd())
        .map_err(|e| PingWriteError::new(target_instance.clone(), e))?;
    //This unwrap is safe because we know this will always either be AF_INET or
    // AF_INET6
    match destination.as_socket().unwrap() {
        SocketAddr::V4(_) => request
            .encode::<IcmpV4>(&mut buffer)
            .map_err(|e| PingWriteError::new(target_instance.clone(), e))?,
        SocketAddr::V6(_) => request
            .encode::<IcmpV6>(&mut buffer)
            .map_err(|e| PingWriteError::new(target_instance.clone(), e))?,
    }
    let time_sent = SystemTime::now();
    match internal_write(async_fd, socket, destination, &buffer).await {
        Ok(_) => {
            info!("Ping successfully sent");
            Ok(PingSentSummary {
                icmp_identity,
                time_sent,
            })
        }
        Err(error) => Err(PingWriteError {
            target_instance,
            time_attempted: time_sent,
            error,
        }),
    }
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
