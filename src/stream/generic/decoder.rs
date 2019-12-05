use std::{
    io::Result,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{codec::Decode, util::PartialBuffer};
use bytes::{Bytes, BytesMut};
use futures_core::{ready, stream::Stream};
use pin_project_lite::pin_project;

const OUTPUT_BUFFER_SIZE: usize = 8_000;

#[derive(Debug)]
enum State {
    Reading,
    Writing,
    Flushing,
    Done,
    Invalid,
}

pin_project! {
    #[derive(Debug)]
    pub struct Decoder<S: Stream<Item = Result<Bytes>>, D: Decode> {
        #[pin]
        stream: S,
        decoder: D,
        state: State,
        input: Bytes,
        output: BytesMut,
    }
}

impl<S: Stream<Item = Result<Bytes>>, D: Decode> Decoder<S, D> {
    pub fn new(stream: S, decoder: D) -> Self {
        Self {
            stream,
            decoder,
            state: State::Reading,
            input: Bytes::new(),
            output: BytesMut::new(),
        }
    }

    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
        self.project().stream
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: Stream<Item = Result<Bytes>>, D: Decode> Stream for Decoder<S, D> {
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
                            *this.input = chunk?;
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

                    if this.output.len() < OUTPUT_BUFFER_SIZE {
                        this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    }

                    let mut input = PartialBuffer::new(this.input.as_ref());
                    let mut output = PartialBuffer::new(this.output.as_mut());

                    let done = this.decoder.decode(&mut input, &mut output)?;

                    let input_len = input.written().len();
                    this.input.advance(input_len);

                    *this.state = if done {
                        State::Flushing
                    } else {
                        State::Writing
                    };

                    let output_len = output.written().len();
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Flushing => {
                    if this.output.len() < OUTPUT_BUFFER_SIZE {
                        this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    }

                    let mut output = PartialBuffer::new(this.output.as_mut());

                    let done = this.decoder.finish(&mut output)?;

                    *this.state = if done { State::Done } else { State::Reading };

                    let output_len = output.written().len();
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("Decoder reached invalid state"),
            };
        }
    }
}
