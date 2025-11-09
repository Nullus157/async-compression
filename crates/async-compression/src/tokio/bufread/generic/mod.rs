mod decoder;
mod encoder;

pub use self::{decoder::Decoder, encoder::Encoder};

use crate::core::util::WriteBuffer;
use std::{io::Result, task::Poll};
use tokio::io::ReadBuf;

fn poll_read(
    buf: &mut ReadBuf<'_>,
    do_poll_read: impl FnOnce(&mut WriteBuffer<'_>) -> Poll<Result<()>>,
) -> Poll<Result<()>> {
    if buf.remaining() == 0 {
        return Poll::Ready(Ok(()));
    }

    let mut output = WriteBuffer::new_initialized(buf.initialize_unfilled());
    match do_poll_read(&mut output)? {
        Poll::Pending if output.written().is_empty() => Poll::Pending,
        _ => {
            let len = output.written_len();
            buf.advance(len);
            Poll::Ready(Ok(()))
        }
    }
}
