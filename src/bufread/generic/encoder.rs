use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{bufread::generic::PartialBuffer, codec::Encode};
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
        let header = encoder.header();
        Self {
            reader,
            encoder,
            state: State::Header(header.into()),
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
            let (state, done) = match this.state {
                State::Header(header) => {
                    let len = std::cmp::min(output.unwritten().len(), header.unwritten().len());
                    output.unwritten()[..len].copy_from_slice(&header.unwritten()[..len]);
                    output.advance(len);
                    header.advance(len);
                    if header.unwritten().is_empty() {
                        (State::Encoding, false)
                    } else {
                        (State::Header(header.take()), false)
                    }
                }

                State::Encoding => {
                    let input = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
                    if input.is_empty() {
                        (State::Flushing, false)
                    } else {
                        let (input_len, output_len) =
                            this.encoder.encode(input, output.unwritten())?;
                        this.reader.as_mut().consume(input_len);
                        output.advance(output_len);
                        (State::Encoding, output.unwritten().is_empty())
                    }
                }

                State::Flushing => {
                    let (done, output_len) = this.encoder.flush(output.unwritten())?;
                    output.advance(output_len);

                    if done {
                        (State::Footer(this.encoder.footer().into()), false)
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
                        (State::Footer(footer.take()), false)
                    }
                }

                State::Done => (State::Done, true),
            };

            *this.state = state;
            if done || output.unwritten().is_empty() {
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
