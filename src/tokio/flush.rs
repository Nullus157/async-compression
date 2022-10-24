//! Types related to [`AsyncFlush`](AsyncFlush) to wrap encoders

use futures_core::Future;
use futures_core::Stream;
use pin_project_lite::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead};

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
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<bool>>;
}

pin_project! {
    /// This structure wraps an `Encoder` implementing [`AsyncRead`](tokio::io::AsyncRead) to
    /// allow the caller to flush its buffers.
    pub struct FlushableEncoder<E: AsyncRead> {
        #[pin]
        encoder: E,
        #[pin]
        receiver: futures_channel::mpsc::Receiver<()>,
    }
}

impl<E: AsyncRead + AsyncFlush> FlushableEncoder<E> {
    /// Creates a new `FlushableEncoder` and a channel sender from an existing `Encoder`
    ///
    /// Whenever a message is sent on the channel, the encoder will flushes its buffers
    /// and compress them.
    pub fn new(encoder: E) -> (Self, futures_channel::mpsc::Sender<()>) {
        let (sender, receiver) = futures_channel::mpsc::channel(1);
        (Self { encoder, receiver }, sender)
    }
}

impl<E: AsyncRead + AsyncFlush> AsyncRead for FlushableEncoder<E> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut this = self.project();

        match this.encoder.as_mut().poll_read(cx, buf) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r),
            std::task::Poll::Pending => match this.receiver.as_mut().poll_next(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(_) => match this.encoder.poll_flush(cx, buf) {
                    std::task::Poll::Ready(Ok(true)) => std::task::Poll::Ready(Ok(())),
                    std::task::Poll::Ready(Ok(false)) => std::task::Poll::Pending,
                    std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                },
            },
        }
    }
}
