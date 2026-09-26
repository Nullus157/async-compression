use crate::DecodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};
use mbrotli::{
    DecodeFailure, DecodeOperation, DecodeProgress, DecodeStreamConfig, DecoderConfig,
    DecoderSessionOwned, DecoderStatus, Decompressor, WindowLimit,
};
use std::io;

#[derive(Debug)]
pub struct MbrotliDecoder {
    session: DecoderSessionOwned,
}

impl Default for MbrotliDecoder {
    fn default() -> Self {
        // Only accept RFC 7932 windows, as the `brotli` backend does. Large windows (RFC 9841) are
        // not valid in the `br` content coding.
        let config = DecoderConfig::default()
            .with_window_limit(WindowLimit::standard(mbrotli::Window::MAX_STANDARD_BITS).unwrap());
        let decompressor = Decompressor::new(config).unwrap();
        // A fresh decompressor without an exact output size has no way to refuse a session.
        let session = decompressor
            .into_session(DecodeStreamConfig::default())
            .unwrap();
        Self { session }
    }
}

impl MbrotliDecoder {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Delivers the output of a call that takes no input, and returns what the session needs next.
fn drain(
    output: &mut WriteBuffer<'_>,
    result: Result<DecodeProgress, DecodeFailure>,
) -> io::Result<DecoderStatus> {
    match result {
        Ok(progress) => {
            output.advance(progress.produced);
            Ok(progress.status)
        }
        Err(failure) => {
            output.advance(failure.produced);
            Err(failure.into_error().into())
        }
    }
}

impl DecodeV2 for MbrotliDecoder {
    fn reinit(&mut self) -> io::Result<()> {
        self.session.reinit(DecodeStreamConfig::default())?;
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        let (consumed, status) = match self.session.process(
            input.unwritten(),
            output.initialize_unwritten(),
            DecodeOperation::Process,
        ) {
            Ok(progress) => (progress.consumed, drain(output, Ok(progress))),
            Err(failure) => (failure.consumed, drain(output, Err(failure))),
        };
        input.advance(consumed);

        match status? {
            DecoderStatus::Finished => Ok(true),
            DecoderStatus::NeedsInput | DecoderStatus::NeedsOutput => Ok(false),
        }
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        let result = self.session.flush(output.initialize_unwritten());

        match drain(output, result)? {
            DecoderStatus::Finished | DecoderStatus::NeedsInput => Ok(true),
            DecoderStatus::NeedsOutput => Ok(false),
        }
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        // Declaring EOF on a member that is still open fails with `UnexpectedEof`.
        let result = self.session.finish(output.initialize_unwritten());

        match drain(output, result)? {
            DecoderStatus::Finished => Ok(true),
            DecoderStatus::NeedsInput | DecoderStatus::NeedsOutput => Ok(false),
        }
    }
}
