use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use pin_project::pin_project;


#[derive(Eq, PartialEq)]
enum InternalState {
    Start,
    SecondaryFinished,
    MainFinished,
}

impl Default for InternalState {
    fn default() -> Self {
        InternalState::Start
    }
}

impl InternalState {
    fn finish(&mut self, side: PollNext) {
        match side {
            PollNext::Main => *self = InternalState::MainFinished,
            PollNext::Secondary => *self = InternalState::SecondaryFinished,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
enum PollNext {
    /// Poll the first stream.
    Main,
    /// Poll the second stream.
    Secondary,
}

impl PollNext {
    /// Toggle the value and return the old one.
    pub fn toggle(&mut self) -> Self {
        let old = *self;
        *self = self.other();
        old
    }

    fn other(&self) -> PollNext {
        match self {
            PollNext::Main => PollNext::Secondary,
            PollNext::Secondary => PollNext::Main,
        }
    }
}

impl Default for PollNext {
    fn default() -> Self {
        PollNext::Main
    }
}

pub(crate) fn half_select<T, Main: Stream<Item = T>, Secondary: Stream<Item = T>>(
    main_stream: Main,
    secondary_stream: Secondary,
) -> HalfSelect<T, Main, Secondary> {
    HalfSelect {
        main_stream,
        secondary_stream,
        poll_next: Default::default(),
        state: Default::default(),
    }
}

#[pin_project(project=HalfSelectProjected)]
pub(crate) struct HalfSelect<T, Main: Stream<Item = T>, Secondary: Stream<Item = T>> {
    #[pin]
    main_stream: Main,
    #[pin]
    secondary_stream: Secondary,
    poll_next: PollNext,
    state: InternalState,
}

impl<T, Main: Stream<Item = T>, Secondary: Stream<Item = T>> Stream
    for HalfSelect<T, Main, Secondary>
{
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut projection = self.as_mut().project();
        match projection.state {
            InternalState::Start => {
                let next_side = projection.poll_next.toggle();
                poll_inner(&mut projection, next_side, cx)
            }
            InternalState::SecondaryFinished => match projection.main_stream.poll_next(cx) {
                Poll::Ready(None) => {
                    *projection.state = InternalState::MainFinished;
                    Poll::Ready(None)
                }
                a => a,
            },
            InternalState::MainFinished => Poll::Ready(None),
        }
    }
}

#[inline]
fn poll_inner<T, Main: Stream<Item = T>, Secondary: Stream<Item = T>>(
    projection: &mut HalfSelectProjected<T, Main, Secondary>,
    next_side: PollNext,
    cx: &mut Context<'_>,
) -> Poll<Option<T>> {
    let first_done = match poll_side(projection, next_side, cx) {
        Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
        Poll::Ready(None) => {
            projection.state.finish(next_side);
            if *projection.state == InternalState::MainFinished {
                return Poll::Ready(None);
            }
            true
        }
        Poll::Pending => false,
    };
    let other = next_side.other();
    match poll_side(projection, other, cx) {
        Poll::Ready(None) => {
            projection.state.finish(other);
            if first_done {
                Poll::Ready(None)
            } else {
                Poll::Pending
            }
        }
        a => a,
    }
}

#[inline]
fn poll_side<T, Main: Stream<Item = T>, Secondary: Stream<Item = T>>(
    projection: &mut HalfSelectProjected<T, Main, Secondary>,
    next_side: PollNext,
    cx: &mut Context<'_>,
) -> Poll<Option<T>> {
    match next_side {
        PollNext::Main => projection.main_stream.as_mut().poll_next(cx),
        PollNext::Secondary => projection.secondary_stream.as_mut().poll_next(cx),
    }
}

#[cfg(test)]
mod test {
    use futures::{stream, StreamExt};

    use super::*;

    #[tokio::test]
    async fn basic_half_select_test() {
        let main_stream_mock = stream::iter(vec![1, 2, 3]);

        let secondary_stream_mock = stream::iter(vec![5, 6, 7]);

        let v: Vec<_> = half_select(main_stream_mock, secondary_stream_mock)
            .collect()
            .await;
        assert_eq!(v, vec![1, 5, 2, 6, 3, 7])
    }

    #[tokio::test]
    async fn main_finished_early_half_select_test() {
        let main_stream_mock = stream::iter(vec![1]);
        let secondary_stream_mock = stream::iter(vec![5, 6, 7, 8, 9]);

        let v: Vec<_> = half_select(main_stream_mock, secondary_stream_mock)
            .collect()
            .await;
        // This will select from main first.  Find something, check secondary next.  Then it will check main again, get Read(None) and then immediately bail
        assert_eq!(v, vec![1, 5]);
    }
}
