use crate::{codec::Encode, util::PartialBuffer};
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
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()> {
        self.inner.encode(input, output)
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        self.inner.finish(output)
    }
}
