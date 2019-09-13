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
    fn header(&mut self) -> Vec<u8> {
        // zstd needs to have 0 bytes written to it to create a valid compressed 0 byte stream,
        // just flushing it after not writing to it is not enough, here seems a decent place to do
        // it.
        let mut output = vec![0; 128];
        let status = self
            .encoder
            .get_mut()
            .run_on_buffers(&[], &mut output)
            .unwrap();
        output.truncate(status.bytes_written);
        output
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let status = self.encoder.get_mut().run_on_buffers(input, output)?;
        Ok((
            status.bytes_read == input.len(),
            status.bytes_read,
            status.bytes_written,
        ))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let mut output = zstd_safe::OutBuffer::around(output);

        let bytes_left = self.encoder.get_mut().flush(&mut output)?;
        if bytes_left == 0 {
            self.encoder.get_mut().finish(&mut output, true)?;
        }
        Ok((bytes_left == 0, output.as_slice().len()))
    }
}
