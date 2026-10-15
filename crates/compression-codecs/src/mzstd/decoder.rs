use crate::{
    mzstd::params::{decoder_config, DParameter},
    DecodeV2,
};
use compression_core::util::{PartialBuffer, WriteBuffer};
use mzstd::{
    dictionary::{DecodeDictionary, DictionaryContent},
    DecodeOperation, DecoderSessionOwned, DecoderStatus, Decompressor,
};
use std::io;

#[derive(Debug)]
pub struct MzstdDecoder {
    session: DecoderSessionOwned,
}

impl Default for MzstdDecoder {
    fn default() -> Self {
        Self::new_with_params(&[])
    }
}

impl MzstdDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// # Panics
    ///
    /// Panics if a parameter is out of range, as the `zstd` backend does.
    pub fn new_with_params(params: &[DParameter]) -> Self {
        let session = Decompressor::new(decoder_config(params)).into_session();
        Self { session }
    }

    pub fn new_with_dict(dictionary: &[u8]) -> io::Result<Self> {
        let dictionary = DecodeDictionary::new(dictionary, DictionaryContent::Auto)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let session =
            Decompressor::new(decoder_config(&[])).into_session_with_dictionary(dictionary);
        Ok(Self { session })
    }
}

/// Feeds `input` to the session, writing into the unwritten tail of `output`.
///
/// Returns how much input was consumed and what the session needs next.
fn process(
    session: &mut DecoderSessionOwned,
    input: &[u8],
    output: &mut WriteBuffer<'_>,
    operation: DecodeOperation,
) -> (usize, io::Result<DecoderStatus>) {
    // Both a successful and a failed call report how much they consumed and produced, so progress
    // made before an error is kept.
    match session.process(input, output.initialize_unwritten(), operation) {
        Ok(progress) => {
            output.advance(progress.produced);
            (progress.consumed, Ok(progress.status))
        }
        Err(failure) => {
            output.advance(failure.produced);
            (failure.consumed, Err(failure.into_error().into()))
        }
    }
}

impl DecodeV2 for MzstdDecoder {
    fn reinit(&mut self) -> io::Result<()> {
        self.session.reinit();
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        let (consumed, status) = process(
            &mut self.session,
            input.unwritten(),
            output,
            DecodeOperation::Process,
        );
        input.advance(consumed);

        match status? {
            DecoderStatus::Finished => Ok(true),
            DecoderStatus::NeedsInput | DecoderStatus::NeedsOutput => Ok(false),
        }
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        // Empty input with `Process` delivers pending output without declaring EOF.
        let (_, status) = process(&mut self.session, &[], output, DecodeOperation::Process);

        match status? {
            DecoderStatus::Finished | DecoderStatus::NeedsInput => Ok(true),
            DecoderStatus::NeedsOutput => Ok(false),
        }
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        // Declaring EOF inside a frame fails with `UnexpectedEof`.
        let (_, status) = process(&mut self.session, &[], output, DecodeOperation::Finish);

        match status? {
            DecoderStatus::Finished => Ok(true),
            DecoderStatus::NeedsInput | DecoderStatus::NeedsOutput => Ok(false),
        }
    }
}
