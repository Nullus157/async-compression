use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::bufread::Decoder;
use futures::io::{AsyncBufRead, AsyncRead};
use pin_project::unsafe_project;

/// A DEFLATE decoder, or uncompressor.
///
/// This structure implements an [`AsyncRead`] interface and will read compressed data from an
/// underlying stream and emit a stream of uncompressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
pub struct DeflateDecoder<R: AsyncBufRead> {
    #[pin]
    inner: Decoder<R, crate::codec::DeflateDecoder>,
}

impl<R: AsyncBufRead> DeflateDecoder<R> {
    /// Creates a new decoder which will read compressed data from the given stream and emit a
    /// uncompressed stream.
    pub fn new(read: R) -> DeflateDecoder<R> {
        DeflateDecoder {
            inner: Decoder::new(read, crate::codec::DeflateDecoder::new()),
        }
    }

    /// Acquires a reference to the underlying reader that this decoder is wrapping.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }

    /// Acquires a mutable reference to the underlying reader that this decoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the reader which may
    /// otherwise confuse this decoder.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying reader that this decoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the reader which may
    /// otherwise confuse this decoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut R> {
        self.project().inner.get_pin_mut()
    }

    /// Consumes this decoder returning the underlying reader.
    ///
    /// Note that this may discard internal state of this decoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}

impl<R: AsyncBufRead> AsyncRead for DeflateDecoder<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.project().inner.poll_read(cx, buf)
    }
}

fn _assert() {
    crate::util::_assert_send::<DeflateDecoder<Pin<Box<dyn AsyncBufRead + Send>>>>();
    crate::util::_assert_sync::<DeflateDecoder<Pin<Box<dyn AsyncBufRead + Sync>>>>();
}
