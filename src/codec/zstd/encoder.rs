use crate::{codec::Encode, unshared::Unshared};
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
    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize)> {
        let status = self.encoder.get_mut().run_on_buffers(input, output)?;
        Ok((status.bytes_read, status.bytes_written))
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let mut output = zstd_safe::OutBuffer::around(output);

        let bytes_left = self.encoder.get_mut().finish(&mut output, true)?;
        Ok((bytes_left == 0, output.as_slice().len()))
    }
}
