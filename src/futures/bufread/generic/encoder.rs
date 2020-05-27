use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{codec::Encode, util::PartialBuffer};
use futures_core::ready;
use futures_io::{AsyncBufRead, AsyncRead, ReadBuf};
use pin_project_lite::pin_project;

#[derive(Debug)]
enum State {
    Encoding,
    Flushing,
    Done,
}

pin_project! {
    #[derive(Debug)]
    pub struct Encoder<R, E: Encode> {
        #[pin]
        reader: R,
        encoder: E,
        state: State,
    }
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

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut R> {
        self.project().reader
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    fn do_poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        let mut this = self.project();

        loop {
            *this.state = match this.state {
                State::Encoding => {
                    let input = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
                    if input.is_empty() {
                        State::Flushing
                    } else {
                        let mut input = PartialBuffer::new(input);
                        this.encoder.encode(&mut input, output)?;
                        let len = input.written().len();
                        this.reader.as_mut().consume(len);
                        State::Encoding
                    }
                }

                State::Flushing => {
                    if this.encoder.finish(output)? {
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
            if output.remaining() == 0 {
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
        let mut buf = ReadBuf::new(buf);
        ready!(self.poll_read_buf(cx, &mut buf))?;
        Poll::Ready(Ok(buf.filled().len()))
    }

    fn poll_read_buf(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let prior = buf.filled().len();
        match self.do_poll_read(cx, buf)? {
            Poll::Pending if buf.filled().len() == prior => Poll::Pending,
            _ => Poll::Ready(Ok(())),
        }
    }
}
