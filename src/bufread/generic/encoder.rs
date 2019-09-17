use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{codec::Encode, util::PartialBuffer};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    Encoding,
    Flushing,
    Done,
}

#[unsafe_project(Unpin)]
#[derive(Debug)]
pub struct Encoder<R: AsyncBufRead, E: Encode> {
    #[pin]
    reader: R,
    encoder: E,
    state: State,
}

impl<R: AsyncBufRead, E: Encode> Encoder<R, E> {
    pub fn new(reader: R, encoder: E) -> Self {
        Self {
            reader,
            encoder,
            state: State::Encoding,
        }
    }

    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut R> {
        self.project().reader
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    fn do_poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Poll<Result<()>> {
        let mut this = self.project();

        loop {
            *this.state = match this.state {
                State::Encoding => {
                    let input = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
                    if input.is_empty() {
                        State::Flushing
                    } else {
                        let (input_len, output_len) =
                            this.encoder.encode(input, output.unwritten_mut())?;
                        this.reader.as_mut().consume(input_len);
                        output.advance(output_len);
                        State::Encoding
                    }
                }

                State::Flushing => {
                    let (done, output_len) = this.encoder.finish(output.unwritten_mut())?;
                    output.advance(output_len);

                    if done {
                        State::Done
                    } else {
                        State::Flushing
                    }
                }

                State::Done => State::Done,
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

impl<R: AsyncBufRead, E: Encode> AsyncRead for Encoder<R, E> {
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
