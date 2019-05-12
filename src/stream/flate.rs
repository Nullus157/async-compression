use std::{
    io::Result,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress, Status};
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
pub(crate) struct FlateEncoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: State,
    output: BytesMut,
    compress: Compress,
}

#[unsafe_project(Unpin)]
pub(crate) struct FlateDecoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: State,
    output: BytesMut,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> FlateEncoder<S> {
    pub(crate) fn new(stream: S, compress: Compress) -> FlateEncoder<S> {
        FlateEncoder {
            inner: stream,
            state: State::Reading,
            output: BytesMut::new(),
            compress,
        }
    }

    pub(crate) fn get_ref(&self) -> &S {
        &self.inner
    }

    pub(crate) fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    pub(crate) fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().inner
    }

    pub(crate) fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Stream<Item = Result<Bytes>>> FlateDecoder<S> {
    pub(crate) fn new(stream: S, decompress: Decompress) -> FlateDecoder<S> {
        FlateDecoder {
            inner: stream,
            state: State::Reading,
            output: BytesMut::new(),
            decompress,
        }
    }

    pub(crate) fn get_ref(&self) -> &S {
        &self.inner
    }

    pub(crate) fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    pub(crate) fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().inner
    }

    pub(crate) fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for FlateEncoder<S> {
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

                State::Invalid => panic!("FlateEncoder reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for FlateDecoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn decompress(
            decompress: &mut Decompress,
            input: &mut Bytes,
            output: &mut BytesMut,
            flush: FlushDecompress,
        ) -> Result<(Status, Bytes)> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let (prior_in, prior_out) = (decompress.total_in(), decompress.total_out());
            let status = decompress.decompress(input, output, flush)?;
            let input_len = decompress.total_in() - prior_in;
            let output_len = decompress.total_out() - prior_out;

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

                    let (status, chunk) = decompress(
                        &mut this.decompress,
                        &mut input,
                        &mut this.output,
                        FlushDecompress::None,
                    )?;

                    *this.state = match status {
                        Status::Ok => State::Writing(input),
                        Status::StreamEnd => State::Reading,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::Flushing => {
                    let (status, chunk) = decompress(
                        &mut this.decompress,
                        &mut Bytes::new(),
                        &mut this.output,
                        FlushDecompress::Finish,
                    )?;

                    *this.state = match status {
                        Status::Ok => State::Flushing,
                        Status::StreamEnd => State::Done,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("FlateEncoder reached invalid state"),
            };
        }
    }
}
