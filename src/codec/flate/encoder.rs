use crate::{codec::Encode, util::PartialBuffer};
use std::io::{Error, ErrorKind, Result};

use flate2::{Compress, Compression, FlushCompress, Status};

#[derive(Debug)]
pub struct FlateEncoder {
    compress: Compress,
}

impl FlateEncoder {
    pub(crate) fn new(level: Compression, zlib_header: bool) -> Self {
        Self {
            compress: Compress::new(level, zlib_header),
        }
    }

    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
        flush: FlushCompress,
    ) -> Result<Status> {
        let prior_in = self.compress.total_in();
        let prior_out = self.compress.total_out();

        let status = self
            .compress
            .compress(input.unwritten(), output.unwritten_mut(), flush)?;

        input.advance((self.compress.total_in() - prior_in) as usize);
        output.advance((self.compress.total_out() - prior_out) as usize);

        Ok(status)
    }
}

impl Encode for FlateEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()> {
        match self.encode(input, output, FlushCompress::None)? {
            Status::Ok => Ok(()),
            Status::StreamEnd => unreachable!(),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        match self.encode(
            &mut PartialBuffer::new(&[][..]),
            output,
            FlushCompress::Finish,
        )? {
            Status::Ok => Ok(false),
            Status::StreamEnd => Ok(true),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }
}
