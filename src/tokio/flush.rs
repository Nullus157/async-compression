use std::{time::Duration, time::Instant};

use futures_core::Future;
use futures_core::Stream;
use pin_project_lite::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead};

use super::bufread::Encoder;
use crate::codec::Encode;

pin_project! {
    pub struct StreamingEncoder<E: AsyncRead> {
        #[pin]
        encoder: E,
        duration: Duration,
        #[pin]
        timeout: tokio::time::Sleep,
    }
}

impl<E: AsyncRead + AsyncFlush> StreamingEncoder<E> {
    pub fn new(encoder: E, timeout: Duration) -> Self {
        let duration = timeout;
        Self {
            encoder,
            duration,
            timeout: tokio::time::sleep(timeout),
        }
    }
}

impl<E: AsyncRead + AsyncFlush> AsyncRead for StreamingEncoder<E> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut this = self.project();

        match this.encoder.as_mut().poll_read(cx, buf) {
            std::task::Poll::Ready(r) => {
                this.timeout
                    .reset(tokio::time::Instant::now() + *this.duration);
                std::task::Poll::Ready(r)
            }
            std::task::Poll::Pending => match this.timeout.as_mut().poll(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(()) => {
                    this.timeout
                        .reset(tokio::time::Instant::now() + *this.duration);
                    match this.encoder.poll_flush(cx, buf) {
                        std::task::Poll::Ready(Ok(true)) => std::task::Poll::Ready(Ok(())),
                        std::task::Poll::Ready(Ok(false)) => std::task::Poll::Pending,
                        std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                        std::task::Poll::Pending => std::task::Poll::Pending,
                    }
                }
            },
        }
    }
}

pub trait AsyncFlush {
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<bool>>;
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
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut this = self.project();

        match this.encoder.as_mut().poll_read(cx, buf) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r),
            std::task::Poll::Pending => match this.receiver.as_mut().poll_next(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(_) => match this.encoder.poll_flush(cx, buf) {
                    std::task::Poll::Ready(Ok(true)) => std::task::Poll::Ready(Ok(())),
                    std::task::Poll::Ready(Ok(false)) => std::task::Poll::Pending,
                    std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                },
            },
        }
    }
}