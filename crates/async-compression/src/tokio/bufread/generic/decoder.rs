use crate::{codecs::Decode, core::util::PartialBuffer, generic::bufread::impl_do_poll_read};

use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::{IoSlice, Result};

use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, ReadBuf};

impl_do_poll_read!();

impl<R: AsyncBufRead, D: Decode> AsyncRead for Decoder<R, D> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let mut output = PartialBuffer::new(buf.initialize_unfilled());
        match self.do_poll_read(cx, &mut output)? {
            Poll::Pending if output.written().is_empty() => Poll::Pending,
            _ => {
                let len = output.written().len();
                buf.advance(len);
                Poll::Ready(Ok(()))
            }
        }
    }
}

impl<R: AsyncWrite, D: Decode> AsyncWrite for Decoder<R, D> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.get_pin_mut().poll_write(cx, buf)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>> {
        self.get_pin_mut().poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.get_ref().is_write_vectored()
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_pin_mut().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_pin_mut().poll_shutdown(cx)
    }
}
