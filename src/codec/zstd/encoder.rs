use crate::{codec::Encode, unshared::Unshared, util::PartialBuffer};
use libzstd::stream::raw::{Encoder, Operation};
use std::io::Result;

#[derive(Debug)]
pub struct ZstdEncoder {
    encoder: Unshared<Encoder>,
}

impl ZstdEncoder {
    pub(crate) fn new(level: i32) -> Self {
        Self {
            encoder: Unshared::new(Encoder::new(level).unwrap()),
        }
    }
}

impl Encode for ZstdEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()> {
        let status = self
            .encoder
            .get_mut()
            .run_on_buffers(input.unwritten(), output.unwritten_mut())?;
        input.advance(status.bytes_read);
        output.advance(status.bytes_written);
        Ok(())
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        let mut out_buf = zstd_safe::OutBuffer::around(output.unwritten_mut());
        let bytes_left = self.encoder.get_mut().finish(&mut out_buf, true)?;
        let len = out_buf.as_slice().len();
        output.advance(len);
        Ok(bytes_left == 0)
    }
}
