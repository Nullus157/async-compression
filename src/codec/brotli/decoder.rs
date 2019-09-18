use crate::{codec::Decode, util::PartialBuffer};
use std::{
    fmt,
    io::{Error, ErrorKind, Result},
};

use brotli2::raw::{DeStatus, Decompress};

pub struct BrotliDecoder {
    decompress: Decompress,
}

impl BrotliDecoder {
    pub(crate) fn new() -> Self {
        Self {
            decompress: Decompress::new(),
        }
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<DeStatus> {
        let mut in_buf = input.unwritten();
        let mut out_buf = output.unwritten_mut();

        let original_input_len = in_buf.len();
        let original_output_len = out_buf.len();

        let status = self.decompress.decompress(&mut in_buf, &mut out_buf)?;

        let input_len = original_input_len - in_buf.len();
        let output_len = original_output_len - out_buf.len();

        input.advance(input_len);
        output.advance(output_len);

        Ok(status)
    }
}

impl Decode for BrotliDecoder {
    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool> {
        match self.decode(input, output)? {
            DeStatus::Finished => Ok(true),
            DeStatus::NeedOutput | DeStatus::NeedInput => Ok(false),
        }
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        match self.decode(&mut PartialBuffer::new(&[][..]), output)? {
            DeStatus::Finished => Ok(true),
            DeStatus::NeedOutput => Ok(false),
            DeStatus::NeedInput => Err(Error::new(
                ErrorKind::UnexpectedEof,
                "reached unexpected EOF",
            )),
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
