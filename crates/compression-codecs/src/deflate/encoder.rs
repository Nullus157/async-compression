use crate::{flate::FlateEncoder, Encode, PartialBuffer};
use std::io::Result;

use flate2::Compression;

#[derive(Debug)]
pub struct DeflateEncoder {
    inner: FlateEncoder,
}

impl DeflateEncoder {
    pub fn new(level: Compression) -> Self {
        Self {
            inner: FlateEncoder::new(level, false),
        }
    }
}

impl Encode for DeflateEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<()> {
        self.inner.encode(input, output)
    }

    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        self.inner.flush(output)
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        self.inner.finish(output)
    }
}
