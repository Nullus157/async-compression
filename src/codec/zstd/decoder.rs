use std::io::Result;

use crate::{codec::Decode, unshared::Unshared};
use libzstd::stream::raw::{Decoder, Operation};

#[derive(Debug)]
pub struct ZstdDecoder {
    decoder: Unshared<Decoder>,
}

impl ZstdDecoder {
    pub(crate) fn new() -> Self {
        Self {
            decoder: Unshared::new(Decoder::new().unwrap()),
        }
    }
}

impl Decode for ZstdDecoder {
    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        let status = self.decoder.get_mut().run_on_buffers(input, output)?;
        Ok((false, status.bytes_read, status.bytes_written))
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let mut output = zstd_safe::OutBuffer::around(output);

        let bytes_left = self.decoder.get_mut().finish(&mut output, true)?;
        Ok((bytes_left == 0, output.as_slice().len()))
    }
}
