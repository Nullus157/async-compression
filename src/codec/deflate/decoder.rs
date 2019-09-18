use crate::util::PartialBuffer;
use std::io::Result;

#[derive(Debug)]
pub struct DeflateDecoder {
    inner: crate::codec::FlateDecoder,
}

impl DeflateDecoder {
    pub(crate) fn new() -> Self {
        Self {
            inner: crate::codec::FlateDecoder::new(false),
        }
    }
}

impl crate::codec::Decode for DeflateDecoder {
    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool> {
        self.inner.decode(input, output)
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        self.inner.finish(output)
    }
}
