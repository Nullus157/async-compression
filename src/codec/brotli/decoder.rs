use crate::codec::Decode;
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

    fn do_decode(
        &mut self,
        mut input: &[u8],
        mut output: &mut [u8],
    ) -> Result<(DeStatus, usize, usize)> {
        let input_len = input.len();
        let output_len = output.len();

        let status = self.decompress.decompress(&mut input, &mut output)?;

        Ok((status, input_len - input.len(), output_len - output.len()))
    }
}

impl Decode for BrotliDecoder {
    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        let (status, in_length, out_length) = self.do_decode(input, output)?;

        match status {
            DeStatus::NeedOutput | DeStatus::NeedInput => Ok((false, in_length, out_length)),
            DeStatus::Finished => Ok((true, in_length, out_length)),
        }
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (status, _, out_length) = self.do_decode(&[], output)?;

        match status {
            DeStatus::Finished => Ok((true, out_length)),
            DeStatus::NeedOutput => Ok((false, out_length)),
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
