use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::bufread::Encoder;
use flate2::Compression;
use futures::io::{AsyncBufRead, AsyncRead};
use pin_project::unsafe_project;

/// A zlib encoder, or compressor.
///
/// This structure implements an [`AsyncRead`] interface and will read uncompressed data from an
/// underlying stream and emit a stream of compressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
pub struct ZlibEncoder<R: AsyncBufRead> {
    #[pin]
    inner: Encoder<R, crate::codec::ZlibEncoder>,
}

impl<R: AsyncBufRead> ZlibEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(read: R, level: Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            inner: Encoder::new(read, crate::codec::ZlibEncoder::new(level)),
        }
    }

    /// Acquires a reference to the underlying reader that this encoder is wrapping.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }

    /// Acquires a mutable reference to the underlying reader that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the reader which may
    /// otherwise confuse this encoder.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying reader that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the reader which may
    /// otherwise confuse this encoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut R> {
        self.project().inner.get_pin_mut()
    }

    /// Consumes this encoder returning the underlying reader.
    ///
    /// Note that this may discard internal state of this encoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}

impl<R: AsyncBufRead> AsyncRead for ZlibEncoder<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.project().inner.poll_read(cx, buf)
    }
}

fn _assert() {
    crate::util::_assert_send::<ZlibEncoder<Pin<Box<dyn AsyncBufRead + Send>>>>();
    crate::util::_assert_sync::<ZlibEncoder<Pin<Box<dyn AsyncBufRead + Sync>>>>();
}
