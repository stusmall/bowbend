use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Stream;
use pin_project::pin_project;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum PollNext {
    /// Poll the first stream.
    Left,
    /// Poll the second stream.
    Right,
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
            PollNext::Left => PollNext::Right,
            PollNext::Right => PollNext::Left,
        }
    }
}

impl Default for PollNext {
    fn default() -> Self {
        PollNext::Left
    }
}

#[pin_project]
struct HalfSelect<T, Main: Stream<Item = T >> {
    #[pin]
    main_stream: Main,
    #[pin]
    trigger_stream: IntervalStream,
    poll_next: PollNext
}



impl<T, Main: Stream<Item = T>> Stream for HalfSelect<T, Main>{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_next.toggle();
        // If either stream is finished, terminate.  Trigger should be a repeating stream, so it should
        // never end.  If it does, this is an issue and we should terminate the stream.   We really
        // care about when the main stream terminates
        match self.poll_side(cx) {
            Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
            Poll::Ready(None) => {
                return Poll::Ready(None)
            }
            Poll::Pending => (),
        };
        self.poll_side(cx)
    }
}

impl<T, Main: Stream<Item = T >, Trigger: Stream<Item = T>> HalfSelect<T, Main, Trigger>{
    fn poll_side(mut self: Pin<&mut Self>,  cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut projection = self.as_mut().project();
        match projection.poll_next{
            PollNext::Left => projection.main_stream.as_mut().poll_next(cx),
            PollNext::Right => projection.trigger_stream.as_mut().poll_next(cx)
        }
    }
}