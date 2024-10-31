#![cfg(all(feature = "tokio", feature = "zstd"))]
#![allow(clippy::unusual_byte_groupings)]

use std::{
    io,
    pin::Pin,
    task::{ready, Context, Poll},
};

use async_compression::tokio::write::ZstdEncoder;
use tokio::io::{AsyncWrite, AsyncWriteExt as _};

/// <https://github.com/Nullus157/async-compression/issues/246>
#[tokio::test]
async fn issue_246() {
    let mut zstd_encoder = Transparent::new(ZstdEncoder::new(DelayedShutdown::default()));
    zstd_encoder.shutdown().await.unwrap();
}

pin_project_lite::pin_project! {
    /// A simple wrapper struct that follows the [`AsyncWrite`] protocol.
    struct Transparent<T> {
        #[pin] inner: T
    }
}

impl<T> Transparent<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: AsyncWrite> AsyncWrite for Transparent<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().inner.poll_flush(cx)
    }

    /// To quote the [`AsyncWrite`] docs:
    /// > Invocation of a shutdown implies an invocation of flush.
    /// > Once this method returns Ready it implies that a flush successfully happened before the shutdown happened.
    /// > That is, callers don't need to call flush before calling shutdown.
    /// > They can rely that by calling shutdown any pending buffered data will be written out.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let mut this = self.project();
        ready!(this.inner.as_mut().poll_flush(cx))?;
        this.inner.poll_shutdown(cx)
    }
}

pin_project_lite::pin_project! {
    /// Yields [`Poll::Pending`] the first time [`AsyncWrite::poll_shutdown`] is called.
    #[derive(Default)]
    struct DelayedShutdown {
        contents: Vec<u8>,
        num_times_shutdown_called: u8,
    }
}

impl AsyncWrite for DelayedShutdown {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let _ = cx;
        self.project().contents.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let _ = cx;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.project().num_times_shutdown_called {
            it @ 0 => {
                *it += 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            _ => Poll::Ready(Ok(())),
        }
    }
}
