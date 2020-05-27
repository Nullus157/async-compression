use crate::{codec::Encode, util::PartialBuffer};
use std::{
    fmt,
    io::{Error, ErrorKind, Result},
    mem::MaybeUninit,
};
use futures_io::ReadBuf;

use brotli::enc::{
    backward_references::BrotliEncoderParams,
    encode::{
        BrotliEncoderCompressStream, BrotliEncoderCreateInstance, BrotliEncoderHasMoreOutput,
        BrotliEncoderIsFinished, BrotliEncoderOperation, BrotliEncoderStateStruct,
    },
    StandardAlloc,
};

pub struct BrotliEncoder {
    state: BrotliEncoderStateStruct<StandardAlloc>,
}

impl BrotliEncoder {
    pub(crate) fn new(params: BrotliEncoderParams) -> Self {
        let mut state = BrotliEncoderCreateInstance(StandardAlloc::default());
        state.params = params;
        Self { state }
    }

    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut ReadBuf<'_>,
        op: BrotliEncoderOperation,
    ) -> Result<()> {
        let in_buf = input.unwritten();
        // Safety: Presumably brotli does not read from this and it's all good
        let mut out_buf = unsafe { MaybeUninit::slice_get_mut(output.unfilled_mut()) };

        let mut input_len = 0;
        let mut output_len = 0;

        if BrotliEncoderCompressStream(
            &mut self.state,
            op,
            &mut in_buf.len(),
            in_buf,
            &mut input_len,
            &mut out_buf.len(),
            &mut out_buf,
            &mut output_len,
            &mut None,
            &mut |_, _, _, _| (),
        ) <= 0
        {
            return Err(Error::new(ErrorKind::Other, "brotli error"));
        }

        input.advance(input_len);
        output.add_filled(output_len);

        Ok(())
    }
}

impl Encode for BrotliEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut ReadBuf<'_>,
    ) -> Result<()> {
        self.encode(
            input,
            output,
            BrotliEncoderOperation::BROTLI_OPERATION_PROCESS,
        )
    }

    fn flush(
        &mut self,
        output: &mut ReadBuf<'_>,
    ) -> Result<bool> {
        self.encode(
            &mut PartialBuffer::new(&[][..]),
            output,
            BrotliEncoderOperation::BROTLI_OPERATION_FLUSH,
        )?;

        Ok(BrotliEncoderHasMoreOutput(&self.state) == 0)
    }

    fn finish(
        &mut self,
        output: &mut ReadBuf<'_>,
    ) -> Result<bool> {
        self.encode(
            &mut PartialBuffer::new(&[][..]),
            output,
            BrotliEncoderOperation::BROTLI_OPERATION_FINISH,
        )?;

        Ok(BrotliEncoderIsFinished(&self.state) == 1)
    }
}

impl fmt::Debug for BrotliEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrotliEncoder")
            .field("compress", &"<no debug>")
            .finish()
    }
}
