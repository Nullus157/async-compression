use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{bufread::generic::PartialBuffer, codec::Decode};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    Header(PartialBuffer<Vec<u8>>),
    Decoding,
    Flushing,
    Footer(PartialBuffer<Vec<u8>>),
    Done,
}

#[unsafe_project(Unpin)]
#[derive(Debug)]
pub struct Decoder<R: AsyncBufRead, E: Decode> {
    #[pin]
    reader: R,
    decoder: E,
    state: State,
}

impl<R: AsyncBufRead, E: Decode> Decoder<R, E> {
    pub fn new(reader: R, decoder: E) -> Self {
        Self {
            reader,
            decoder,
            state: State::Header(vec![0; E::HEADER_LENGTH].into()),
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
                    let len = ready!(this.reader.as_mut().poll_read(cx, header.unwritten()))?;
                    header.advance(len);

                    if header.unwritten().is_empty() {
                        this.decoder.parse_header(header.written())?;
                        (State::Decoding, false)
                    } else {
                        (State::Header(header.take()), true)
                    }
                }

                State::Decoding => {
                    let input = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
                    let (done, input_len, output_len) =
                        this.decoder.decode(input, output.unwritten())?;
                    this.reader.as_mut().consume(input_len);
                    output.advance(output_len);
                    if done {
                        (State::Flushing, false)
                    } else {
                        (State::Decoding, false)
                    }
                }

                State::Flushing => {
                    let (done, output_len) = this.decoder.flush(output.unwritten())?;
                    output.advance(output_len);

                    if done {
                        (State::Footer(vec![0; E::FOOTER_LENGTH].into()), false)
                    } else {
                        (State::Flushing, true)
                    }
                }

                State::Footer(footer) => {
                    let len = ready!(this.reader.as_mut().poll_read(cx, footer.unwritten()))?;
                    footer.advance(len);

                    if footer.unwritten().is_empty() {
                        this.decoder.check_footer(footer.written())?;
                        (State::Done, true)
                    } else {
                        (State::Footer(footer.take()), true)
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

impl<R: AsyncBufRead, E: Decode> AsyncRead for Decoder<R, E> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let mut output = PartialBuffer::new(buf);
        match self.do_poll_read(cx, &mut output)? {
            Poll::Pending if output.written().is_empty() => Poll::Pending,
            _ => Poll::Ready(Ok(output.written().len())),
        }
    }
}
