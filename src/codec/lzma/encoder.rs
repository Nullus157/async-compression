use crate::{codec::Encode, util::PartialBuffer};

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::Result;
use xz2::stream::{Action, Check, Status, Stream};

pub struct LzmaEncoder {
    stream: Stream,
}

impl Debug for LzmaEncoder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "LzmaEncoder")
    }
}

impl LzmaEncoder {
    pub fn new(level: u32) -> Self {
        Self {
            stream: Stream::new_easy_encoder(level, Check::Crc64).unwrap(),
        }
    }
}

impl Encode for LzmaEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<()> {
        let previous_in = self.stream.total_in() as usize;
        let previous_out = self.stream.total_out() as usize;

        let status = self
            .stream
            .process(input.unwritten(), output.unwritten_mut(), Action::Run)?;

        input.advance(self.stream.total_in() as usize - previous_in);
        output.advance(self.stream.total_out() as usize - previous_out);

        match status {
            Status::Ok | Status::StreamEnd => Ok(()),
            Status::GetCheck => panic!("Unexpected lzma integrity check"),
            Status::MemNeeded => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "out of memory",
            )),
        }
    }

    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        let previous_out = self.stream.total_out() as usize;

        let status = self
            .stream
            .process(&[], output.unwritten_mut(), Action::SyncFlush)?;

        output.advance(self.stream.total_out() as usize - previous_out);

        match status {
            Status::Ok => Ok(false),
            Status::StreamEnd => Ok(true),
            Status::GetCheck => panic!("Unexpected lzma integrity check"),
            Status::MemNeeded => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "out of memory",
            )),
        }
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        let previous_out = self.stream.total_out() as usize;

        let status = self
            .stream
            .process(&[], output.unwritten_mut(), Action::Finish)?;

        output.advance(self.stream.total_out() as usize - previous_out);

        match status {
            Status::Ok => Ok(false),
            Status::StreamEnd => Ok(true),
            Status::GetCheck => panic!("Unexpected lzma integrity check"),
            Status::MemNeeded => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "out of memory",
            )),
        }
    }
}
