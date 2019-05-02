use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use bytes::{Bytes, BytesMut};
use flate2::{FlushCompress, FlushDecompress};
pub use flate2::{Compress, Compression, Decompress};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

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
        this.compress
            .compress(this.input_buffer, this.output_buffer, flush)?;
        let input = this.compress.total_in() - prior_in;
        let output = this.compress.total_out() - prior_out;

        this.input_buffer.advance(input as usize);
        Poll::Ready(Some(Ok(this
            .output_buffer
            .split_to(output as usize)
            .freeze())))
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
pub struct DecompressedStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    output_buffer: BytesMut,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for DecompressedStream<S> {
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
            FlushDecompress::Finish
        } else {
            FlushDecompress::None
        };

        let (prior_in, prior_out) = (this.decompress.total_in(), this.decompress.total_out());
        this.decompress
            .decompress(this.input_buffer, this.output_buffer, flush)?;
        let input = this.decompress.total_in() - prior_in;
        let output = this.decompress.total_out() - prior_out;

        this.input_buffer.advance(input as usize);
        Poll::Ready(Some(Ok(this
            .output_buffer
            .split_to(output as usize)
            .freeze())))
    }
}

impl<S: Stream<Item = Result<Bytes>>> DecompressedStream<S> {
    pub fn new(stream: S, decompress: Decompress) -> DecompressedStream<S> {
        DecompressedStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            output_buffer: BytesMut::new(),
            decompress,
        }
    }
}
