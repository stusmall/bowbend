use std::{ops::Range, sync::Arc};

use futures::{stream, Stream};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::sync::Semaphore;

use crate::{icmp::PingResult, report::Report, target::TargetInstance};

pub(crate) async fn syn_scan(
    mut input_stream: impl Stream<Item = (TargetInstance, Option<PingResult>)> + Unpin,
    port_list: Vec<u16>,
    semaphore: Arc<Semaphore>,
    throttle_range: Option<Range<u64>>,
) -> impl Stream<Item = Report> {
    // let icmpv4_sender = Socket::new_raw(Domain::IPV4, Type::RAW,
    // Some(Protocol::ICMPV4))     .map_err(socket_open_error)?;

    stream::iter(vec![])
}
