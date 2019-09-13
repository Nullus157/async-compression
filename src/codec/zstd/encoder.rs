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
    fn write_header(&mut self, output: &mut [u8]) -> Result<usize> {
        // zstd needs to have 0 bytes written to it to create a valid compressed 0 byte stream,
        // just flushing it after not writing to it is not enough, here seems a decent place to do
        // it.
        let status = self.encoder.get_mut().run_on_buffers(&[], output)?;
        dbg!(status.bytes_read);
        Ok(dbg!(status.bytes_written))
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let status = self.encoder.get_mut().run_on_buffers(input, output)?;
        Ok((
            status.bytes_read == input.len(),
            dbg!(status.bytes_read),
            dbg!(status.bytes_written),
        ))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let mut output = zstd_safe::OutBuffer::around(output);

        let bytes_left = self.encoder.get_mut().flush(&mut output)?;
        if bytes_left == 0 {
            self.encoder.get_mut().finish(&mut output, true)?;
        }
        Ok((bytes_left == 0, dbg!(output.as_slice().len())))
    }

    fn write_footer(&mut self, _output: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}
