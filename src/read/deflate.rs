use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

pub use flate2::Compression;
use flate2::{Compress, FlushCompress};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};

pub struct DeflateRead<R: AsyncBufRead> {
    inner: R,
    flushing: bool,
    compress: Compress,
}

impl<R: AsyncBufRead> AsyncRead for DeflateRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let (mut inner, flushing, compress) = unsafe {
            let DeflateRead {
                inner,
                flushing,
                compress,
            } = self.get_unchecked_mut();
            (Pin::new_unchecked(inner), flushing, compress)
        };

        loop {
            let input_buffer = ready!(inner.as_mut().poll_fill_buf(cx))?;
            if input_buffer.is_empty() {
                *flushing = true;
            }

            let flush = if *flushing {
                FlushCompress::Finish
            } else {
                FlushCompress::None
            };

            let (prior_in, prior_out) = (compress.total_in(), compress.total_out());
            compress.compress(input_buffer, buf, flush)?;
            let input = compress.total_in() - prior_in;
            let output = compress.total_out() - prior_out;

            inner.as_mut().consume(input as usize);
            if *flushing || output > 0 {
                return Poll::Ready(Ok(output as usize));
            }
        }
    }
}

impl<R: AsyncBufRead> DeflateRead<R> {
    pub fn new(read: R, level: Compression) -> DeflateRead<R> {
        DeflateRead {
            inner: read,
            flushing: false,
            compress: Compress::new(level, false),
        }
    }
}
