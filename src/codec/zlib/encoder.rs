use crate::codec::Encode;
use std::io::Result;

use flate2::Compression;

#[derive(Debug)]
pub struct ZlibEncoder {
    inner: crate::codec::FlateEncoder,
}

impl ZlibEncoder {
    pub(crate) fn new(level: Compression) -> Self {
        Self {
            inner: crate::codec::FlateEncoder::new(level, true),
        }
    }
}

impl Encode for ZlibEncoder {
    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize)> {
        self.inner.encode(input, output)
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        self.inner.finish(output)
    }
}
