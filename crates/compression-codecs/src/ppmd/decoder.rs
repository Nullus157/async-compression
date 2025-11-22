use crate::{DecodeV2, DecodedSize};
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::io;

use super::params::PpmdDecoderParams;

#[derive(Default)]
pub struct PpmdDecoder {
    order: u32,
    memory_size: u32,
}

impl PpmdDecoder {
    pub fn with_params(params: PpmdDecoderParams) -> Self {
        Self {
            order: params.order,
            memory_size: params.memory_size,
        }
    }

    pub fn default_params() -> PpmdDecoderParams {
        PpmdDecoderParams {
            order: 8,
            memory_size: 4 << 20,
        }
    }

    pub fn new() -> Self {
        Self::with_params(Self::default_params())
    }
}

impl std::fmt::Debug for PpmdDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PpmdDecoder")
            .field("order", &self.order)
            .field("memory_size", &self.memory_size)
            .finish()
    }
}

impl DecodeV2 for PpmdDecoder {
    fn reinit(&mut self) -> io::Result<()> {
        todo!()
    }

    fn decode(
        &mut self,
        _input: &mut PartialBuffer<&[u8]>,
        _output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        todo!()
    }

    fn flush(&mut self, _output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        todo!()
    }

    fn finish(&mut self, _output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        todo!()
    }
}

impl DecodedSize for PpmdDecoder {
    fn decoded_size(_input: &[u8]) -> io::Result<u64> {
        todo!()
    }
}
