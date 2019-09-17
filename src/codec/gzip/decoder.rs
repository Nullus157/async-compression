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
    const HEADER_LENGTH: usize = 10;
    const FOOTER_LENGTH: usize = 8;

    fn parse_header(&mut self, input: &[u8]) -> Result<()> {
        if input.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Invalid gzip header length",
            ));
        }

        if input[0..3] != [0x1f, 0x8b, 0x08] {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid gzip header"));
        }

        // TODO: Check that header doesn't contain any extra headers
        Ok(())
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        let (done, in_length, out_length) = self.inner.decode(input, output)?;
        self.crc.update(&output[..out_length]);
        Ok((done, in_length, out_length))
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (done, out_length) = self.inner.finish(output)?;
        self.crc.update(&output[..out_length]);
        Ok((done, out_length))
    }

    fn check_footer(&mut self, input: &[u8]) -> Result<()> {
        if input.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Invalid gzip footer length",
            ));
        }

        let crc = self.crc.sum().to_le_bytes();
        let bytes_read = self.crc.amount().to_le_bytes();

        if crc != input[0..4] {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "CRC computed does not match",
            ));
        }

        if bytes_read != input[4..8] {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "amount of bytes read does not match",
            ));
        }

        Ok(())
    }
}
