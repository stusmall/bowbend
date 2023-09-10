use std::{collections::HashMap, fmt::Debug, hash::Hash, pin::Pin, task::Poll};
use std::sync::Arc;
use std::time::Duration;

use futures::{stream::BoxStream, Stream, StreamExt, stream, FutureExt, TryFutureExt};
use futures::stream::select;
use pin_project::pin_project;
use tokio::time::{Instant, Interval, interval};
use tracing::{debug, error, trace, warn};
use linked_hash_map::LinkedHashMap;
use crate::logging::setup_tracing;

//TODO: remove debug
pub(crate) trait Context: Debug + Send {
    type Result: Result;

    fn start_time(&self) -> Instant;

    fn create_timeout_result(&self) -> Self::Result;
}

//TODO: remove debug
pub(crate) trait Result: Debug + Send {

}

pub(crate) trait Index: Clone + Debug + Eq + Hash + PartialEq {

}

enum Item<I, C, R> {
    Context((I, C)),
    Result((I, R)),
}

#[pin_project]
pub(crate) struct Reactor<I: Index, C: Context> {
    waiting_for_match: LinkedHashMap<I, C>,
    out_of_order_results: HashMap<I, C::Result>,
    #[pin]
    input: BoxStream<'static, Item<I, C, C::Result>>, //TODO: try and make this generic rather than boxed
    #[pin]
    gc_interval: Interval,
    final_pass: bool
}

impl<I: Index, C: Context>
    Reactor<I, C>
{
    pub fn new(
        context_stream: impl Stream<Item = (I, C)> + 'static + Send,
        results_stream: impl Stream<Item = (I, C::Result)> + 'static + Send,
        timeout: Duration
    ) -> Self {

        let input = select(context_stream.map(|(i, c)| Item::Context((i, c))),
        results_stream.map(|(i, r)| Item::Result((i, r))));

        Self {
            waiting_for_match: Default::default(),
            out_of_order_results: Default::default(),
            input: input.boxed(),
            gc_interval: interval(timeout),
            final_pass: false
        }
    }
}

impl<I: Index, C: Context> Stream
    for Reactor<I, C>
{
    type Item = Vec<(C, C::Result)>;

    #[tracing::instrument(level = "trace", ret, skip(self, cx))]
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut projection = self.as_mut().project();
        let mut item = None;
        if *projection.final_pass == false {
            loop {
                match projection.input.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Item::Context((index, context)))) => {
                        // We have an item from the context stream.  Let's reset the GC timer, try and match it up, then do a GC pass
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
                        // We have an item from the result stream.  Let's reset the GC timer, try to match it up then do a GC pass
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
                        // Both our streams have finished.  This is the only case where we should skip reseting the timer.  Let it expire one more time and then we are finished.
                        debug!("Both streams have finished");
                        *projection.final_pass = true;
                        break;
                    },
                    Poll::Pending => {
                        trace!("Pending");
                        // The streams are pending. This could have been because of the timer or even
                        // an event outside this module.  Either way, reset the timer and do a GC pass.
                        projection.gc_interval.reset();
                        break;
                    },
                }
            };
        }

        let mut to_ret = item.map(|x| vec![x]).unwrap_or_default();

        // Do a GC pass.
        // Any items started before this Instant have timed out.  Remove them from our "waiting" list and return expired entries
        //TODO: if final it should return all entries in waiting_for_match
        let trim_instant = Instant::now() - projection.gc_interval.period();

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
                //TODO: There must be a better way to do this.  We have something to return on this
                // tick but want poll called one more time almost immediately to return Ready(None)
                *projection.gc_interval = interval(Duration::from_millis(1));
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

#[tokio::test]
async fn basic_reactor_test(){
    setup_tracing();
    #[derive(Debug)]
    struct TestContext {
        ctx: String,
        started: Instant
    }

    impl TestContext {
        fn new(s: &str) -> Self {
            TestContext {
                ctx: s.to_string(),
                started: Instant::now()
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

    impl Index for & str {

    }

    impl Result for String{}

    let context_stream = vec![
        ("target1", TestContext::new("context1")),
        ("target2", TestContext::new("context2")),
        ("target3", TestContext::new("context3")),
    ];

    let result_stream = vec![
        ("target1", "result1".to_string()),
        ("target2", "result2".to_string()),
        ("target3", "result3".to_string()),
    ];
    let reactor = Reactor::new(stream::iter(context_stream), stream::iter(result_stream), Duration::from_millis(500));
    let v: Vec<_> = reactor.collect().await;
    println!("{:#?}", v);
    panic!();
}

// impl<I: Index, C: Context, R: Result>  InnerReactor<I, C, R> {
//
//     fn handle_input(mut self: Pin<&mut Self>, item: Item<I, C, R>) -> Option<Vec<(C, R)>>{
//         match item {
//             Item::Context((index, context)) => {
//                 if let Some(result) = self.out_of_order_results.remove(&index){
//                     return Some(vec![(context, result)]);
//                 }
//             }
//             Item::Result((index, result)) => {
//                 if let Some(context) = self.waiting_for_match.remove(&index) {
//                     return Some(vec![(context, result)])
//                 }
//             }
//         }
//         unimplemented!()
//
//     }
//
//     fn internal_poll(mut self: Pin<&mut Self>,
//                      cx: &mut std::task::Context<'_>,) -> Option<Poll<Option<(C, R)>>> {
//         match self.input.as_mut().poll_next(cx) {
//             Poll::Ready(Some(Item::Result((index, result)))) => {
//                 match self.waiting_for_match.remove(&index) {
//                     Some(context) => {
//                         debug!("We got a match!  Yielding");
//                         Some(Poll::Ready(Some((context, result))))
//                     }
//                     None => {
//                         debug!(
//                             "We have an item with an unknown key: {:?}.  Holding on to it",
//                             index
//                         );
//                         if let Some(_) = self.out_of_order_results.insert(index, result){
//                             warn!("non unique index")
//                         }
//                         None
//                     }
//                 }
//             }
//             Poll::Ready(Some(Item::Context((index, context)))) => {
//                 if let Some(result) = self.out_of_order_results.remove(&index) {
//                     debug!("We matched with an earlier out of order result for {:?}", index);
//                     return Some(Poll::Ready(Some((context, result))))
//                 }
//                 if let Some(_) = self.waiting_for_match.insert(index, context) {
//                     error!("Inserted a new value and it replaced a previously used value.  This is a sign that the index was non-unique");
//                 }
//                 None
//             }
//             Poll::Ready(Some(ScanForTimeouts)) => {
//                 //TODO: Scan waiting entries
//                 None
//             }
//             Poll::Ready(None) => {
//                 Some(Poll::Ready(None))
//             }
//             Poll::Pending => {
//                 Some(Poll::Pending)
//             }
//         }
//     }
// }



// #[tokio::test]
// async fn basic_reactor_test(){
//     let context_stream = vec![
//         ("target1", "context1"),
//         ("target2", "context2"),
//         ("target3", "context3"),
//     ];
//
//     let result_stream = vec![
//         ("target1", "result1"),
//         ("target2", "result2"),
//         ("target3", "result3"),
//     ];
//
//     let reactor = Reactor::new(stream::iter(context_stream), stream::iter(result_stream));
//     let v: Vec<(&str, &str)> = reactor.collect().await;
//     assert_eq!(v, vec![("context1", "result1"),("context2", "result2"),("context3", "result3"),]);
// }
//
// #[tokio::test]
// async fn out_of_order() {
//     let context_stream = vec![
//         ("target1", 1u32),
//         ("target2", 2),
//         ("target3", 3),
//     ];
//
//     let result_stream = vec![
//         ("target3", "result3"),
//         ("target2", "result2"),
//         ("target1", "result1"),
//     ];
//
//     let reactor = Reactor::new(stream::iter(context_stream), stream::iter(result_stream));
//     let matched: Vec<_> = reactor.map(|(context, result)| {
//         (&format!("result{}", context)) == result
//     }).collect().await;
//     assert_eq!(matched, vec![true, true, true])
// }