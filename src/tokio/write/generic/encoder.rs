use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    codec::Encode,
    tokio::write::{AsyncBufWrite, BufWriter},
    util::PartialBuffer,
};
use futures_core::ready;
use pin_project_lite::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, ReadBuf};

pin_project! {
    #[derive(Debug)]
    pub struct Encoder<W, E> {
        #[pin]
        writer: BufWriter<W>,
        encoder: E,
        finished: bool
    }
}

impl<W: AsyncWrite, E: Encode> Encoder<W, E> {
    pub fn new(writer: W, encoder: E) -> Self {
        Self {
            writer: BufWriter::new(writer),
            encoder,
            finished: false,
        }
    }
}

impl<W, E> Encoder<W, E> {
    pub fn get_ref(&self) -> &W {
        self.writer.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.writer.get_mut()
    }

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
        self.project().writer.get_pin_mut()
    }

    pub(crate) fn get_encoder_ref(&self) -> &E {
        &self.encoder
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

impl<W: AsyncWrite, E: Encode> AsyncWrite for Encoder<W, E> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let mut this = self.project();

        let mut encodeme = PartialBuffer::new(buf);

        loop {
            let mut space =
                PartialBuffer::new(ready!(this.writer.as_mut().poll_partial_flush_buf(cx))?);
            this.encoder.encode(&mut encodeme, &mut space)?;
            let bytes_encoded = space.written().len();
            this.writer.as_mut().produce(bytes_encoded);
            if encodeme.unwritten().is_empty() {
                break;
            }
        }

        Poll::Ready(Ok(encodeme.written().len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        loop {
            let mut space =
                PartialBuffer::new(ready!(this.writer.as_mut().poll_partial_flush_buf(cx))?);
            let flushed = this.encoder.flush(&mut space)?;
            let bytes_encoded = space.written().len();
            this.writer.as_mut().produce(bytes_encoded);
            if flushed {
                break;
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        while !*this.finished {
            let mut space =
                PartialBuffer::new(ready!(this.writer.as_mut().poll_partial_flush_buf(cx))?);
            *this.finished = this.encoder.finish(&mut space)?;
            let bytes_encoded = space.written().len();
            this.writer.as_mut().produce(bytes_encoded);
        }
        this.writer.poll_shutdown(cx)
    }
}

impl<W: AsyncRead, E> AsyncRead for Encoder<W, E> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.get_pin_mut().poll_read(cx, buf)
    }
}

impl<W: AsyncBufRead, E> AsyncBufRead for Encoder<W, E> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        self.get_pin_mut().poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.get_pin_mut().consume(amt)
    }
}
