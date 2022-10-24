//! Types related to [`AsyncFlush`](AsyncFlush) to wrap encoders

use futures_core::Future;
use futures_core::Stream;
use futures_io::AsyncRead;
use pin_project_lite::pin_project;

use super::bufread::Encoder;
use crate::codec::Encode;

/// Flushes asynchronously
///
/// `AsyncRead` and `AsyncBufRead` implementations may not have enough information
/// to know when to flush the data they have in store, so they can implement this
/// trait and let the caller decide when data should be flushed
pub trait AsyncFlush {
    /// Attempts to flush in flight data from the `AsyncFlush` into `buf`.
    ///
    /// On success, returns `Poll::Ready(Ok(()))` and places data in the
    /// unfilled portion of `buf`. If no data was read (`buf.filled().len()` is
    /// unchanged), it implies that EOF has been reached.
    ///
    /// If no data is available for reading, the method returns `Poll::Pending`
    /// and arranges for the current task (via `cx.waker()`) to receive a
    /// notification when the object becomes readable or is closed.
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>>;
}

pin_project! {
    /// This structure wraps an `Encoder` implementing [`AsyncRead`](tokio::io::AsyncRead) to
    /// allow the caller to flush its buffers.
    pub struct FlushableEncoder<E: AsyncRead, Rx: Stream<Item=()>> {
        #[pin]
        encoder: E,
        #[pin]
        receiver: Rx,
    }
}

impl<E: AsyncRead + AsyncFlush, Rx: Stream<Item = ()>> FlushableEncoder<E, Rx> {
    /// Creates a new `FlushableEncoder` from an existing `Encoder` and a Stream
    ///
    /// Whenever a message is received from the stream, the encoder will flushes its buffers
    /// and compress them.
    pub fn new(encoder: E, receiver: Rx) -> Self {
        Self { encoder, receiver }
    }
}

impl<E: AsyncRead + AsyncFlush, Rx: Stream<Item = ()>> AsyncRead for FlushableEncoder<E, Rx> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut this = self.project();

        match this.encoder.as_mut().poll_read(cx, buf) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r),
            std::task::Poll::Pending => match this.receiver.as_mut().poll_next(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(_) => match this.encoder.poll_flush(cx, buf) {
                    std::task::Poll::Ready(Ok(sz)) => std::task::Poll::Ready(Ok(sz)),
                    std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                },
            },
        }
    }
}
