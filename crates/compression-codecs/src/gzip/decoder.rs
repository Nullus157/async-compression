use super::header::{self, Header};
use crate::{DecodeV2, FlateDecoder};
use compression_core::util::{PartialBuffer, WriteBuffer};
use flate2::Crc;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
enum State {
    Header(header::Parser),
    Decoding,
    Footer(PartialBuffer<Vec<u8>>),
    Done,
}

#[derive(Debug)]
pub struct GzipDecoder {
    inner: FlateDecoder,
    crc: Crc,
    state: State,
    header: Header,
}

fn check_footer(crc: &Crc, input: &[u8]) -> Result<()> {
    if input.len() < 8 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid gzip footer length",
        ));
    }

    let crc_sum = crc.sum().to_le_bytes();
    let bytes_read = crc.amount().to_le_bytes();

    if crc_sum != input[0..4] {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "CRC computed does not match",
        ));
    }

    if bytes_read != input[4..8] {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "amount of bytes read does not match",
        ));
    }

    Ok(())
}

impl Default for GzipDecoder {
    fn default() -> Self {
        Self {
            inner: FlateDecoder::new(false),
            crc: Crc::new(),
            state: State::Header(header::Parser::default()),
            header: Header::default(),
        }
    }
}

impl GzipDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    fn process(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
        inner: impl Fn(&mut Self, &mut PartialBuffer<&[u8]>, &mut WriteBuffer<'_>) -> Result<bool>,
    ) -> Result<bool> {
        loop {
            match &mut self.state {
                State::Header(parser) => {
                    if let Some(header) = parser.input(input)? {
                        self.header = header;
                        self.state = State::Decoding;
                    }
                }

                State::Decoding => {
                    let prior = output.written_len();

                    let res = inner(self, input, output);

                    if output.written_len() > prior {
                        // update CRC even if there was an error
                        self.crc.update(&output.written()[prior..]);
                    }

                    let done = res?;

                    if done {
                        self.state = State::Footer(vec![0; 8].into())
                    }
                }

                State::Footer(footer) => {
                    footer.copy_unwritten_from(input);

                    if footer.unwritten().is_empty() {
                        check_footer(&self.crc, footer.written())?;
                        self.state = State::Done
                    }
                }

                State::Done => {}
            };

            if let State::Done = self.state {
                return Ok(true);
            }

            if input.unwritten().is_empty() || output.has_no_spare_space() {
                return Ok(false);
            }
        }
    }
}

impl DecodeV2 for GzipDecoder {
    fn reinit(&mut self) -> Result<()> {
        self.inner.reinit()?;
        self.crc = Crc::new();
        self.state = State::Header(header::Parser::default());
        self.header = Header::default();
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> Result<bool> {
        self.process(input, output, |this, input, output| {
            this.inner.decode(input, output)
        })
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> Result<bool> {
        loop {
            match self.state {
                State::Header(_) | State::Footer(_) | State::Done => return Ok(true),

                State::Decoding => {
                    let prior = output.written_len();
                    let done = self.inner.flush(output)?;
                    self.crc.update(&output.written()[prior..]);
                    if done {
                        return Ok(true);
                    }
                }
            };

            if output.has_no_spare_space() {
                return Ok(false);
            }
        }
    }

    fn finish(&mut self, _output: &mut WriteBuffer<'_>) -> Result<bool> {
        // Because of the footer we have to have already flushed all the data out before we get here
        if let State::Done = self.state {
            Ok(true)
        } else {
            Err(Error::from(ErrorKind::UnexpectedEof))
        }
    }
}
