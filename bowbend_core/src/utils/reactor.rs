use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    pin::Pin,
    task::Poll,
    time::{Duration, SystemTime},
};

use futures::{stream, Stream, StreamExt};
use linked_hash_map::LinkedHashMap;
use pin_project::pin_project;
use tokio::time::{interval, Interval};
use tracing::{debug, trace, warn};

use crate::utils::half_select::half_select;

/// This trait is used to represent any data about the context of the initial
/// request that will be needed to form a result.  It might be the time it was
/// started, parameters of the scan, etc.
pub(crate) trait Context: Send {
    type Reply: Reply;
    type Conclusion: Conclusion;

    fn start_time(&self) -> SystemTime;

    fn create_timeout_conclusion(&self) -> Self::Conclusion;

    fn create_conclusion(&self, _: Self::Reply) -> Self::Conclusion;
}

/// The only requirement we have for the reply type is that it is [Send].
pub(crate) trait Reply: Send {}

pub(crate) trait Conclusion: Send {}

/// The is the unique identifier that is used to link a response back to the
/// request.  For example for something that is unique to the host, like ICMP,
/// it could be the IP address.  For something tied to a port it would need to
/// be the IP address and port.
pub(crate) trait Index: Clone + Debug + Eq + Hash + PartialEq {}

/// The internal stream is made up of either context about a new request that
/// has been start, or it has results from a response.  This enum helps us
/// better represent this internally
enum Item<I, C, R> {
    Context((I, C)),
    Reply((I, R)),
}

/// Sometimes we need to match incoming packets with the original request to
/// make any use of them and we can't rely on any underlying OS functionality.
/// One example where we need this is when sending ICMP pings out.  Another is
/// when are working with raw sockets like when doing SYN scans.
///
/// This helper utility allows us to easily do this.  It will take a stream of
/// new request contexts and a stream of responses.  When a request context
/// comes in it will held on to waiting for a response.  When a response comes
/// in we will attempt to match it with a previous seen request. If no response
/// for a request comes in by the time it hits a timeout, we will call the
/// [Context::create_timeout_conclusion] to create a timeout result to emmit on
/// the stream.
///
/// Timeouts are not guaranteed to be be emitted exactly when the event would
/// have expired. Internally this stream maintains a timer that is used to
/// trigger GC passes over all outstanding requests when it hasn't been waken
/// for any other reason.  The setting of this timer is a trade off between
/// resolution and accuracy.  A longer setting means we have less wasteful
/// waking, but possibly longer to wait when the timeout happened and when the
/// event is emitted.
///
/// This supports response/requests coming in out of order but currently logs a
/// warning when a response arrives before its corresponding request.
#[allow(unused)]
pub(crate) fn reactor<
    I: Index,
    C: Context,
    S1: Stream<Item = (I, C)> + 'static + Send,
    S2: Stream<Item = (I, C::Reply)> + 'static + Send,
>(
    context_stream: S1,
    reply_stream: S2,
    timeout: Duration,
) -> impl Stream<Item = (C, C::Conclusion)> {
    let input = half_select(
        context_stream.map(|(i, c)| Item::Context((i, c))),
        reply_stream.map(|(i, r)| Item::Reply((i, r))),
    );
    let reactor = InnerReactor::new(input, timeout);
    reactor.map(stream::iter).flatten()
}

/// The resulting stream of responses or timeouts.  Returned by [reactor]
#[pin_project]
struct InnerReactor<I: Index, C: Context, S: Stream<Item = Item<I, C, C::Reply>>> {
    waiting_for_match: LinkedHashMap<I, C>,
    out_of_order_results: HashMap<I, C::Reply>,
    #[pin]
    input: S,
    timeout: Duration,
    #[pin]
    gc_interval: Interval,
    final_pass: bool,
}

impl<I: Index, C: Context, S: Stream<Item = Item<I, C, C::Reply>>> InnerReactor<I, C, S> {
    pub fn new(input: S, timeout: Duration) -> Self {
        let gc_interval = interval(timeout / 5); //TODO: don't hardcore

        Self {
            waiting_for_match: Default::default(),
            out_of_order_results: Default::default(),
            input,
            timeout,
            gc_interval,
            final_pass: false,
        }
    }
}

impl<I: Index, C: Context, S: Stream<Item = Item<I, C, C::Reply>>> Stream
    for InnerReactor<I, C, S>
{
    type Item = Vec<(C, C::Conclusion)>;

    #[tracing::instrument(level = "trace", skip(self, cx))]
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut projection = self.as_mut().project();
        let mut item = None;
        while let Poll::Ready(_) = projection.gc_interval.poll_tick(cx) {  }
        if !(*projection.final_pass) {
            loop {
                match projection.input.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Item::Context((index, context)))) => {
                        // We have an item from the context stream.  Let's reset the GC timer, try
                        // and match it up, then do a GC pass
                        projection.gc_interval.reset();
                        if let Some(reply) = projection.out_of_order_results.remove(&index) {
                            warn!("We found a match for an out of order response!  The system will continue to work but this is a strange situation");
                            let conclusion = context.create_conclusion(reply);
                            item = Some((context, conclusion));
                            break;
                        } else {
                            trace!("Got a context");
                            projection.waiting_for_match.insert(index, context);
                        }
                    }
                    Poll::Ready(Some(Item::Reply((index, reply)))) => {
                        // We have an item from the result stream.  Let's reset the GC timer, try to
                        // match it up then do a GC pass
                        projection.gc_interval.reset();
                        if let Some(context) = projection.waiting_for_match.remove(&index) {
                            trace!("Found a waiting match");
                            let conclusion = context.create_conclusion(reply);
                            item = Some((context, conclusion));
                            break;
                        } else {
                            warn!("We have a result without a context.  Holding onto this but this is unexpected behavior");
                            projection.out_of_order_results.insert(index, reply);
                        }
                    }
                    Poll::Ready(None) => {
                        // Both our streams have finished.  This is the only case where we should
                        // skip reseting the timer.  Let it expire one more time and then we are
                        // finished.
                        debug!("Both streams have finished");
                        *projection.final_pass = true;
                        break;
                    }
                    Poll::Pending => {
                        trace!("Pending");
                        // The streams are pending. This could have been because of the timer or
                        // even an event outside this module.  Either way,
                        // reset the timer and do a GC pass.
                        projection.gc_interval.reset();
                        break;
                    }
                }
            }
        }

        let mut to_ret = item.map(|x| vec![x]).unwrap_or_default();

        // Do a GC pass.
        // Any items started before this Instant have timed out.  Remove them from our
        // "waiting" list and return expired entries
        let trim_instant = SystemTime::now() - *projection.timeout;
        for entry in projection.waiting_for_match.entries() {
            trace!(
                "checking start time {:?} against trim time {:?}",
                entry.get().start_time(),
                trim_instant
            );
            if trim_instant >= entry.get().start_time() {
                let context = entry.remove();
                let conclusion = context.create_timeout_conclusion();
                to_ret.push((context, conclusion));
            }
        }

        if *projection.final_pass {
            let mut timeouts: Vec<_> = projection
                .waiting_for_match
                .drain()
                .map(|x| {
                    let conclusion = x.1.create_timeout_conclusion();
                    (x.1, conclusion)
                })
                .collect();
            to_ret.append(&mut timeouts);
            if to_ret.is_empty() {
                Poll::Ready(None)
            } else {
                // We are at the end of it all.  Poll one more time so we can get a None
                cx.waker().wake_by_ref();
                Poll::Ready(Some(to_ret))
            }
        } else if to_ret.is_empty() {
                Poll::Pending
            } else {
                Poll::Ready(Some(to_ret))
            }

    }
}
#[cfg(test)]
mod test {
    use std::time::Instant;
    use tokio_test::stream_mock::StreamMockBuilder;

    use super::*;

    #[derive(Debug)]
    struct TestContext {
        ctx: String,
        started: SystemTime,
    }

    impl TestContext {
        fn new(s: &str) -> Self {
            TestContext {
                ctx: s.to_string(),
                started: SystemTime::now(),
            }
        }
    }

    impl Context for TestContext {
        type Reply = String;
        type Conclusion = String;

        fn start_time(&self) -> SystemTime {
            self.started
        }

        fn create_timeout_conclusion(&self) -> Self::Reply {
            format!("{} timeout", self.ctx)
        }

        fn create_conclusion(&self, reply: Self::Reply) -> Self::Conclusion {
            format!("{} conclusion", reply)
        }
    }

    impl Index for &str {}

    impl Reply for String {}

    impl Conclusion for String {}

    #[tokio::test]
    async fn basic_reactor_test() {
        let context_stream_mock = StreamMockBuilder::new()
            .next(("target1", TestContext::new("context1")))
            .next(("target2", TestContext::new("context2")))
            .next(("target3", TestContext::new("context3")))
            .build();

        let result_stream_mock = StreamMockBuilder::new()
            .next(("target1", "result1".to_string()))
            .next(("target2", "result2".to_string()))
            .next(("target3", "result3".to_string()))
            .build();
        let results: Vec<(TestContext, String)> = reactor(
            context_stream_mock,
            result_stream_mock,
            Duration::from_millis(500),
        )
        .collect()
        .await;
        assert_eq!(results.get(0).unwrap().1, "result1 conclusion");
        assert_eq!(results.get(1).unwrap().1, "result2 conclusion");
        assert_eq!(results.get(2).unwrap().1, "result3 conclusion");
    }

    #[tokio::test]
    async fn basic_timeout() {
        let start = Instant::now();
        let context_stream_mock = StreamMockBuilder::new()
            .next(("target1", TestContext::new("context1")))
            .next(("target2", TestContext::new("context2")))
            .build();

        let result_stream_mock = StreamMockBuilder::new()
            .next(("target1", "result1".to_string()))
            .wait(Duration::from_secs(10))
            .build();
        let results: Vec<(TestContext, String)> = reactor(
            context_stream_mock,
            result_stream_mock,
            Duration::from_millis(500),
        )
        .collect()
        .await;
        assert_eq!(results.get(0).unwrap().1, "result1 conclusion");
        assert_eq!(results.get(1).unwrap().1, "context2 timeout");
        // Assert that we didn't wait for the 10 second pause from the result stream
        assert!((Instant::now() - start) < Duration::from_secs(5));
    }
}
