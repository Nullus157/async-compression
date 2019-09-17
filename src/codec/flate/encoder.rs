use crate::codec::Encode;
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

    fn do_encode(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushCompress,
    ) -> Result<(Status, usize, usize)> {
        let prior_in = self.compress.total_in();
        let prior_out = self.compress.total_out();

        let status = self.compress.compress(input, output, flush)?;

        let in_length = (self.compress.total_in() - prior_in) as usize;
        let out_length = (self.compress.total_out() - prior_out) as usize;

        Ok((status, in_length, out_length))
    }
}

impl Encode for FlateEncoder {
    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize)> {
        let (status, in_length, out_length) = self.do_encode(input, output, FlushCompress::None)?;

        match status {
            Status::Ok => Ok((in_length, out_length)),
            Status::StreamEnd => unreachable!(),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (status, _, out_length) = self.do_encode(&[], output, FlushCompress::Finish)?;

        match status {
            Status::Ok => Ok((false, out_length)),
            Status::StreamEnd => Ok((true, out_length)),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }
}
