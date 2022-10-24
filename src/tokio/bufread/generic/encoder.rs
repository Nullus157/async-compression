use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{codec::Encode, tokio::flush::AsyncFlush, util::PartialBuffer};
use futures_core::ready;
use pin_project_lite::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};

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
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let mut output = PartialBuffer::new(buf.initialize_unfilled());
        match self.do_poll_read(cx, &mut output)? {
            Poll::Pending if output.written().is_empty() => Poll::Pending,
            _ => {
                let len = output.written().len();
                buf.advance(len);
                Poll::Ready(Ok(()))
            }
        }
    }
}

impl<R: AsyncBufRead, E: Encode> AsyncFlush for Encoder<R, E> {
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<bool>> {
        let mut output = PartialBuffer::new(buf.initialize_unfilled());
        let mut this = self.project();

        match this.encoder.flush(&mut output)? {
            true => {
                let len = output.written().len();
                buf.advance(len);
                Poll::Ready(Ok(true))
            }
            false => Poll::Ready(Ok(false)),
        }
    }
}
