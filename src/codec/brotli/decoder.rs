use crate::codec::Decoder;
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

impl Decoder for BrotliDecoder {
    fn parse_header(&mut self, _input: &[u8]) -> Option<Result<usize>> {
        Some(Ok(0))
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let (status, in_length, out_length) = self.do_decode(input, output)?;

        match status {
            DeStatus::NeedOutput => Ok((false, in_length, out_length)),
            DeStatus::Finished | DeStatus::NeedInput => Ok((true, in_length, out_length)),
        }
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
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

    fn check_footer(&mut self, input: &[u8]) -> Result<()> {
        if input.is_empty() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "extra data after end of compressed block",
            ))
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
