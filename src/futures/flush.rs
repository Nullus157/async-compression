use std::{time::Duration, time::Instant};

use futures_core::Future;
use futures_core::Stream;
use futures_io::AsyncRead;
use pin_project_lite::pin_project;

use super::bufread::Encoder;
use crate::codec::Encode;

pub trait AsyncFlush {
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>>;
}

pin_project! {
    pub struct FlushableEncoder<E: AsyncRead> {
        #[pin]
        encoder: E,
        #[pin]
        receiver: futures_channel::mpsc::Receiver<()>,
    }
}

impl<E: AsyncRead + AsyncFlush> FlushableEncoder<E> {
    pub fn new(encoder: E, receiver: futures_channel::mpsc::Receiver<()>) -> Self {
        Self { encoder, receiver }
    }
}

impl<E: AsyncRead + AsyncFlush> AsyncRead for FlushableEncoder<E> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut this = self.project();

        match this.encoder.as_mut().poll_read(cx, buf) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r),
            std::task::Poll::Pending => match this.receiver.as_mut().poll_next(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(_) => match this.encoder.poll_flush(cx, buf) {
                    std::task::Poll::Ready(Ok(sz)) => std::task::Poll::Ready(Ok(sz)),
                    std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                },
            },
        }
    }
}
