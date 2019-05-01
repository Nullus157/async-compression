use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use brotli2::raw::{CoStatus, CompressOp};
pub use brotli2::{raw::Compress, CompressParams};
use bytes::{BufMut, Bytes, BytesMut};
use futures::{ready, stream::Stream};

pub struct BrotliStream<S: Stream<Item = Result<Bytes>>> {
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for BrotliStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let (inner, flushing, input_buffer, compress) = unsafe {
            let BrotliStream {
                inner,
                flushing,
                input_buffer,
                compress,
            } = self.get_unchecked_mut();
            (Pin::new_unchecked(inner), flushing, input_buffer, compress)
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

        let mut compressed_output = BytesMut::with_capacity(OUTPUT_BUFFER_SIZE);
        let input_ref = &mut input_buffer.as_ref();
        let output_ref = &mut &mut [][..];
        loop {
            let status = compress.compress(
                if *flushing {
                    CompressOp::Finish
                } else {
                    CompressOp::Process
                },
                input_ref,
                output_ref,
            )?;
            while let Some(buf) = dbg!(compress.take_output(None)) {
                compressed_output.put(buf);
            }
            match status {
                CoStatus::Finished => break,
                CoStatus::Unfinished => (),
            }
        }
        input_buffer.clear();

        Poll::Ready(Some(Ok(compressed_output.freeze())))
    }
}

impl<S: Stream<Item = Result<Bytes>>> BrotliStream<S> {
    pub fn new(stream: S, compress: Compress) -> BrotliStream<S> {
        BrotliStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            compress,
        }
    }
}
