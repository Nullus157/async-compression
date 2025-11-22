use crate::EncodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::io;

use super::params::PpmdEncoderParams;

#[derive(Debug)]
pub struct PpmdEncoder {
    order: u32,
    memory_size: u32,
    end_marker: bool,
}

impl PpmdEncoder {
    pub fn from_params(params: PpmdEncoderParams) -> Self {
        Self {
            order: params.order,
            memory_size: params.memory_size,
            end_marker: params.end_marker,
        }
    }
}

impl EncodeV2 for PpmdEncoder {
    fn encode(
        &mut self,
        _input: &mut PartialBuffer<&[u8]>,
        _output: &mut WriteBuffer<'_>,
    ) -> io::Result<()> {
        todo!()
    }

    fn flush(&mut self, _output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        todo!()
    }

    fn finish(&mut self, _output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        todo!()
    }
}
