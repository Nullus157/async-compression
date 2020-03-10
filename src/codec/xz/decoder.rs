use crate::{codec::Decode, util::PartialBuffer};

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::Result;

#[derive(Debug)]
pub struct XzDecoder {
    inner: crate::codec::Xz2Decoder,
}

impl XzDecoder {
    pub fn new() -> Self {
        Self {
            inner: crate::codec::Xz2Decoder::new(),
        }
    }
}

impl Decode for XzDecoder {
    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        self.inner.decode(input, output)
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
