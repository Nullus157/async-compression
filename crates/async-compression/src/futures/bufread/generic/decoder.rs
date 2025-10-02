use crate::codecs::Decode;
use crate::core::util::PartialBuffer;
use crate::generic::bufread::{AsyncBufRead as GenericAsyncBufRead, Decoder as GenericDecoder};

use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::{IoSlice, Result};

use futures_core::ready;
use futures_io::{AsyncBufRead, AsyncRead, AsyncWrite};
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]
    pub struct Decoder<R, D> {
        #[pin]
        reader: R,
        decoder: D,
        inner: GenericDecoder,
    }
}

impl<R: AsyncBufRead, D: Decode> Decoder<R, D> {
    pub fn new(reader: R, decoder: D) -> Self {
        Self {
            reader,
            decoder,
            inner: GenericDecoder::default(),
        }
    }
}

impl<R, D> Decoder<R, D> {
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut R> {
        self.project().reader
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    pub fn multiple_members(&mut self, enabled: bool) {
        self.inner.multiple_members(enabled);
    }
}

impl<R: AsyncBufRead, D: Decode> Decoder<R, D> {
    pub(crate) fn do_poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Poll<Result<()>> {
        let this = self.project();

        struct Reader<'a, R>(Pin<&'a mut R>);

        impl<R: AsyncBufRead> GenericAsyncBufRead for Reader<'_, R> {
            fn poll_fill_buf(&mut self, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
                self.0.as_mut().poll_fill_buf(cx)
            }
            fn consume(&mut self, bytes: usize) {
                self.0.as_mut().consume(bytes)
            }
        }

        this.inner
            .do_poll_read(cx, output, &mut Reader(this.reader), this.decoder)
    }
}

impl<R: AsyncBufRead, D: Decode> AsyncRead for Decoder<R, D> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let mut output = PartialBuffer::new(buf);
        match self.do_poll_read(cx, &mut output)? {
            Poll::Pending if output.written().is_empty() => Poll::Pending,
            _ => Poll::Ready(Ok(output.written().len())),
        }
    }
}

impl<R: AsyncWrite, D> AsyncWrite for Decoder<R, D> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.get_pin_mut().poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_pin_mut().poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_pin_mut().poll_close(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>> {
        self.get_pin_mut().poll_write_vectored(cx, bufs)
    }
}
