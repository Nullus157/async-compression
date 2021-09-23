use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{codec::Decode, util::PartialBuffer};
use futures_core::ready;
use futures_io::{AsyncBufRead, AsyncRead};
use pin_project_lite::pin_project;

#[derive(Debug)]
enum State {
    Decoding,
    Flushing,
    Done,
    Next,
}

pin_project! {
    #[derive(Debug)]
    pub struct Decoder<R, D: Decode> {
        #[pin]
        reader: R,
        decoder: D,
        state: State,
        multiple_members: bool,
    }
}

impl<R: AsyncBufRead, D: Decode> Decoder<R, D> {
    pub fn new(reader: R, decoder: D) -> Self {
        Self {
            reader,
            decoder,
            state: State::Decoding,
            multiple_members: false,
        }
    }

    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut R> {
        self.project().reader
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    pub fn multiple_members(&mut self, enabled: bool) {
        self.multiple_members = enabled;
    }

    fn do_poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Poll<Result<()>> {
        let mut this = self.project();

        let mut first = true;

        loop {
            *this.state = match this.state {
                State::Decoding => {
                    let input = if first {
                        &[][..]
                    } else {
                        ready!(this.reader.as_mut().poll_fill_buf(cx))?
                    };

                    if input.is_empty() && !first {
                        State::Flushing
                    } else {
                        let mut input = PartialBuffer::new(input);
                        let done = this.decoder.decode(&mut input, output).or_else(|e| {
                            // ignore the first error, occurs when input is empty
                            // but we need to run decode to flush
                            if first {
                                Ok(false)
                            } else {
                                Err(e)
                            }
                        })?;

                        first = false;

                        let len = input.written().len();
                        this.reader.as_mut().consume(len);
                        if done {
                            State::Flushing
                        } else {
                            State::Decoding
                        }
                    }
                }

                State::Flushing => {
                    if this.decoder.finish(output)? {
                        if *this.multiple_members {
                            this.decoder.reinit()?;
                            State::Next
                        } else {
                            State::Done
                        }
                    } else {
                        State::Flushing
                    }
                }

                State::Done => State::Done,

                State::Next => {
                    let input = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
                    if input.is_empty() {
                        State::Done
                    } else {
                        State::Decoding
                    }
                }
            };

            if let State::Done = *this.state {
                return Poll::Ready(Ok(()));
            }
            if output.unwritten().is_empty() {
                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl<R: AsyncBufRead, D: Decode> AsyncRead for Decoder<R, D> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let mut output = PartialBuffer::new(buf);
        match self.do_poll_read(cx, &mut output)? {
            Poll::Pending if output.written().is_empty() => Poll::Pending,
            _ => Poll::Ready(Ok(output.written().len())),
        }
    }
}
