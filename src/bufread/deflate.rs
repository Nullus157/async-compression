use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use flate2::{Compress, Compression, FlushCompress};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};
use pin_project::unsafe_project;

/// A DEFLATE encoder, or compressor.
///
/// This structure implements an [`AsyncRead`] interface and will read uncompressed data from an
/// underlying stream and emit a stream of compressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
pub struct DeflateEncoder<R: AsyncBufRead> {
    #[pin]
    inner: R,
    flushing: bool,
    compress: Compress,
}

impl<R: AsyncBufRead> DeflateEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(read: R, level: Compression) -> DeflateEncoder<R> {
        DeflateEncoder {
            inner: read,
            flushing: false,
            compress: Compress::new(level, false),
        }
    }

    /// Acquires a reference to the underlying reader that this encoder is wrapping.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying reader that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the reader which may
    /// otherwise confuse this encoder.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Acquires a pinned mutable reference to the underlying reader that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the reader which may
    /// otherwise confuse this encoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut R> {
        self.project().inner
    }

    /// Consumes this encoder returning the underlying reader.
    ///
    /// Note that this may discard internal state of this encoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: AsyncBufRead> AsyncRead for DeflateEncoder<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let mut this = self.project();

        loop {
            let input_buffer = ready!(this.inner.as_mut().poll_fill_buf(cx))?;
            *this.flushing = input_buffer.is_empty();

            let flush = if *this.flushing {
                FlushCompress::Finish
            } else {
                FlushCompress::None
            };

            let (prior_in, prior_out) = (this.compress.total_in(), this.compress.total_out());
            this.compress.compress(input_buffer, buf, flush)?;
            let input = this.compress.total_in() - prior_in;
            let output = this.compress.total_out() - prior_out;

            this.inner.as_mut().consume(input as usize);
            if *this.flushing || output > 0 {
                return Poll::Ready(Ok(output as usize));
            }
        }
    }
}
