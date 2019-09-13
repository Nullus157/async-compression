use std::{
    io::Result,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::codec::Encode;
use bytes::{Bytes, BytesMut};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

const OUTPUT_BUFFER_SIZE: usize = 8_000;

#[derive(Debug)]
enum State {
    WritingHeader,
    Reading,
    Writing,
    Flushing,
    WritingFooter,
    Done,
    Invalid,
}

#[unsafe_project(Unpin)]
#[derive(Debug)]
pub struct Encoder<S: Stream<Item = Result<Bytes>>, E: Encode> {
    #[pin]
    stream: S,
    encoder: E,
    state: State,
    input: Bytes,
    output: BytesMut,
}

impl<S: Stream<Item = Result<Bytes>>, E: Encode> Encoder<S, E> {
    pub(crate) fn new(stream: S, encoder: E) -> Self {
        Self {
            stream,
            encoder,
            state: State::WritingHeader,
            input: Bytes::new(),
            output: BytesMut::new(),
        }
    }

    pub(crate) fn get_ref(&self) -> &S {
        &self.stream
    }

    pub(crate) fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub(crate) fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().stream
    }

    pub(crate) fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: Stream<Item = Result<Bytes>>, E: Encode> Stream for Encoder<S, E> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, State::Invalid) {
                State::WritingHeader => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    let output_len = this.encoder.write_header(&mut this.output)?;
                    *this.state = State::Reading;
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Reading => {
                    *this.state = State::Reading;
                    *this.state = match ready!(this.stream.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            if this.input.is_empty() {
                                *this.input = chunk?;
                            } else {
                                this.input.extend_from_slice(&chunk?);
                            }
                            State::Writing
                        }
                        None => State::Flushing,
                    };
                    continue;
                }

                State::Writing => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let (done, input_len, output_len) =
                        this.encoder.encode(&this.input, &mut this.output)?;

                    *this.state = if done { State::Reading } else { State::Writing };

                    this.input.advance(input_len as usize);
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Flushing => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let (done, output_len) = this.encoder.flush(&mut this.output)?;

                    *this.state = if done {
                        State::WritingFooter
                    } else {
                        State::Flushing
                    };
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::WritingFooter => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let output_len = this.encoder.write_footer(&mut this.output)?;
                    *this.state = State::Done;
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("GzipEncoder reached invalid state"),
            };
        }
    }
}
