use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::codec::Encode;
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    Header,
    Encoding,
    Flushing,
    Footer,
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
            state: State::Header,
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
}

struct PartialBuffer<'a> {
    buffer: &'a mut [u8],
    index: usize,
}

impl<'a> PartialBuffer<'a> {
    fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer, index: 0 }
    }

    fn written(&mut self) -> &mut [u8] {
        &mut self.buffer[..self.index]
    }

    fn unwritten(&mut self) -> &mut [u8] {
        &mut self.buffer[self.index..]
    }

    fn advance(&mut self, amount: usize) {
        self.index += amount;
    }
}

impl<R: AsyncBufRead, E: Encode> AsyncRead for Encoder<R, E> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let mut this = self.project();

        let mut output = PartialBuffer::new(buf);
        loop {
            let (state, done) = match this.state {
                State::Header => {
                    // TODO: This will error if the output buffer is small, need to allow partial
                    // writes of the header.
                    let output_len = this.encoder.write_header(output.unwritten())?;
                    output.advance(output_len);
                    (State::Encoding, false)
                }

                State::Encoding => {
                    let input = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
                    if input.is_empty() {
                        (State::Flushing, false)
                    } else {
                        let (done, input_len, output_len) =
                            this.encoder.encode(input, output.unwritten())?;
                        this.reader.as_mut().consume(input_len);
                        output.advance(output_len);
                        (State::Encoding, done)
                    }
                }

                State::Flushing => {
                    let (done, output_len) = this.encoder.flush(output.unwritten())?;
                    output.advance(output_len);

                    if done {
                        (State::Footer, false)
                    } else {
                        (State::Flushing, true)
                    }
                }

                State::Footer => {
                    // TODO: This will error if the output buffer is small, need to allow partial
                    // writes of the footer.
                    let output_len = this.encoder.write_footer(output.unwritten())?;
                    output.advance(output_len);
                    (State::Done, true)
                }

                State::Done => (State::Done, true),
            };

            *this.state = state;
            if done {
                return Poll::Ready(Ok(output.written().len()));
            }
        }
    }
}
