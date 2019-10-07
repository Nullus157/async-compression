use crate::{codec::Decode, util::PartialBuffer};
use std::fmt;
use std::io::{Error, ErrorKind, Result};

use bzip2::{Decompress, Status};

pub struct BzDecoder {
    decompress: Decompress,
}

impl fmt::Debug for BzDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BzDecoder {{total_in: {}, total_out: {}}}",
            self.decompress.total_in(),
            self.decompress.total_out()
        )
    }
}

impl BzDecoder {
    pub(crate) fn new() -> Self {
        Self {
            decompress: Decompress::new(false),
        }
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<Status> {
        let prior_in = self.decompress.total_in();
        let prior_out = self.decompress.total_out();

        let status = self
            .decompress
            .decompress(input.unwritten(), output.unwritten_mut())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        input.advance((self.decompress.total_in() - prior_in) as usize);
        output.advance((self.decompress.total_out() - prior_out) as usize);

        Ok(status)
    }
}

impl Decode for BzDecoder {
    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool> {
        match self.decode(input, output)? {
            // Decompression went fine, nothing much to report.
            Status::Ok => Ok(false),

            // The Flush action on a compression went ok.
            Status::FlushOk => unreachable!(),

            // THe Run action on compression went ok.
            Status::RunOk => unreachable!(),

            // The Finish action on compression went ok.
            Status::FinishOk => unreachable!(),

            // The stream's end has been met, meaning that no more data can be input.
            Status::StreamEnd => Ok(true),

            // There was insufficient memory in the input or output buffer to complete
            // the request, but otherwise everything went normally.
            Status::MemNeeded => Err(Error::new(ErrorKind::Other, "out of memory")),
        }
    }

    fn finish(&mut self, _output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        Ok(true)
    }
}
