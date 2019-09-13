use std::io::{Error, ErrorKind, Result};

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
    fn parse_header(&mut self, _input: &[u8]) -> Option<Result<usize>> {
        Some(Ok(0))
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let status = self.decoder.get_mut().run_on_buffers(input, output)?;
        Ok((
            status.bytes_read == input.len(),
            status.bytes_read,
            status.bytes_written,
        ))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let mut output = zstd_safe::OutBuffer::around(output);

        let bytes_left = self.decoder.get_mut().flush(&mut output)?;
        Ok((bytes_left == 0, output.as_slice().len()))
    }

    fn check_footer(&mut self, input: &[u8]) -> Result<()> {
        if input.is_empty() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "extra data after end of compressed block",
            ))
        }
    }
}
