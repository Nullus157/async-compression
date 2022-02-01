use crate::{codec::Decode, util::PartialBuffer};
use std::{
    fmt,
    io::{Error, ErrorKind, Result},
};

use brotli::{enc::StandardAlloc, BrotliDecompressStream, BrotliResult, BrotliState};

pub struct BrotliDecoder {
    state: BrotliState<StandardAlloc, StandardAlloc, StandardAlloc>,
}

impl BrotliDecoder {
    pub(crate) fn new() -> Self {
        Self {
            state: BrotliState::new(
                StandardAlloc::default(),
                StandardAlloc::default(),
                StandardAlloc::default(),
            ),
        }
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<BrotliResult> {
        let in_buf = input.unwritten();
        let mut out_buf = output.unwritten_mut();

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
            BrotliResult::ResultFailure => {
                return Err(Error::new(ErrorKind::Other, "brotli error"))
            }
            status => status,
        };

        input.advance(input_len);
        output.advance(output_len);

        Ok(status)
    }
}

impl Decode for BrotliDecoder {
    fn reinit(&mut self) -> Result<()> {
        self.state = BrotliState::new(
            StandardAlloc::default(),
            StandardAlloc::default(),
            StandardAlloc::default(),
        );
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        match self.decode(input, output)? {
            BrotliResult::ResultSuccess => Ok(true),
            BrotliResult::NeedsMoreOutput | BrotliResult::NeedsMoreInput => Ok(false),
            BrotliResult::ResultFailure => unreachable!(),
        }
    }

    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        match self.decode(&mut PartialBuffer::new(&[][..]), output)? {
            BrotliResult::ResultSuccess | BrotliResult::NeedsMoreInput => Ok(true),
            BrotliResult::NeedsMoreOutput => Ok(false),
            BrotliResult::ResultFailure => unreachable!(),
        }
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        match self.decode(&mut PartialBuffer::new(&[][..]), output)? {
            BrotliResult::ResultSuccess => Ok(true),
            BrotliResult::NeedsMoreOutput => Ok(false),
            BrotliResult::NeedsMoreInput => Err(Error::new(
                ErrorKind::UnexpectedEof,
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
            .finish()
    }
}
