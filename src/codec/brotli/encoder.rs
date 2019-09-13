use crate::codec::Encode;
use std::{fmt, io::Result};

use brotli2::{
    raw::{CoStatus, Compress, CompressOp},
    CompressParams,
};

pub struct BrotliEncoder {
    compress: Compress,
}

impl BrotliEncoder {
    pub(crate) fn new(params: &CompressParams) -> Self {
        let mut compress = Compress::new();
        compress.set_params(params);
        Self { compress }
    }

    fn do_encode(
        &mut self,
        mut input: &[u8],
        mut output: &mut [u8],
        op: CompressOp,
    ) -> Result<(CoStatus, usize, usize)> {
        let input_len = input.len();
        let output_len = output.len();

        let status = self.compress.compress(op, &mut input, &mut output)?;

        Ok((status, input_len - input.len(), output_len - output.len()))
    }
}

impl Encode for BrotliEncoder {
    fn write_header(&mut self, _output: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let (status, in_length, out_length) = self.do_encode(input, output, CompressOp::Process)?;

        match status {
            CoStatus::Unfinished => Ok((false, in_length, out_length)),
            CoStatus::Finished => Ok((true, in_length, out_length)),
        }
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (status, _, out_length) = self.do_encode(&[], output, CompressOp::Finish)?;

        match status {
            CoStatus::Unfinished => Ok((false, out_length)),
            CoStatus::Finished => Ok((true, out_length)),
        }
    }

    fn write_footer(&mut self, _output: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

impl fmt::Debug for BrotliEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrotliEncoder")
            .field("compress", &"<no debug>")
            .finish()
    }
}
