use crate::{codec::Encode, util::PartialBuffer};
use std::io::Result;

use flate2::{Compression, Crc};

#[derive(Debug)]
enum State {
    Header(PartialBuffer<Vec<u8>>),
    Encoding,
    Footer(PartialBuffer<Vec<u8>>),
    Done,
    Invalid,
}

#[derive(Debug)]
pub struct GzipEncoder {
    inner: crate::codec::FlateEncoder,
    crc: Crc,
    state: State,
}

fn header(level: Compression) -> Vec<u8> {
    let level_byte = if level.level() >= Compression::best().level() {
        0x02
    } else if level.level() <= Compression::fast().level() {
        0x04
    } else {
        0x00
    };

    vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, level_byte, 0xff]
}

impl GzipEncoder {
    pub(crate) fn new(level: Compression) -> Self {
        Self {
            inner: crate::codec::FlateEncoder::new(level, false),
            crc: Crc::new(),
            state: State::Header(header(level).into()),
        }
    }

    fn footer(&mut self) -> Vec<u8> {
        let mut output = Vec::with_capacity(8);

        output.extend(&self.crc.sum().to_le_bytes());
        output.extend(&self.crc.amount().to_le_bytes());

        output
    }

    fn process(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
        inner: impl Fn(
            &mut Self,
            &mut PartialBuffer<&[u8]>,
            &mut PartialBuffer<&mut [u8]>,
        ) -> Result<bool>,
    ) -> Result<bool> {
        loop {
            self.state = match std::mem::replace(&mut self.state, State::Invalid) {
                State::Header(mut header) => {
                    output.copy_unwritten_from(&mut header);

                    if header.unwritten().is_empty() {
                        State::Encoding
                    } else {
                        State::Header(header)
                    }
                }

                State::Encoding => {
                    if inner(self, &mut *input, &mut *output)? {
                        State::Footer(self.footer().into())
                    } else {
                        State::Encoding
                    }
                }

                State::Footer(mut footer) => {
                    output.copy_unwritten_from(&mut footer);

                    if footer.unwritten().is_empty() {
                        State::Done
                    } else {
                        State::Footer(footer)
                    }
                }

                State::Done => State::Done,
                State::Invalid => panic!("Reached invalid state"),
            };

            if let State::Done = self.state {
                return Ok(true);
            }

            if input.unwritten().is_empty() || output.unwritten().is_empty() {
                return Ok(false);
            }
        }
    }
}

impl Encode for GzipEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()> {
        self.process(input, output, |this, input, output| {
            let prior_written = input.written().len();
            this.inner.encode(input, output)?;
            this.crc.update(&input.written()[prior_written..]);
            Ok(false)
        })
        .map(drop)
    }

    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool> {
        self.process(
            &mut PartialBuffer::new(&[][..]),
            output,
            |this, _, output| {
                let done = this.inner.finish(output)?;
                Ok(done)
            },
        )
    }
}
