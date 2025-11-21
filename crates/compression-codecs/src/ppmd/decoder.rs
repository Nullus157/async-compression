use crate::{DecodeV2, DecodedSize};
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::io::{self, Read};

use super::params::PpmdDecoderParams;

#[derive(Default)]
pub struct PpmdDecoder {
    order: u32,
    memory_size: u32,
    input: Vec<u8>,
    decoder: Option<ppmd_rust::Ppmd7Decoder<std::io::Cursor<Vec<u8>>>>,
    done: bool,
}

impl PpmdDecoder {
    pub fn with_params(params: PpmdDecoderParams) -> Self {
        Self {
            order: params.order,
            memory_size: params.memory_size,
            input: Vec::new(),
            decoder: None,
            done: false,
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
        self.input.clear();
        self.decoder = None;
        self.done = false;
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        _output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        let src = input.unwritten();
        if !src.is_empty() {
            self.input.extend_from_slice(src);
            input.advance(src.len());
        }
        Ok(false)
    }

    fn flush(&mut self, _output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        Ok(true)
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        if self.done {
            return Ok(true);
        }

        if self.decoder.is_none() {
            let cursor = std::io::Cursor::new(std::mem::take(&mut self.input));
            let decoder = ppmd_rust::Ppmd7Decoder::new(cursor, self.order, self.memory_size)
                .map_err(|e| io::Error::other(e.to_string()))?;
            self.decoder = Some(decoder);
        }

        let mut_slice = output.initialize_unwritten();
        let read_bytes = self.decoder.as_mut().unwrap().read(mut_slice)?;
        output.advance(read_bytes);

        if read_bytes == 0 {
            self.done = true;
        }

        Ok(self.done)
    }
}

impl DecodedSize for PpmdDecoder {
    fn decoded_size(_input: &[u8]) -> io::Result<u64> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "unknown size"))
    }
}
