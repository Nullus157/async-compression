use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::io::AsyncWrite;
use std::io::{IoSlice, Result};

pub trait TrackClosedExt: AsyncWrite {
    fn track_closed(self) -> TrackClosed<Self>
    where
        Self: Sized + Unpin,
    {
        TrackClosed {
            inner: self,
            closed: false,
        }
    }
}

impl<W: AsyncWrite> TrackClosedExt for W {}

pub struct TrackClosed<W: AsyncWrite + Unpin> {
    inner: W,
    closed: bool,
}

impl<W: AsyncWrite + Unpin> TrackClosed<W> {
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for TrackClosed<W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        assert!(!self.closed);
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        assert!(!self.closed);
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        assert!(!self.closed);
        match Pin::new(&mut self.inner).poll_close(cx) {
            Poll::Ready(Ok(())) => {
                self.closed = true;
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        bufs: &[IoSlice],
    ) -> Poll<Result<usize>> {
        assert!(!self.closed);
        Pin::new(&mut self.inner).poll_write_vectored(cx, bufs)
    }
}
