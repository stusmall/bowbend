use std::{collections::HashMap, fmt::Debug, hash::Hash, pin::Pin, task::Poll, time::Duration};

use futures::{
    stream,
    stream::{select, BoxStream},
    Stream, StreamExt,
};
use linked_hash_map::LinkedHashMap;
use pin_project::pin_project;
use tokio::time::{interval, Instant, Interval};
use tracing::{debug, trace, warn};

use crate::logging::setup_tracing;

//TODO: remove debug
pub(crate) trait Context: Debug + Send {
    type Result: Result;

    fn start_time(&self) -> Instant;

    fn create_timeout_result(&self) -> Self::Result;
}

//TODO: remove debug
pub(crate) trait Result: Debug + Send {}

pub(crate) trait Index: Clone + Debug + Eq + Hash + PartialEq {}

enum Item<I, C, R> {
    Context((I, C)),
    Result((I, R)),
}
pub(crate) fn reactor<I: Index, C: Context>(
    context_stream: impl Stream<Item = (I, C)> + 'static + Send,
    result_stream: impl Stream<Item = (I, C::Result)> + 'static + Send,
    timeout: Duration,
) -> impl Stream<Item = (C, C::Result)> {
    let reactor = InnerReactor::new(context_stream, result_stream, timeout);
    reactor.map(stream::iter).flatten()
}
#[pin_project]
struct InnerReactor<I: Index, C: Context> {
    waiting_for_match: LinkedHashMap<I, C>,
    out_of_order_results: HashMap<I, C::Result>,
    // TODO: try and make this generic rather than boxed
    #[pin]
    input: BoxStream<'static, Item<I, C, C::Result>>,
    timeout: Duration,
    #[pin]
    gc_interval: Interval,
    final_pass: bool,
}

impl<I: Index, C: Context> InnerReactor<I, C> {
    pub fn new(
        context_stream: impl Stream<Item = (I, C)> + 'static + Send,
        results_stream: impl Stream<Item = (I, C::Result)> + 'static + Send,
        timeout: Duration,
    ) -> Self {
        let input = select(
            context_stream.map(|(i, c)| Item::Context((i, c))),
            results_stream.map(|(i, r)| Item::Result((i, r))),
        );

        let gc_interval = interval(timeout / 5);
        Self {
            waiting_for_match: Default::default(),
            out_of_order_results: Default::default(),
            input: input.boxed(),
            timeout,
            gc_interval,
            final_pass: false,
        }
    }
}

impl<I: Index, C: Context> Stream for InnerReactor<I, C> {
    type Item = Vec<(C, C::Result)>;

    #[tracing::instrument(level = "trace", ret, skip(self, cx))]
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut projection = self.as_mut().project();
        let mut item = None;
        loop {
            match projection.gc_interval.poll_tick(cx) {
                Poll::Ready(_) => {}
                Poll::Pending => break,
            }
        }
        if *projection.final_pass == false {
            loop {
                match projection.input.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Item::Context((index, context)))) => {
                        // We have an item from the context stream.  Let's reset the GC timer, try
                        // and match it up, then do a GC pass
                        projection.gc_interval.reset();
                        if let Some(result) = projection.out_of_order_results.remove(&index) {
                            warn!("We found a match for an out of order response!  The system will continue to work but this is a strange situation");
                            item = Some((context, result));
                            break;
                        } else {
                            trace!("Got a context");
                            projection.waiting_for_match.insert(index, context);
                        }
                    }
                    Poll::Ready(Some(Item::Result((index, result)))) => {
                        // We have an item from the result stream.  Let's reset the GC timer, try to
                        // match it up then do a GC pass
                        projection.gc_interval.reset();
                        if let Some(context) = projection.waiting_for_match.remove(&index) {
                            trace!("Found a waiting match");
                            item = Some((context, result));
                            break;
                        } else {
                            warn!("We have a result without a context.  Holding onto this but this is unexpected behavior");
                            projection.out_of_order_results.insert(index, result);
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
        // "waiting" list and return expired entries TODO: if final it should
        // return all entries in waiting_for_match
        let trim_instant = Instant::now() - *projection.timeout;
        for entry in projection.waiting_for_match.entries() {
            if trim_instant > entry.get().start_time() {
                let context = entry.remove();
                let result = context.create_timeout_result();
                to_ret.push((context, result));
            }
        }

        if *projection.final_pass {
            if to_ret.is_empty() {
                Poll::Ready(None)
            } else {
                // We are at the end of it all.  Poll one more time so we can get a None
                cx.waker().wake_by_ref();
                Poll::Ready(Some(to_ret))
            }
        } else {
            if to_ret.is_empty() {
                Poll::Pending
            } else {
                Poll::Ready(Some(to_ret))
            }
        }
    }
}
mod test {
    use tokio_test::stream_mock::StreamMockBuilder;

    use super::*;

    #[derive(Debug)]
    struct TestContext {
        ctx: String,
        started: Instant,
    }

    impl TestContext {
        fn new(s: &str) -> Self {
            TestContext {
                ctx: s.to_string(),
                started: Instant::now(),
            }
        }
    }

    impl Context for TestContext {
        type Result = String;

        fn start_time(&self) -> Instant {
            self.started
        }

        fn create_timeout_result(&self) -> Self::Result {
            format!("{} timeout", self.ctx)
        }
    }

    impl Index for &str {}

    impl Result for String {}

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
        let results: Vec<(TestContext, String)> = reactor(context_stream_mock, result_stream_mock, Duration::from_millis(500)).collect().await;
        assert_eq!(results.get(0).unwrap().1, "result1");
        assert_eq!(results.get(1).unwrap().1, "result2");
        assert_eq!(results.get(2).unwrap().1, "result3");
    }

    #[tokio::test]
    async fn basic_timeout() {
        setup_tracing();
        let context_stream_mock = StreamMockBuilder::new()
            .next(("target1", TestContext::new("context1")))
            .next(("target2", TestContext::new("context2")))
            .build();

        let result_stream_mock = StreamMockBuilder::new()
            .next(("target1", "result1".to_string()))
            .wait(Duration::from_secs(2))
            .next(("target2", "result2".to_string()))
            .build();
        let results: Vec<(TestContext, String)> = reactor(
            context_stream_mock,
            result_stream_mock,
            Duration::from_millis(500),
        )
        .collect()
        .await;
        assert_eq!(results.get(0).unwrap().1, "result1");
        assert_eq!(results.get(1).unwrap().1, "context2 timeout");
    }
}
