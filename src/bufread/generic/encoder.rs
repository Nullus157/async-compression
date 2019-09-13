use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{codec::Encode, bufread::generic::PartialBuffer};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    Header(PartialBuffer<Vec<u8>>),
    Encoding,
    Flushing,
    Footer(PartialBuffer<Vec<u8>>),
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
    pub fn new(reader: R, mut encoder: E) -> Self {
        let header = PartialBuffer::new(encoder.header());
        Self {
            reader,
            encoder,
            state: State::Header(header),
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
                State::Header(header) => {
                    let len = std::cmp::min(output.unwritten().len(), header.unwritten().len());
                    output.unwritten()[..len].copy_from_slice(&header.unwritten()[..len]);
                    output.advance(len);
                    header.advance(len);
                    if header.unwritten().is_empty() {
                        (State::Encoding, false)
                    } else {
                        (State::Header(std::mem::replace(header, PartialBuffer::new(Vec::new()))), false)
                    }
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
                        (State::Footer(PartialBuffer::new(this.encoder.footer())), false)
                    } else {
                        (State::Flushing, true)
                    }
                }

                State::Footer(footer) => {
                    let len = std::cmp::min(output.unwritten().len(), footer.unwritten().len());
                    output.unwritten()[..len].copy_from_slice(&footer.unwritten()[..len]);
                    output.advance(len);
                    footer.advance(len);
                    if footer.unwritten().is_empty() {
                        (State::Done, true)
                    } else {
                        (State::Footer(std::mem::replace(footer, PartialBuffer::new(Vec::new()))), false)
                    }
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
