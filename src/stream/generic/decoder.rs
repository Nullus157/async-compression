use std::{
    io::{Error, ErrorKind, Result},
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::codec::Decode;
use bytes::{Bytes, BytesMut};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

const OUTPUT_BUFFER_SIZE: usize = 8_000;

#[derive(Debug)]
enum State {
    ReadingHeader,
    Reading,
    Writing,
    Flushing,
    CheckingFooter,
    Done,
    Invalid,
}

#[unsafe_project(Unpin)]
#[derive(Debug)]
pub struct Decoder<S: Stream<Item = Result<Bytes>>, D: Decode> {
    #[pin]
    stream: S,
    decoder: D,
    state: State,
    input: Bytes,
    output: BytesMut,
}

impl<S: Stream<Item = Result<Bytes>>, D: Decode> Decoder<S, D> {
    pub fn new(stream: S, decoder: D) -> Self {
        Self {
            stream,
            decoder,
            state: State::ReadingHeader,
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

    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
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
                State::ReadingHeader => {
                    *this.state = State::ReadingHeader;
                    *this.state = match ready!(this.stream.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            this.input.extend_from_slice(&chunk?);
                            if this.input.len() >= D::HEADER_LENGTH {
                                this.decoder.parse_header(&this.input)?;
                                this.input.advance(D::HEADER_LENGTH);
                                State::Writing
                            } else {
                                State::ReadingHeader
                            }
                        }
                        None => {
                            return Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "A valid header was not found",
                            ))));
                        }
                    };
                    continue;
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
                    if this.output.len() < OUTPUT_BUFFER_SIZE {
                        this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    }

                    let (done, input_len, output_len) =
                        this.decoder.decode(&this.input, &mut this.output)?;

                    *this.state = if done { State::Reading } else { State::Writing };
                    this.input.advance(input_len);
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Flushing => {
                    if this.output.len() < OUTPUT_BUFFER_SIZE {
                        this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    }

                    let (done, output_len) = this.decoder.flush(&mut this.output)?;

                    *this.state = if done {
                        State::CheckingFooter
                    } else {
                        State::Reading
                    };
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::CheckingFooter => {
                    if this.input.len() >= D::FOOTER_LENGTH {
                        this.decoder.check_footer(&this.input)?;
                        this.input.advance(D::FOOTER_LENGTH);
                        *this.state = State::Done;
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(Err(Error::new(
                            ErrorKind::UnexpectedEof,
                            "could not read footer",
                        ))))
                    }
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("Decoder reached invalid state"),
            };
        }
    }
}
