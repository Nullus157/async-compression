use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io;

use bytes::{Bytes, BytesMut};
use flate2::{Compress, Compression, FlushCompress};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
    stream::Stream,
};

pub fn compress_stream(stream: impl Stream<Item = io::Result<Bytes>>) -> impl Stream<Item = io::Result<Bytes>> {
    struct CompressedStream<S: Stream<Item = io::Result<Bytes>>> {
        inner: S,
        flushing: bool,
        input_buffer: Bytes,
        output_buffer: BytesMut,
        compress: Compress,
    }

    impl<S: Stream<Item = io::Result<Bytes>>> Stream for CompressedStream<S> {
        type Item = io::Result<Bytes>;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<io::Result<Bytes>>> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000; // 🤷

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
                }

                if let Some(bytes) = ready!(inner.poll_next(cx)) {
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

    CompressedStream {
        inner: stream,
        flushing: false,
        input_buffer: Bytes::new(),
        output_buffer: BytesMut::new(),
        compress: Compress::new(Compression::fast(), false),
    }
}

pub fn compress_read(read: impl AsyncBufRead) -> impl AsyncRead {
    struct CompressedRead<R: AsyncBufRead> {
        inner: R,
        flushing: bool,
        compress: Compress,
    }

    impl<R: AsyncBufRead> AsyncRead for CompressedRead<R> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let (mut inner, flushing, compress) = unsafe {
                let CompressedRead {
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

    CompressedRead {
        inner: read,
        flushing: false,
        compress: Compress::new(Compression::fast(), false),
    }
}
