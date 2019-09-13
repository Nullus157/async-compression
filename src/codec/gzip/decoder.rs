use crate::codec::Decode;
use std::io::{Error, ErrorKind, Result};

use flate2::Crc;

#[derive(Debug)]
pub struct GzipDecoder {
    inner: crate::codec::FlateDecoder,
    crc: Crc,
}

impl GzipDecoder {
    pub(crate) fn new() -> Self {
        Self {
            inner: crate::codec::FlateDecoder::new(false),
            crc: Crc::new(),
        }
    }
}

impl Decode for GzipDecoder {
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
        let (done, in_length, out_length) = self.inner.decode(input, output)?;
        self.crc.update(&output[..out_length]);
        Ok((done, in_length, out_length))
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (done, out_length) = self.inner.flush(output)?;
        self.crc.update(&output[..out_length]);
        Ok((done, out_length))
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
