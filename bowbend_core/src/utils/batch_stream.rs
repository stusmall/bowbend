use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use pin_project::pin_project;

/// This accepts a stream of T and a batch size.  It will then create a new
/// stream of [`Vec<T>`].  As the original stream omits values this stream will
/// batch them up and only omit a [`Vec<T>`] once it reaches `batch_size`
pub(crate) fn batch_stream<T, S: 'static + Stream<Item = T> + Send>(
    batch_size: usize,
    stream: S,
) -> BatchStream<T, S> {
    BatchStream {
        batch_size,
        buffer: Vec::new(),
        stream,
    }
}

#[pin_project]
pub(crate) struct BatchStream<T, S: 'static + Stream<Item = T> + Send> {
    batch_size: usize,
    buffer: Vec<T>,
    #[pin]
    stream: S,
}

impl<T, S> Stream for BatchStream<T, S>
where
    S: 'static + Stream<Item = T> + Send,
{
    type Item = Vec<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let stream: Pin<&mut S> = this.stream;
        let buffer: &mut Vec<T> = this.buffer;
        let batch_size = *this.batch_size;
        match stream.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                buffer.push(value);
                if buffer.len() >= batch_size {
                    let batch_size = batch_size;
                    cx.waker().wake_by_ref();
                    Poll::Ready(Some(buffer.drain(0..batch_size).collect()))
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Poll::Ready(None) => {
                if buffer.is_empty() {
                    Poll::Ready(None)
                } else {
                    cx.waker().wake_by_ref();
                    let l = buffer.len();
                    Poll::Ready(Some(buffer.drain(0..l).collect()))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{stream, StreamExt};

    use super::batch_stream;

    #[tokio::test]
    async fn basic_test() {
        let v = vec![0, 1, 2];
        let mut stream = batch_stream(3, stream::iter(v));
        assert_eq!(stream.next().await, Some(vec![0, 1, 2]));
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test]
    async fn test_remainder() {
        let v = vec![0, 1, 2, 3];
        let mut stream = batch_stream(3, stream::iter(v));
        assert_eq!(stream.next().await, Some(vec![0, 1, 2]));
        assert_eq!(stream.next().await, Some(vec![3]));
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test]
    async fn test_empty() {
        let v: Vec<i32> = vec![];
        let mut stream = batch_stream(3, stream::iter(v));
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test]
    async fn test_batch_size_one() {
        let v: Vec<i32> = vec![1, 2, 3];
        let mut stream = batch_stream(1, stream::iter(v));
        assert_eq!(stream.next().await, Some(vec![1]));
        assert_eq!(stream.next().await, Some(vec![2]));
        assert_eq!(stream.next().await, Some(vec![3]));
        assert_eq!(stream.next().await, None);
    }
}
