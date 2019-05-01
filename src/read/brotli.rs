use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use brotli2::raw::{CoStatus, CompressOp};
pub use brotli2::{raw::Compress, CompressParams};
use futures::{
    io::{AsyncBufRead, AsyncRead},
    ready,
};

pub struct BrotliRead<R: AsyncBufRead> {
    inner: R,
    flushing: bool,
    compress: Compress,
}

impl<R: AsyncBufRead> AsyncRead for BrotliRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        unimplemented!()
    }
}

impl<R: AsyncBufRead> BrotliRead<R> {
    pub fn new(read: R, compress: Compress) -> BrotliRead<R> {
        BrotliRead {
            inner: read,
            flushing: false,
            compress,
        }
    }
}
