use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;
use std::marker::Unpin;

use super::flate::{CompressedStream, DecompressedStream};
use bytes::Bytes;
use flate2::{Compress, Decompress};
pub use flate2::Compression;
use futures::{stream::Stream, stream::StreamExt};

pub struct ZlibStream<S: Stream<Item = Result<Bytes>> + Unpin> {
    inner: CompressedStream<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for ZlibStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> ZlibStream<S> {
    pub fn new(stream: S, level: Compression) -> ZlibStream<S> {
        ZlibStream {
            inner: CompressedStream::new(stream, Compress::new(level, true)),
        }
    }
}

pub struct DecompressedZlibStream<S: Stream<Item = Result<Bytes>> + Unpin> {
    inner: DecompressedStream<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for DecompressedZlibStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> DecompressedZlibStream<S> {
    pub fn new(stream: S) -> DecompressedZlibStream<S> {
        DecompressedZlibStream {
            inner: DecompressedStream::new(stream, Decompress::new(true)),
        }
    }
}
