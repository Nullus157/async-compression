use crate::{mbrotli::params::EncoderParams, EncodeV2};
use compression_core::util::{PartialBuffer, WriteBuffer};
use mbrotli::{Compressor, EncoderSessionOwned, EncoderStatus, Operation};
use std::io;

#[derive(Debug)]
pub struct MbrotliEncoder {
    session: EncoderSessionOwned,
}

impl MbrotliEncoder {
    pub fn new(params: EncoderParams) -> Self {
        // `EncoderParams` only builds standard windows, the one combination mbrotli rejects at
        // configuration time is a large window at a low quality.
        let compressor = Compressor::new(params.config()).unwrap();
        // A fresh compressor with a zero stream offset has no way to refuse a session other than
        // failing to allocate its workspace.
        let session = compressor.into_session(params.stream()).unwrap();
        Self { session }
    }
}

impl EncodeV2 for MbrotliEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<()> {
        let progress = self.session.process(
            input.unwritten(),
            output.initialize_unwritten(),
            Operation::Process,
        )?;

        input.advance(progress.consumed);
        output.advance(progress.produced);

        Ok(())
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        let progress = self.session.flush(output.initialize_unwritten())?;
        output.advance(progress.produced);

        match progress.status {
            EncoderStatus::NeedsInput | EncoderStatus::Finished => Ok(true),
            EncoderStatus::NeedsOutput => Ok(false),
        }
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        let progress = self.session.finish(output.initialize_unwritten())?;
        output.advance(progress.produced);

        match progress.status {
            EncoderStatus::Finished => Ok(true),
            EncoderStatus::NeedsInput | EncoderStatus::NeedsOutput => Ok(false),
        }
    }
}
