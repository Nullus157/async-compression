use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use bytes::{BufMut, Bytes, BytesMut};
use futures::{
    //io::{AsyncBufRead, AsyncRead},
    ready,
    stream::Stream,
};

use brotli2::raw::{CoStatus, CompressOp};
pub use brotli2::{raw::Compress, CompressParams};

pub struct CompressedStream<S: Stream<Item = Result<Bytes>>> {
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for CompressedStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let (inner, flushing, input_buffer, compress) = unsafe {
            let CompressedStream {
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

impl<S: Stream<Item = Result<Bytes>>> CompressedStream<S> {
    pub fn new(stream: S, compress: Compress) -> CompressedStream<S> {
        CompressedStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            compress,
        }
    }
}
