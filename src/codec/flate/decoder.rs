use crate::codec::Decoder;
use std::io::{Error, ErrorKind, Result};

use flate2::{Decompress, FlushDecompress, Status};

#[derive(Debug)]
pub struct FlateDecoder {
    decompress: Decompress,
}

impl FlateDecoder {
    pub(crate) fn new(zlib_header: bool) -> Self {
        Self {
            decompress: Decompress::new(zlib_header),
        }
    }

    fn do_decode(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushDecompress,
    ) -> Result<(Status, usize, usize)> {
        let prior_in = self.decompress.total_in();
        let prior_out = self.decompress.total_out();

        let status = self.decompress.decompress(input, output, flush)?;

        let in_length = (self.decompress.total_in() - prior_in) as usize;
        let out_length = (self.decompress.total_out() - prior_out) as usize;

        Ok((status, in_length, out_length))
    }
}

impl Decoder for FlateDecoder {
    fn parse_header(&mut self, _input: &[u8]) -> Option<Result<usize>> {
        Some(Ok(0))
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let (status, in_length, out_length) =
            self.do_decode(input, output, FlushDecompress::None)?;

        match status {
            Status::Ok => Ok((false, in_length, out_length)),
            Status::StreamEnd => Ok((true, in_length, out_length)),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (status, _, out_length) = self.do_decode(&[], output, FlushDecompress::Finish)?;

        match status {
            Status::Ok => Ok((false, out_length)),
            Status::StreamEnd => Ok((true, out_length)),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
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
