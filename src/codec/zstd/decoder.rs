use std::io::{Result, Error, ErrorKind};

use crate::codec::Decode;
use zstd_rs::frame_decoder::FrameDecoder;

pub struct ZstdDecoder {
    decoder: FrameDecoder,
    flushing: Option<Vec<u8>>,
}

impl ZstdDecoder {
    pub(crate) fn new() -> Self {
        Self {
            decoder: FrameDecoder::new(),
            flushing: None,
        }
    }
}

impl Decode for ZstdDecoder {
    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let (bytes_read, bytes_written) = self.decoder.decode_from_to(input, output).map_err(|e| Error::new(ErrorKind::Other, e))?;
        Ok((self.decoder.is_finished(), bytes_read, bytes_written))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        if self.flushing.is_none() {
            self.flushing = Some(self.decoder.drain_buffer());
        }

        if let Some(flushing) = &mut self.flushing {
            let len = std::cmp::min(flushing.len(), output.len());
            output[..len].copy_from_slice(&flushing[..len]);
            flushing.drain(..len);
            Ok((flushing.is_empty(), len))
        } else {
            unreachable!()
        }
    }
}

impl core::fmt::Debug for ZstdDecoder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ZstdDecoder")
            .field("decoder", &"[no debug]")
            .finish()
    }
}
