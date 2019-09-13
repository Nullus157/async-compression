use crate::codec::Decode;
use std::io::{Error, ErrorKind, Result};

use flate2::Crc;

#[derive(Debug)]
pub struct GzipDecoder {
    inner: crate::codec::FlateDecoder,
    crc: Crc,
}

impl GzipDecoder {
    pub(crate) fn new() -> Self {
        Self {
            inner: crate::codec::FlateDecoder::new(false),
            crc: Crc::new(),
        }
    }
}

impl Decode for GzipDecoder {
    fn parse_header(&mut self, input: &[u8]) -> Result<Option<usize>> {
        if input.len() < 10 {
            return Ok(None);
        }

        if input[0..3] != [0x1f, 0x8b, 0x08] {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Invalid gzip header",
            ));
        }

        // TODO: Check that header doesn't contain any extra headers
        Ok(Some(10))
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        let (done, in_length, out_length) = self.inner.decode(input, output)?;
        self.crc.update(&output[..out_length]);
        Ok((done, in_length, out_length))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (done, out_length) = self.inner.flush(output)?;
        self.crc.update(&output[..out_length]);
        Ok((done, out_length))
    }

    fn check_footer(&mut self, input: &[u8]) -> Result<Option<usize>> {
        if input.len() < 8 {
            return Ok(None);
        }

        let crc = self.crc.sum().to_le_bytes();
        let bytes_read = self.crc.amount().to_le_bytes();

        if crc != input[0..4] {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "CRC computed does not match",
            ))
        }

        if bytes_read != input[4..8] {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "amount of bytes read does not match",
            ))
        }

        Ok(Some(8))
    }
}
