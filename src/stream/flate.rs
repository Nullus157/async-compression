use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use bytes::{Bytes, BytesMut};
use flate2::FlushCompress;
pub use flate2::{Compress, Compression};
use futures::{ready, stream::Stream};

pub struct CompressedStream<S: Stream<Item = Result<Bytes>>> {
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

        let (inner, flushing, input_buffer, output_buffer, compress) = unsafe {
            let CompressedStream {
                inner,
                flushing,
                input_buffer,
                output_buffer,
                compress,
            } = self.get_unchecked_mut();
            (
                Pin::new_unchecked(inner),
                flushing,
                input_buffer,
                output_buffer,
                compress,
            )
        };

        if input_buffer.is_empty() {
            if *flushing {
                return Poll::Ready(None);
            } else if let Some(bytes) = ready!(inner.poll_next(cx)) {
                *input_buffer = bytes?;
            } else {
                *flushing = true;
            }
        }

        output_buffer.resize(OUTPUT_BUFFER_SIZE, 0);

        let flush = if *flushing {
            FlushCompress::Finish
        } else {
            FlushCompress::None
        };

        let (prior_in, prior_out) = (compress.total_in(), compress.total_out());
        compress.compress(input_buffer, output_buffer, flush)?;
        let input = compress.total_in() - prior_in;
        let output = compress.total_out() - prior_out;

        input_buffer.advance(input as usize);
        Poll::Ready(Some(Ok(output_buffer.split_to(output as usize).freeze())))
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
