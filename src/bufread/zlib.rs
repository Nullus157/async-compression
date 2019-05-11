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

#[unsafe_project(Unpin)]
pub struct ZlibEncoder<R: AsyncBufRead> {
    #[pin]
    inner: R,
    flushing: bool,
    compress: Compress,
}

impl<R: AsyncBufRead> AsyncRead for ZlibEncoder<R> {
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

impl<R: AsyncBufRead> ZlibEncoder<R> {
    pub fn new(read: R, level: Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            inner: read,
            flushing: false,
            compress: Compress::new(level, true),
        }
    }
}
