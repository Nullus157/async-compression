#![cfg(all(feature = "tokio", feature = "zstd"))]
#![allow(clippy::unusual_byte_groupings)]

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use async_compression::tokio::write::ZstdEncoder;
use tokio::io::{AsyncWrite, AsyncWriteExt as _};

/// <https://github.com/Nullus157/async-compression/issues/246>
#[tokio::test]
async fn issue_246() {
    let mut zstd_encoder =
        Transparent::new(Trace::new(ZstdEncoder::new(DelayedShutdown::default())));
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
        eprintln!("Transparent::poll_write = ...");
        let ret = self.project().inner.poll_write(cx, buf);
        eprintln!("Transparent::poll_write = {:?}", ret);
        ret
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        eprintln!("Transparent::poll_flush = ...");
        let ret = self.project().inner.poll_flush(cx);
        eprintln!("Transparent::poll_flush = {:?}", ret);
        ret
    }

    /// To quote the [`AsyncWrite`] docs:
    /// > Invocation of a shutdown implies an invocation of flush.
    /// > Once this method returns Ready it implies that a flush successfully happened before the shutdown happened.
    /// > That is, callers don't need to call flush before calling shutdown.
    /// > They can rely that by calling shutdown any pending buffered data will be written out.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        eprintln!("Transparent::poll_shutdown = ...");
        let mut this = self.project();
        let ret = match this.inner.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => this.inner.poll_shutdown(cx),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        };
        eprintln!("Transparent::poll_shutdown = {:?}", ret);
        ret
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
        eprintln!("DelayedShutdown::poll_write = ...");
        let _ = cx;
        self.project().contents.extend_from_slice(buf);
        let ret = Poll::Ready(Ok(buf.len()));
        eprintln!("DelayedShutdown::poll_write = {:?}", ret);
        ret
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        eprintln!("DelayedShutdown::poll_flush = ...");
        let _ = cx;
        let ret = Poll::Ready(Ok(()));
        eprintln!("DelayedShutdown::poll_flush = {:?}", ret);
        ret
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        eprintln!("DelayedShutdown::poll_shutdown = ...");
        let ret = match self.project().num_times_shutdown_called {
            it @ 0 => {
                *it += 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            _ => Poll::Ready(Ok(())),
        };
        eprintln!("DelayedShutdown::poll_shutdown = {:?}", ret);
        ret
    }
}

pin_project_lite::pin_project! {
    /// A wrapper which traces all calls
    struct Trace<T> {
        #[pin] inner: T
    }
}

impl<T> Trace<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: AsyncWrite> AsyncWrite for Trace<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        eprintln!("Trace::poll_write = ...");
        let ret = self.project().inner.poll_write(cx, buf);
        eprintln!("Trace::poll_write = {:?}", ret);
        ret
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        eprintln!("Trace::poll_flush = ...");
        let ret = self.project().inner.poll_flush(cx);
        eprintln!("Trace::poll_flush = {:?}", ret);
        ret
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        eprintln!("Trace::poll_shutdown = ...");
        let ret = self.project().inner.poll_shutdown(cx);
        eprintln!("Trace::poll_shutdown = {:?}", ret);
        ret
    }
}
