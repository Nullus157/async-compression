use crate::util::PartialBuffer;
use std::io::Result;

#[derive(Debug)]
pub struct ZlibDecoder {
    inner: crate::codec::FlateDecoder,
}

impl ZlibDecoder {
    pub(crate) fn new() -> Self {
        Self {
            inner: crate::codec::FlateDecoder::new(true),
        }
    }
}

impl crate::codec::Decode for ZlibDecoder {
    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool> {
        self.inner.decode(input, output)
    }

    fn flush(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        self.inner.flush(output)
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        self.inner.finish(output)
    }
}
