use std::{collections::HashMap, fmt::Debug, hash::Hash, pin::Pin, task::Poll};

use futures::{stream::BoxStream, Stream, StreamExt, stream, FutureExt, ready};
use futures::stream::select;
use pin_project::pin_project;
use tracing::{debug, error, trace, warn};

//TODO: Add a Timeoutable<T> to wrap R.  It should allow a Timeout stream event to come on the stream.  It will generate a timeout event if a C exists, will be dropped if it doesn't (IE a result has already come)

enum Item<C, R> {
    Context(C),
    Result(R)
}

#[pin_project]
pub struct Reactor<Index: Clone + Debug + Eq + Hash + PartialEq, Context, Result: Debug> {
    waiting_for_match: HashMap<Index, Context>,
    out_of_order_results: HashMap<Index, Result>,
    input: BoxStream<'static, (Index, Item<Context, Result>)>
}

impl<Index: Clone + Debug + Eq + Hash + PartialEq, Context, Result: Debug>
    Reactor<Index, Context, Result>
{
    pub fn new(
        context_stream: impl Stream<Item = (Index, Context)> + 'static + Send,
        results_stream: impl Stream<Item = (Index, Result)> + 'static + Send,
    ) -> Self {

        let input = select(context_stream.map(|(i, c)| (i, Item::Context(c))),
        results_stream.map(|(i, r)| (i, Item::Result(r))));


        Self {
            waiting_for_match: Default::default(),
            out_of_order_results: Default::default(),
            input: input.boxed()
        }
    }
}

impl<Index: Clone + Debug + Eq + Hash + PartialEq, Context, Result: Debug> Stream
    for Reactor<Index, Context, Result>
{
    type Item = (Context, Result);

    #[tracing::instrument(level = "trace", skip(self, cx))]
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        loop {
            match self.as_mut().internal_poll(cx) {
                None => {}
                Some(x) => {
                    return x
                }
            }
        }
    }
}

impl<Index: Clone + Debug + Eq + Hash + PartialEq, Context, Result: Debug>  Reactor<Index, Context, Result> {
    fn internal_poll(mut self: Pin<&mut Self>,
                     cx: &mut std::task::Context<'_>,) -> Option<Poll<Option<(Context, Result)>>> {
        match self.input.as_mut().poll_next(cx) {
            Poll::Ready(Some((index, Item::Result(result)))) => {
                match self.waiting_for_match.remove(&index) {
                    Some(context) => {
                        debug!("We got a match!  Yielding");
                        Some(Poll::Ready(Some((context, result))))
                    }
                    None => {
                        debug!(
                            "We have an item with an unknown key: {:?}.  Holding on to it",
                            index
                        );
                        if let Some(_) = self.out_of_order_results.insert(index, result){
                            warn!("non unique index")
                        }
                        None
                    }
                }
            }
            Poll::Ready(Some((index, Item::Context(context)))) => {
                if let Some(result) = self.out_of_order_results.remove(&index) {
                    debug!("We matched with an earlier out of order result for {:?}", index);
                    return Some(Poll::Ready(Some((context, result))))
                }
                if let Some(_) = self.waiting_for_match.insert(index, context) {
                    error!("Inserted a new value and it replaced a previously used value.  This is a sign that the index was non-unique");
                }
                None
            }
            Poll::Ready(None) => {
                Some(Poll::Ready(None))
            }
            Poll::Pending => {
                Some(Poll::Pending)
            }
        }
    }
}


#[tokio::test]
async fn basic_reactor_test(){
    let context_stream = vec![
        ("target1", "context1"),
        ("target2", "context2"),
        ("target3", "context3"),
    ];

    let result_stream = vec![
        ("target1", "result1"),
        ("target2", "result2"),
        ("target3", "result3"),
    ];

    let reactor = Reactor::new(stream::iter(context_stream), stream::iter(result_stream));
    let v: Vec<(&str, &str)> = reactor.collect().await;
    assert_eq!(v, vec![("context1", "result1"),("context2", "result2"),("context3", "result3"),]);
}

#[tokio::test]
async fn out_of_order() {
    let context_stream = vec![
        ("target1", 1u32),
        ("target2", 2),
        ("target3", 3),
    ];

    let result_stream = vec![
        ("target3", "result3"),
        ("target2", "result2"),
        ("target1", "result1"),
    ];

    let reactor = Reactor::new(stream::iter(context_stream), stream::iter(result_stream));
    let matched: Vec<_> = reactor.map(|(context, result)| {
        (&format!("result{}", context)) == result
    }).collect().await;
    assert_eq!(matched, vec![true, true, true])
}