use std::{fmt, io};

use liblzma::stream::{Action, Status, Stream};

use crate::{codec::Decode, util::PartialBuffer};

pub struct Xz2Decoder {
    stream: Stream,
    #[cfg(feature = "xz-parallel")]
    threads: Option<std::num::NonZeroU32>,
}

impl fmt::Debug for Xz2Decoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Xz2Decoder").finish_non_exhaustive()
    }
}

impl Xz2Decoder {
    pub fn new(mem_limit: u64) -> Self {
        Self {
            stream: Stream::new_auto_decoder(mem_limit, 0).unwrap(),
            #[cfg(feature = "xz-parallel")]
            threads: None,
        }
    }

    #[cfg(feature = "xz-parallel")]
    pub fn parallel(threads: std::num::NonZeroU32, mem_limit: u64) -> Self {
        Self {
            stream: liblzma::stream::MtStreamBuilder::new()
                .threads(threads.get())
                .timeout_ms(300)
                .memlimit_stop(mem_limit)
                .decoder()
                .unwrap(),
            threads: Some(threads),
        }
    }
}

impl Decode for Xz2Decoder {
    fn reinit(&mut self) -> io::Result<()> {
        #[cfg(feature = "xz-parallel")]
        {
            *self = match self.threads {
                Some(threads) => Self::parallel(threads, self.stream.memlimit()),
                None => Self::new(self.stream.memlimit()),
            };
        }

        #[cfg(not(feature = "xz-parallel"))]
        {
            *self = Self::new(self.stream.memlimit());
        }

        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> io::Result<bool> {
        let previous_in = self.stream.total_in() as usize;
        let previous_out = self.stream.total_out() as usize;

        let status = self
            .stream
            .process(input.unwritten(), output.unwritten_mut(), Action::Run)?;

        input.advance(self.stream.total_in() as usize - previous_in);
        output.advance(self.stream.total_out() as usize - previous_out);

        match status {
            Status::Ok => Ok(false),
            Status::StreamEnd => Ok(true),
            Status::GetCheck => Err(io::Error::other("Unexpected lzma integrity check")),
            Status::MemNeeded => Err(io::Error::other("More memory needed")),
        }
    }

    fn flush(
        &mut self,
        _output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> io::Result<bool> {
        // While decoding flush is a noop
        Ok(true)
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> io::Result<bool> {
        let previous_out = self.stream.total_out() as usize;

        let status = self
            .stream
            .process(&[], output.unwritten_mut(), Action::Finish)?;

        output.advance(self.stream.total_out() as usize - previous_out);

        match status {
            Status::Ok => Ok(false),
            Status::StreamEnd => Ok(true),
            Status::GetCheck => Err(io::Error::other("Unexpected lzma integrity check")),
            Status::MemNeeded => Err(io::Error::other("More memory needed")),
        }
    }
}
