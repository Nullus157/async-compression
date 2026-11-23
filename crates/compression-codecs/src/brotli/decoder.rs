use crate::DecodeV2;
use brotli::{enc::StandardAlloc, BrotliDecompressStream, BrotliResult};
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::{fmt, io};

type BrotliState = brotli::BrotliState<StandardAlloc, StandardAlloc, StandardAlloc>;

pub struct BrotliDecoder {
    // `BrotliState` is very large (over 2kb) which is why we're boxing it.
    state: Box<BrotliState>,
    large_window: bool,
}

impl Default for BrotliDecoder {
    fn default() -> Self {
        Self::with_window(false)
    }
}

impl BrotliDecoder {
    fn with_window(large_window: bool) -> Self {
        Self {
            state: Box::new(Self::new_brotli_state(large_window)),
            large_window,
        }
    }

    /// `BrotliState::new` accepts the non-standard large-window format (RFC 9841), whose header
    /// can declare a window of up to 1 GiB, and the ring buffer is sized from it. A few hundred
    /// bytes of input could then make the decoder allocate ~1 GiB, regardless of how much output
    /// the caller reads. So only accept RFC 7932 windows (at most 16 MiB) unless the caller opts
    /// in, as the `mbrotli` backend does; `BrotliEncoder` never produces large-window streams.
    fn new_brotli_state(large_window: bool) -> BrotliState {
        let new = if large_window {
            BrotliState::new
        } else {
            BrotliState::new_strict
        };
        new(
            StandardAlloc::default(),
            StandardAlloc::default(),
            StandardAlloc::default(),
        )
    }

    /// Creates a decoder that only accepts standard (RFC 7932) brotli streams, whose window is
    /// at most 16 MiB.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a decoder that also accepts the non-standard large-window brotli format
    /// (RFC 9841).
    ///
    /// A large-window stream can declare a window of up to 1 GiB, which the decoder allocates
    /// however few bytes the stream decodes to, so only use this for trusted input. Large windows
    /// are not valid in the HTTP `br` content coding.
    pub fn new_with_large_window() -> Self {
        Self::with_window(true)
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<BrotliResult> {
        let in_buf = input.unwritten();
        let out_buf = output.initialize_unwritten();

        let mut input_len = 0;
        let mut output_len = 0;

        let status = match BrotliDecompressStream(
            &mut in_buf.len(),
            &mut input_len,
            in_buf,
            &mut out_buf.len(),
            &mut output_len,
            out_buf,
            &mut 0,
            &mut self.state,
        ) {
            BrotliResult::ResultFailure => return Err(io::Error::other("brotli error")),
            status => status,
        };

        input.advance(input_len);
        output.advance(output_len);

        Ok(status)
    }
}

impl DecodeV2 for BrotliDecoder {
    fn reinit(&mut self) -> io::Result<()> {
        *self.state = Self::new_brotli_state(self.large_window);
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> io::Result<bool> {
        match self.decode(input, output)? {
            BrotliResult::ResultSuccess => Ok(true),
            BrotliResult::NeedsMoreOutput | BrotliResult::NeedsMoreInput => Ok(false),
            BrotliResult::ResultFailure => unreachable!(),
        }
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        match self.decode(&mut PartialBuffer::new(&[][..]), output)? {
            BrotliResult::ResultSuccess | BrotliResult::NeedsMoreInput => Ok(true),
            BrotliResult::NeedsMoreOutput => Ok(false),
            BrotliResult::ResultFailure => unreachable!(),
        }
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> io::Result<bool> {
        match self.decode(&mut PartialBuffer::new(&[][..]), output)? {
            BrotliResult::ResultSuccess => Ok(true),
            BrotliResult::NeedsMoreOutput => Ok(false),
            BrotliResult::NeedsMoreInput => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "reached unexpected EOF",
            )),
            BrotliResult::ResultFailure => unreachable!(),
        }
    }
}

impl fmt::Debug for BrotliDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrotliDecoder")
            .field("decompress", &"<no debug>")
            .field("large_window", &self.large_window)
            .finish()
    }
}
