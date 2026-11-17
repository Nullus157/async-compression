use crate::{
    zrip::params::{max_window, DParameter},
    DecodeV2,
};
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::io;
use zrip::push::{PushDecoder, Status};

#[derive(Debug)]
pub struct ZripDecoder {
    decoder: PushDecoder,
    /// A frame ended and no later one has started.
    stream_ended: bool,
}

impl Default for ZripDecoder {
    fn default() -> Self {
        Self::new_with_params(&[])
    }
}

impl ZripDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_params(params: &[DParameter]) -> Self {
        let mut decoder = PushDecoder::new();
        if let Some(max) = max_window(params) {
            decoder = decoder.max_window(max);
        }
        Self {
            decoder,
            stream_ended: false,
        }
    }

    pub fn new_with_dict(dictionary: &[u8]) -> io::Result<Self> {
        let dict = zrip::Dictionary::from_bytes(dictionary)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        Ok(Self {
            decoder: PushDecoder::with_dict(dict),
            stream_ended: false,
        })
    }

    fn process(
        &mut self,
        input: &[u8],
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<(usize, Status)> {
        let progress = self
            .decoder
            .decode(input, output.initialize_unwritten())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        output.advance(progress.produced);
        if progress.consumed > 0 {
            self.stream_ended = false;
        }
        if progress.status == Status::Finished {
            self.stream_ended = true;
        }
        Ok((progress.consumed, progress.status))
    }
}

impl DecodeV2 for ZripDecoder {
    fn reinit(&mut self) -> io::Result<()> {
        self.decoder.reset();
        self.stream_ended = false;
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        let (consumed, status) = self.process(input.unwritten(), output)?;
        input.advance(consumed);
        Ok(status == Status::Finished)
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        let (_, status) = self.process(&[], output)?;
        Ok(status != Status::NeedsOutput)
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        let (_, status) = self.process(&[], output)?;
        if status == Status::NeedsOutput {
            return Ok(false);
        }
        // Like the `zstd` backend: the stream must end with a complete frame.
        if self.stream_ended && self.decoder.finish().is_ok() {
            Ok(true)
        } else {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "zstd stream did not finish",
            ))
        }
    }
}
