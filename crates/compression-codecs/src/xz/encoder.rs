use crate::{
    xz2::{Xz2Encoder, Xz2FileFormat},
    Encode, PartialBuffer,
};

use std::io::Result;

#[derive(Debug)]
pub struct XzEncoder {
    inner: Xz2Encoder,
}

impl XzEncoder {
    pub fn new(level: u32) -> Self {
        Self {
            inner: Xz2Encoder::new(Xz2FileFormat::Xz, level),
        }
    }
}

impl Encode for XzEncoder {
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
