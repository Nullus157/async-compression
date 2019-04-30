use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use bytes::{Bytes, BytesMut};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
    stream::Stream,
};
use pin_project::unsafe_project;

use flate2::FlushCompress;
pub use flate2::{Compress, Compression};

#[unsafe_project(Unpin)]
pub struct CompressedStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    output_buffer: BytesMut,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for CompressedStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

        if this.input_buffer.is_empty() {
            if *this.flushing {
                return Poll::Ready(None);
            } else if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
                *this.input_buffer = bytes?;
            } else {
                *this.flushing = true;
            }
        }

        this.output_buffer.resize(OUTPUT_BUFFER_SIZE, 0);

        let flush = if *this.flushing {
            FlushCompress::Finish
        } else {
            FlushCompress::None
        };

        let (prior_in, prior_out) = (this.compress.total_in(), this.compress.total_out());
        this.compress.compress(this.input_buffer, this.output_buffer, flush)?;
        let input = this.compress.total_in() - prior_in;
        let output = this.compress.total_out() - prior_out;

        this.input_buffer.advance(input as usize);
        Poll::Ready(Some(Ok(this.output_buffer.split_to(output as usize).freeze())))
    }
}

impl<S: Stream<Item = Result<Bytes>>> CompressedStream<S> {
    pub fn new(stream: S, compress: Compress) -> CompressedStream<S> {
        CompressedStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            output_buffer: BytesMut::new(),
            compress,
        }
    }
}

#[unsafe_project(Unpin)]
pub struct CompressedRead<R: AsyncBufRead> {
    #[pin]
    inner: R,
    flushing: bool,
    compress: Compress,
}

impl<R: AsyncBufRead> AsyncRead for CompressedRead<R> {
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

impl<R: AsyncBufRead> CompressedRead<R> {
    pub fn new(read: R, compress: Compress) -> CompressedRead<R> {
        CompressedRead {
            inner: read,
            flushing: false,
            compress,
        }
    }
}
