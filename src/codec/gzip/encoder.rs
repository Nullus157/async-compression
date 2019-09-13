use crate::codec::Encode;
use std::io::{Error, ErrorKind, Result};

use flate2::{Compression, Crc};

#[derive(Debug)]
pub struct GzipEncoder {
    inner: crate::codec::FlateEncoder,
    crc: Crc,
    level: Compression,
}

impl GzipEncoder {
    pub(crate) fn new(level: Compression) -> Self {
        Self {
            inner: crate::codec::FlateEncoder::new(level, false),
            crc: Crc::new(),
            level,
        }
    }
}

impl Encode for GzipEncoder {
    fn write_header(&mut self, output: &mut [u8]) -> Result<usize> {
        if output.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "output buffer too short",
            ));
        }

        let level_byte = if self.level.level() >= Compression::best().level() {
            0x02
        } else if self.level.level() <= Compression::fast().level() {
            0x04
        } else {
            0x00
        };

        let header = [0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, level_byte, 0xff];

        output[0..10].copy_from_slice(&header);

        Ok(10)
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        let (done, in_length, out_length) = self.inner.encode(input, output)?;
        self.crc.update(&input[..in_length]);
        Ok((done, in_length, out_length))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        self.inner.flush(output)
    }

    fn write_footer(&mut self, output: &mut [u8]) -> Result<usize> {
        if output.len() < 8 {}

        let crc = self.crc.sum().to_le_bytes();
        let bytes_read = self.crc.amount().to_le_bytes();

        output[0..4].copy_from_slice(&crc);
        output[4..8].copy_from_slice(&bytes_read);

        Ok(8)
    }
}
