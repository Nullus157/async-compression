use crate::codec::Encode;
use std::io::Result;

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
    fn header(&mut self) -> Vec<u8> {
        let level_byte = if self.level.level() >= Compression::best().level() {
            0x02
        } else if self.level.level() <= Compression::fast().level() {
            0x04
        } else {
            0x00
        };

        vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, level_byte, 0xff]
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize)> {
        let (in_length, out_length) = self.inner.encode(input, output)?;
        self.crc.update(&input[..in_length]);
        Ok((in_length, out_length))
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        self.inner.finish(output)
    }

    fn footer(&mut self) -> Vec<u8> {
        let mut output = Vec::with_capacity(8);

        output.extend(&self.crc.sum().to_le_bytes());
        output.extend(&self.crc.amount().to_le_bytes());

        output
    }
}
