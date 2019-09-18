use std::{
    io::Result,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{codec::Encode, util::PartialBuffer};
use bytes::{Bytes, BytesMut};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

const OUTPUT_BUFFER_SIZE: usize = 8_000;

#[derive(Debug)]
enum State {
    Reading,
    Writing,
    Flushing,
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
            state: State::Reading,
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
                    if this.input.is_empty() {
                        *this.state = State::Reading;
                        continue;
                    }

                    *this.state = State::Writing;

                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let mut input = PartialBuffer::new(this.input.as_ref());
                    let mut output = PartialBuffer::new(this.output.as_mut());

                    this.encoder.encode(&mut input, &mut output)?;

                    let input_len = input.written().len();
                    this.input.advance(input_len);

                    let output_len = output.written().len();
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Flushing => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let mut output = PartialBuffer::new(this.output.as_mut());

                    let done = this.encoder.finish(&mut output)?;

                    *this.state = if done { State::Done } else { State::Flushing };

                    let output_len = output.written().len();
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("Encoder reached invalid state"),
            };
        }
    }
}
