use futures_core::ready;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

make_compat! {
    futures_io_03::AsyncWrite {
        poll_close
    }
}

impl<W: crate::AsyncWrite + futures_io_03::AsyncSeek> futures_io_03::AsyncSeek
    for crate::AsyncBufWriter<W>
{
    /// Seek to the offset, in bytes, in the underlying writer.
    ///
    /// Seeking always writes out the internal buffer before seeking.
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: futures_io_03::SeekFrom,
    ) -> Poll<io::Result<u64>> {
        ready!(self.as_mut().flush_buf(cx))?;
        self.project().inner.poll_seek(cx, pos)
    }
}
