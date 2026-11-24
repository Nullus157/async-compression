use crate::DecodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};
use mbrotli::{
    DecodeOperation, DecodeStreamConfig, DecoderConfig, DecoderSessionOwned, DecoderStatus,
    Decompressor, WindowLimit,
};
use std::io;

/// Only accept RFC 7932 windows by default, as the `brotli` backend does (via
/// `BrotliState::new_strict`). Large windows (RFC 9841) are not valid in the `br` content coding.
const DECODER_CONFIG: DecoderConfig = DecoderConfig::new().with_window_limit(
    match WindowLimit::standard(mbrotli::Window::MAX_STANDARD_BITS) {
        Ok(limit) => limit,
        Err(_) => panic!("the standard window limit is valid"),
    },
);

/// With large windows enabled, accept up to 2^30 bytes, the most the `brotli` backend accepts.
const LARGE_WINDOW_DECODER_CONFIG: DecoderConfig =
    DecoderConfig::new().with_window_limit(match WindowLimit::large(30) {
        Ok(limit) => limit,
        Err(_) => panic!("the large window limit is valid"),
    });

#[derive(Debug)]
pub struct MbrotliDecoder {
    session: DecoderSessionOwned,
}

impl Default for MbrotliDecoder {
    fn default() -> Self {
        Self::with_config(DECODER_CONFIG)
    }
}

impl MbrotliDecoder {
    fn with_config(config: DecoderConfig) -> Self {
        let decompressor = Decompressor::new(config).unwrap();
        // A fresh decompressor without an exact output size has no way to refuse a session.
        let session = decompressor
            .into_session(DecodeStreamConfig::default())
            .unwrap();
        Self { session }
    }

    /// Creates a decoder that only accepts standard (RFC 7932) brotli streams, whose window is
    /// at most 16 MiB.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a decoder that also accepts the non-standard large-window brotli format
    /// (RFC 9841), with windows of up to 1 GiB.
    ///
    /// A large-window stream chooses its own window, so only use this for trusted input. Large
    /// windows are not valid in the HTTP `br` content coding.
    pub fn new_with_large_window() -> Self {
        Self::with_config(LARGE_WINDOW_DECODER_CONFIG)
    }
}

/// Feeds `input` to the session, writing straight into the uninitialized tail of `output`.
///
/// Returns how much input was consumed and what the session needs next.
fn process(
    session: &mut DecoderSessionOwned,
    input: &[u8],
    output: &mut WriteBuffer<'_>,
    operation: DecodeOperation,
) -> (usize, io::Result<DecoderStatus>) {
    // SAFETY: `process_uninit` never de-initializes bytes of `output`.
    let result = session.process_uninit(input, unsafe { output.unwritten_mut() }, operation);
    // `process_uninit` initializes exactly `produced` leading bytes of `output`, both when it
    // succeeds and when it fails.
    match result {
        Ok(progress) => {
            // SAFETY: see above.
            unsafe { output.assume_init_and_advance(progress.produced) };
            (progress.consumed, Ok(progress.status))
        }
        Err(failure) => {
            // SAFETY: see above.
            unsafe { output.assume_init_and_advance(failure.produced) };
            (failure.consumed, Err(failure.into_error().into()))
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
        // Declaring EOF on a member that is still open fails with `UnexpectedEof`.
        let (_, status) = process(&mut self.session, &[], output, DecodeOperation::Finish);

        match status? {
            DecoderStatus::Finished => Ok(true),
            DecoderStatus::NeedsInput | DecoderStatus::NeedsOutput => Ok(false),
        }
    }
}
