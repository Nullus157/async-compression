mod decoder;
mod encoder;
pub mod params;

pub use self::{decoder::ZstdDecoder, encoder::ZstdEncoder};

use compression_core::{
    unshared::Unshared,
    util::{PartialBuffer, WriteBuffer},
};
use libzstd::stream::raw::{InBuffer, Operation, OutBuffer, WriteBuf};
use std::io;

#[repr(transparent)]
struct WriteBufferWrapper<'a>(WriteBuffer<'a>);

/// SAFETY: all methods implemented as pass-through safe/unsafe methods on `WriteBuffer`.
unsafe impl WriteBuf for WriteBufferWrapper<'_> {
    fn as_slice(&self) -> &[u8] {
        self.0.written()
    }

    fn capacity(&self) -> usize {
        self.0.capacity()
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }

    unsafe fn filled_until(&mut self, n: usize) {
        self.0.set_written_and_initialized_len(n);
    }
}

trait WriteBufExt {
    fn get_out_buf(&mut self) -> OutBuffer<'_, WriteBufferWrapper<'_>>;
}

impl WriteBufExt for WriteBuffer<'_> {
    fn get_out_buf(&mut self) -> OutBuffer<'_, WriteBufferWrapper<'_>> {
        // Pass written_len to avoid overwriting existing data in buffer.
        let written_len = self.written_len();

        OutBuffer::around_pos(
            // SAFETY: zstd_safe expects partially a initialized input and handles is correctly
            unsafe { &mut *(self as *mut _ as *mut WriteBufferWrapper<'_>) },
            written_len,
        )
    }
}

trait OperationExt {
    fn reinit(&mut self) -> io::Result<()>;

    /// Return `true` if finished.
    fn run(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool>;

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool>;

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool>;
}

impl<C: Operation> OperationExt for Unshared<C> {
    fn reinit(&mut self) -> io::Result<()> {
        self.get_mut().reinit()
    }

    fn run(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        let mut in_buf = InBuffer::around(input.unwritten());
        let result = self.get_mut().run(&mut in_buf, &mut output.get_out_buf());
        input.advance(in_buf.pos());
        Ok(result? == 0)
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        Ok(self.get_mut().flush(&mut output.get_out_buf())? == 0)
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        Ok(self.get_mut().finish(&mut output.get_out_buf(), true)? == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static_assertions::assert_eq_size!(WriteBuffer<'static>, WriteBufferWrapper<'static>);
    static_assertions::assert_eq_align!(WriteBuffer<'static>, WriteBufferWrapper<'static>);
}
