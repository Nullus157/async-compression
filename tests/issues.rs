#![cfg(all(feature = "tokio", feature = "zstd"))]

use std::{
    io,
    pin::Pin,
    task::{ready, Context, Poll},
};

use async_compression::tokio::write::ZstdEncoder;
use tokio::io::{AsyncWrite, AsyncWriteExt as _};
use tracing_subscriber::fmt::format::FmtSpan;

/// This issue covers our state machine being invalid when using adapters
/// like [`tokio_util::codec`].
///
/// After the first [`poll_shutdown`] call,
/// we must expect any number of [`poll_flush`] and [`poll_shutdown`] calls,
/// until [`poll_shutdown`] returns [`Poll::Ready`],
/// according to the documentation on [`AsyncWrite`].
///
/// <https://github.com/Nullus157/async-compression/issues/246>
///
/// [`tokio_util::codec`](https://docs.rs/tokio-util/latest/tokio_util/codec)
/// [`poll_shutdown`](AsyncWrite::poll_shutdown)
/// [`poll_flush`](AsyncWrite::poll_flush)
#[test]
fn issue_246() {
    tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_level(false)
        .with_test_writer()
        .with_target(false)
        .with_span_events(FmtSpan::NEW)
        .init();
    let mut zstd_encoder =
        Transparent::new(Trace::new(ZstdEncoder::new(DelayedShutdown::default())));
    futures::executor::block_on(zstd_encoder.shutdown()).unwrap();
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
    #[tracing::instrument(name = "Transparent::poll_write", skip_all, ret)]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().inner.poll_write(cx, buf)
    }

    #[tracing::instrument(name = "Transparent::poll_flush", skip_all, ret)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().inner.poll_flush(cx)
    }

    /// To quote the [`AsyncWrite`] docs:
    /// > Invocation of a shutdown implies an invocation of flush.
    /// > Once this method returns Ready it implies that a flush successfully happened before the shutdown happened.
    /// > That is, callers don't need to call flush before calling shutdown.
    /// > They can rely that by calling shutdown any pending buffered data will be written out.
    #[tracing::instrument(name = "Transparent::poll_shutdown", skip_all, ret)]
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
    #[tracing::instrument(name = "DelayedShutdown::poll_write", skip_all, ret)]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let _ = cx;
        self.project().contents.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    #[tracing::instrument(name = "DelayedShutdown::poll_flush", skip_all, ret)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let _ = cx;
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(name = "DelayedShutdown::poll_shutdown", skip_all, ret)]
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
    #[tracing::instrument(name = "Trace::poll_write", skip_all, ret)]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().inner.poll_write(cx, buf)
    }
    #[tracing::instrument(name = "Trace::poll_flush", skip_all, ret)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().inner.poll_flush(cx)
    }

    #[tracing::instrument(name = "Trace::poll_shutdown", skip_all, ret)]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().inner.poll_shutdown(cx)
    }
}
