use std::{
    io::Result,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
pub(crate) use flate2::Compress;
use flate2::{FlushCompress, FlushDecompress, Status, Decompress};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    Reading,
    Writing(Bytes),
    Flushing,
    Done,
    Invalid,
}

#[unsafe_project(Unpin)]
pub(crate) struct CompressedStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: State,
    output: BytesMut,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for CompressedStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn compress(
            compress: &mut Compress,
            input: &mut Bytes,
            output: &mut BytesMut,
            flush: FlushCompress,
        ) -> Result<(Status, Bytes)> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let (prior_in, prior_out) = (compress.total_in(), compress.total_out());
            let status = compress.compress(input, output, flush)?;
            let input_len = compress.total_in() - prior_in;
            let output_len = compress.total_out() - prior_out;

            input.advance(input_len as usize);
            Ok((status, output.split_to(output_len as usize).freeze()))
        }

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, State::Invalid) {
                State::Reading => {
                    *this.state = State::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => State::Writing(chunk?),
                        None => State::Flushing,
                    };
                    continue;
                }

                State::Writing(mut input) => {
                    if input.is_empty() {
                        *this.state = State::Reading;
                        continue;
                    }

                    let (status, chunk) = compress(
                        &mut this.compress,
                        &mut input,
                        &mut this.output,
                        FlushCompress::None,
                    )?;

                    *this.state = match status {
                        Status::Ok => State::Writing(input),
                        Status::StreamEnd => unreachable!(),
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::Flushing => {
                    let (status, chunk) = compress(
                        &mut this.compress,
                        &mut Bytes::new(),
                        &mut this.output,
                        FlushCompress::Finish,
                    )?;

                    *this.state = match status {
                        Status::Ok => State::Flushing,
                        Status::StreamEnd => State::Done,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("CompressedStream reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>>> CompressedStream<S> {
    pub(crate) fn new(stream: S, compress: Compress) -> CompressedStream<S> {
        CompressedStream {
            inner: stream,
            state: State::Reading,
            output: BytesMut::new(),
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
                this.input_buffer.extend_from_slice(&bytes?);
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
