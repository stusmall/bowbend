use std::{
    ops::Range,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use pin_project::pin_project;
use rand::{prelude::StdRng, Rng, SeedableRng};
use tokio::time::{Duration, Instant, Sleep};

/// This consumes a stream of T and creates a new stream that pause a random
/// value inside `range` before emitting the next value
pub(crate) fn throttle_stream<T, S: 'static + Stream<Item = T> + Send>(
    range: Range<u64>,
    stream: S,
) -> ThrottledStream<T, StdRng, S> {
    ThrottledStream {
        range,
        sleeping: false,
        random: StdRng::from_entropy(),
        stream,
        sleep: tokio::time::sleep(Default::default()),
    }
}

#[pin_project]
pub(crate) struct ThrottledStream<T, R: Rng, S: 'static + Stream<Item = T> + Send> {
    range: Range<u64>,
    sleeping: bool,
    random: R,
    #[pin]
    stream: S,
    #[pin]
    sleep: Sleep,
}

impl<T, R: Rng, S: 'static + Stream<Item = T> + Send> Stream for ThrottledStream<T, R, S> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut projection = self.project();
        if *projection.sleeping {
            match projection.sleep.as_mut().poll(cx) {
                Poll::Ready(_) => {
                    *projection.sleeping = false;
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        match projection.stream.poll_next(cx) {
            Poll::Ready(x) => {
                let sleep_duration =
                    Duration::from_millis(projection.random.gen_range(projection.range.clone()));
                *projection.sleeping = true;
                projection.sleep.reset(Instant::now() + sleep_duration);
                Poll::Ready(x)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use futures::{stream, StreamExt};
    use rand::{rngs::SmallRng, SeedableRng};

    use crate::utils::throttle_stream::ThrottledStream;

    #[tokio::test]
    async fn basic_test() {
        let v = vec![1, 2, 3];
        // Let's set this up with a static seed for more reliable tests
        let random = SmallRng::seed_from_u64(123);
        let instant = Instant::now();
        let mut stream = ThrottledStream {
            range: (100..300),
            sleeping: false,
            random,
            stream: stream::iter(v),
            sleep: tokio::time::sleep(Default::default()),
        }
        .boxed();
        assert_eq!(stream.next().await, Some(1));
        let first = instant.elapsed();
        // Our first return should be instant
        assert!(first.as_millis() < 10);

        let instant = Instant::now();
        assert_eq!(stream.next().await, Some(2));
        // Now we should wait somewhere between 100ms and 300ms
        let second = instant.elapsed();
        assert!(second.as_millis() > 90 && second.as_millis() < 320);

        // Now lets grab one more value to confirm that also pauses
        let instant = Instant::now();
        assert_eq!(stream.next().await, Some(3));
        // Now we should wait somewhere between 100ms and 300ms
        let third = instant.elapsed();
        assert!(third.as_millis() > 90 && third.as_millis() < 320);

        assert_eq!(stream.next().await, None);
    }
}
