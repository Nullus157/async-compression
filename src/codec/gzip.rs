use crate::codec::Decoder;
use std::io::{Error, ErrorKind, Result};

use flate2::{Crc, Decompress, FlushDecompress, Status};

#[derive(Debug)]
pub(crate) struct GzipDecoder {
    crc: Crc,
    decompress: Decompress,
}

impl GzipDecoder {
    pub(crate) fn new() -> Self {
        Self {
            crc: Crc::new(),
            decompress: Decompress::new(false),
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

        self.crc.update(&output[..out_length]);

        Ok((status, in_length, out_length))
    }
}

impl Decoder for GzipDecoder {
    fn parse_header(&mut self, input: &[u8]) -> Option<Result<usize>> {
        if input.len() >= 10 {
            if input[0..3] == [0x1f, 0x8b, 0x08] {
                Some(Ok(10))
            } else {
                Some(Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid gzip header",
                )))
            }
        } else {
            None
        }
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
        match input.len().cmp(&8) {
            std::cmp::Ordering::Less => Err(Error::new(
                ErrorKind::UnexpectedEof,
                "reached unexpected EOF",
            )),
            std::cmp::Ordering::Greater => Err(Error::new(
                ErrorKind::InvalidData,
                "extra data after end of compressed block",
            )),
            std::cmp::Ordering::Equal => {
                let crc = self.crc.sum().to_le_bytes();
                let bytes_read = self.crc.amount().to_le_bytes();

                if crc != input[0..4] {
                    Err(Error::new(
                        ErrorKind::InvalidData,
                        "CRC computed does not match",
                    ))
                } else if bytes_read != input[4..8] {
                    Err(Error::new(
                        ErrorKind::InvalidData,
                        "amount of bytes read does not match",
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}
