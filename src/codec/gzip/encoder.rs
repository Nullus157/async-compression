use crate::codec::Encoder;
use std::io::{Error, ErrorKind, Result};

use flate2::{Compress, Compression, Crc, FlushCompress, Status};

#[derive(Debug)]
pub(crate) struct GzipEncoder {
    crc: Crc,
    compress: Compress,
    level: Compression,
}

impl GzipEncoder {
    pub(crate) fn new(level: Compression) -> Self {
        Self {
            crc: Crc::new(),
            compress: Compress::new(level, false),
            level,
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

        self.crc.update(&input[..in_length]);

        Ok((status, in_length, out_length))
    }
}

impl Encoder for GzipEncoder {
    fn write_header(&mut self, output: &mut [u8]) -> Result<usize> {
        if output.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "output buffer too short",
            ));
        }

        let level_byte = if self.level.level() >= Compression::best().level() {
            0x02
        } else if self.level.level() <= Compression::fast().level() {
            0x04
        } else {
            0x00
        };

        let header = [0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, level_byte, 0xff];

        output[0..10].copy_from_slice(&header);

        Ok(10)
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        if input.is_empty() {
            return Ok((true, 0, 0));
        }

        let (status, in_length, out_length) = self.do_encode(input, output, FlushCompress::None)?;

        match status {
            Status::Ok => Ok((false, in_length, out_length)),
            Status::StreamEnd => Ok((true, in_length, out_length)),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        let (status, _, out_length) = self.do_encode(&[], output, FlushCompress::Finish)?;

        match status {
            Status::Ok => Ok((false, out_length)),
            Status::StreamEnd => Ok((true, out_length)),
            Status::BufError => Err(Error::new(ErrorKind::Other, "unexpected BufError")),
        }
    }

    fn write_footer(&mut self, output: &mut [u8]) -> Result<usize> {
        if output.len() < 8 {}

        let crc = self.crc.sum().to_le_bytes();
        let bytes_read = self.crc.amount().to_le_bytes();

        output[0..4].copy_from_slice(&crc);
        output[4..8].copy_from_slice(&bytes_read);

        Ok(8)
    }
}
