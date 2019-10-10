use crate::{codec::Encode, util::PartialBuffer};
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

    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
        op: CompressOp,
    ) -> Result<CoStatus> {
        let mut in_buf = input.unwritten();
        let mut out_buf = output.unwritten_mut();

        let original_input_len = in_buf.len();
        let original_output_len = out_buf.len();

        let status = self.compress.compress(op, &mut in_buf, &mut out_buf)?;

        let input_len = original_input_len - in_buf.len();
        let output_len = original_output_len - out_buf.len();

        input.advance(input_len);
        output.advance(output_len);

        Ok(status)
    }
}

impl Encode for BrotliEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()> {
        self.encode(input, output, CompressOp::Process).map(drop)
    }

    fn flush(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        match self.encode(&mut PartialBuffer::new(&[][..]), output, CompressOp::Flush)? {
            CoStatus::Unfinished => Ok(false),
            CoStatus::Finished => Ok(true),
        }
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        match self.encode(&mut PartialBuffer::new(&[][..]), output, CompressOp::Finish)? {
            CoStatus::Unfinished => Ok(false),
            CoStatus::Finished => Ok(true),
        }
    }
}

impl fmt::Debug for BrotliEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrotliEncoder")
            .field("compress", &"<no debug>")
            .finish()
    }
}
