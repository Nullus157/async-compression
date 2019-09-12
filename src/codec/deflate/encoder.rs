use crate::codec::Encoder;
use std::io::Result;

use flate2::Compression;

#[derive(Debug)]
pub struct DeflateEncoder {
    inner: crate::codec::FlateEncoder,
}

impl DeflateEncoder {
    pub(crate) fn new(level: Compression) -> Self {
        Self {
            inner: crate::codec::FlateEncoder::new(level, false),
        }
    }
}

impl Encoder for DeflateEncoder {
    fn write_header(&mut self, output: &mut [u8]) -> Result<usize> {
        self.inner.write_header(output)
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        self.inner.encode(input, output)
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        self.inner.flush(output)
    }

    fn write_footer(&mut self, output: &mut [u8]) -> Result<usize> {
        self.inner.write_footer(output)
    }
}
