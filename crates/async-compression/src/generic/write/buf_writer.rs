// Originally sourced from `futures_util::io::buf_writer`, needs to be redefined locally so that
// the `AsyncBufWrite` impl can access its internals, and changed a bit to make it more efficient
// with those methods.

use super::AsyncBufWrite;
use futures_core::ready;
use std::{
    fmt, io,
    pin::Pin,
    task::{Context, Poll},
};

const DEFAULT_BUF_SIZE: usize = 8192;

pub struct BufWriter {
    buf: Box<[u8]>,
    written: usize,
    buffered: usize,
}

impl fmt::Debug for BufWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenericBufWriter")
            .field(
                "buffer",
                &format_args!("{}/{}", self.buffered, self.buf.len()),
            )
            .field("written", &self.written)
            .finish()
    }
}

impl BufWriter {
    /// Creates a new `BufWriter` with a default buffer capacity. The default is currently 8 KB,
    /// but may change in the future.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE)
    }

    /// Creates a new `BufWriter` with the specified buffer capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: vec![0; cap].into(),
            written: 0,
            buffered: 0,
        }
    }

    /// Remove the already written data
    fn reshuffle_and_remove_written(&mut self) {
        self.buf.copy_within(self.written..self.buffered, 0);
        self.buffered -= self.written;
        self.written = 0;
    }

    fn do_flush(
        &mut self,
        poll_write: &mut dyn FnMut(&[u8]) -> Poll<io::Result<usize>>,
    ) -> Poll<io::Result<()>> {
        while self.written < self.buffered {
            let bytes_written = ready!(poll_write(&self.buf[self.written..self.buffered]))?;
            if bytes_written == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write the buffered data",
                )));
            }

            self.written += bytes_written;
        }

        Poll::Ready(Ok(()))
    }

    fn partial_flush_buf(
        &mut self,
        poll_write: &mut dyn FnMut(&[u8]) -> Poll<io::Result<usize>>,
    ) -> Poll<io::Result<()>> {
        let ret = if let Poll::Ready(res) = self.do_flush(poll_write) {
            res
        } else {
            Ok(())
        };

        if self.written > 0 {
            self.reshuffle_and_remove_written();

            Poll::Ready(ret)
        } else if self.buffered == 0 {
            Poll::Ready(ret)
        } else {
            ret?;
            Poll::Pending
        }
    }

    pub fn flush_buf(
        &mut self,
        poll_write: &mut dyn FnMut(&[u8]) -> Poll<io::Result<usize>>,
    ) -> Poll<io::Result<()>> {
        let ret = ready!(self.do_flush(poll_write));
        self.reshuffle_and_remove_written();
        Poll::Ready(ret)
    }

    pub fn poll_write(
        &mut self,
        buf: &[u8],
        poll_write: &mut dyn FnMut(&[u8]) -> Poll<io::Result<usize>>,
    ) -> Poll<io::Result<usize>> {
        if self.buffered + buf.len() > self.buf.len() {
            ready!(self.partial_flush_buf(poll_write))?;
        }

        if buf.len() >= self.buf.len() {
            if self.buffered == 0 {
                poll_write(buf)
            } else {
                // The only way that `partial_flush_buf` would have returned with
                // `this.buffered != 0` is if it were Pending, so our waker was already queued
                Poll::Pending
            }
        } else {
            let len = buf.len().min(self.buf.len() - self.buffered);
            self.buf[self.buffered..self.buffered + len].copy_from_slice(&buf[..len]);
            self.buffered += len;
            Poll::Ready(Ok(len))
        }
    }

    pub fn poll_partial_flush_buf(
        &mut self,
        poll_write: &mut dyn FnMut(&[u8]) -> Poll<io::Result<usize>>,
    ) -> Poll<io::Result<&mut [u8]>> {
        ready!(self.partial_flush_buf(poll_write))?;
        Poll::Ready(Ok(&mut self.buf[self.buffered..]))
    }

    pub fn produce(&mut self, amt: usize) {
        debug_assert!(
            self.buffered + amt <= self.buf.len(),
            "produce called with amt exceeding buffer capacity"
        );
        self.buffered += amt;
    }
}

macro_rules! impl_buf_writer {
    ($poll_close: tt) => {
        use crate::generic::write::{AsyncBufWrite, BufWriter as GenericBufWriter};
        use futures_core::ready;
        use pin_project_lite::pin_project;

        pin_project! {
            #[derive(Debug)]
            pub struct BufWriter<W> {
                #[pin]
                writer: W,
                inner: GenericBufWriter,
            }
        }

        impl<W> BufWriter<W> {
            /// Creates a new `BufWriter` with a default buffer capacity. The default is currently 8 KB,
            /// but may change in the future.
            pub fn new(writer: W) -> Self {
                Self {
                    writer,
                    inner: GenericBufWriter::new(),
                }
            }

            /// Creates a new `BufWriter` with the specified buffer capacity.
            pub fn with_capacity(cap: usize, writer: W) -> Self {
                Self {
                    writer,
                    inner: GenericBufWriter::with_capacity(cap),
                }
            }

            /// Gets a reference to the underlying writer.
            pub fn get_ref(&self) -> &W {
                &self.writer
            }

            /// Gets a mutable reference to the underlying writer.
            ///
            /// It is inadvisable to directly write to the underlying writer.
            pub fn get_mut(&mut self) -> &mut W {
                &mut self.writer
            }

            /// Gets a pinned mutable reference to the underlying writer.
            ///
            /// It is inadvisable to directly write to the underlying writer.
            pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
                self.project().writer
            }

            /// Consumes this `BufWriter`, returning the underlying writer.
            ///
            /// Note that any leftover data in the internal buffer is lost.
            pub fn into_inner(self) -> W {
                self.writer
            }
        }

        fn get_poll_write<'a, 'b, W: AsyncWrite>(
            mut writer: Pin<&'a mut W>,
            cx: &'a mut Context<'b>,
        ) -> impl for<'buf> FnMut(&'buf [u8]) -> Poll<io::Result<usize>> + use<'a, 'b, W> {
            move |buf| writer.as_mut().poll_write(cx, buf)
        }

        impl<W: AsyncWrite> BufWriter<W> {
            fn flush_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                let this = self.project();
                this.inner.flush_buf(&mut get_poll_write(this.writer, cx))
            }
        }

        impl<W: AsyncWrite> AsyncWrite for BufWriter<W> {
            fn poll_write(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<io::Result<usize>> {
                let this = self.project();
                this.inner
                    .poll_write(buf, &mut get_poll_write(this.writer, cx))
            }

            fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                ready!(self.as_mut().flush_buf(cx))?;
                self.project().writer.poll_flush(cx)
            }

            fn $poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                ready!(self.as_mut().flush_buf(cx))?;
                self.project().writer.$poll_close(cx)
            }
        }

        impl<W: AsyncWrite> AsyncBufWrite for BufWriter<W> {
            fn poll_partial_flush_buf(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<io::Result<&mut [u8]>> {
                let this = self.project();
                this.inner
                    .poll_partial_flush_buf(&mut get_poll_write(this.writer, cx))
            }

            fn produce(self: Pin<&mut Self>, amt: usize) {
                self.project().inner.produce(amt)
            }
        }
    };
}
pub(crate) use impl_buf_writer;
