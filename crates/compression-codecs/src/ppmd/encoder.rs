use crate::EncodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::io::{self, Write};

use super::params::PpmdEncoderParams;

#[derive(Debug)]
pub struct PpmdEncoder {
    order: u32,
    memory_size: u32,
    end_marker: bool,
    input: Vec<u8>,
    output: Option<PartialBuffer<Vec<u8>>>,
}

impl PpmdEncoder {
    pub fn from_params(params: PpmdEncoderParams) -> Self {
        Self {
            order: params.order,
            memory_size: params.memory_size,
            end_marker: params.end_marker,
            input: Vec::new(),
            output: None,
        }
    }
}

impl EncodeV2 for PpmdEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        _output: &mut WriteBuffer<'_>,
    ) -> io::Result<()> {
        let src = input.unwritten();
        self.input.extend_from_slice(src);
        input.advance(src.len());
        Ok(())
    }

    fn flush(&mut self, _output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        Ok(true)
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        if self.output.is_none() {
            let mut encoded = Vec::new();
            {
                let mut encoder =
                    ppmd_rust::Ppmd7Encoder::new(&mut encoded, self.order, self.memory_size)
                        .map_err(|e| io::Error::other(e.to_string()))?;
                encoder.write_all(&self.input)?;
                let _ = encoder.finish(self.end_marker)?;
            }
            self.output = Some(PartialBuffer::from(encoded));
        }

        let buf = self.output.as_mut().unwrap();
        let _ = output.copy_unwritten_from(buf);
        Ok(buf.unwritten().is_empty())
    }
}
