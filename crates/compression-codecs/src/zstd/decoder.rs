use std::io;
use std::io::Result;

use crate::{unshared::Unshared, Decode, PartialBuffer};
use libzstd::stream::raw::{Decoder, Operation};
use zstd_safe::DParameter;

#[derive(Debug)]
pub struct ZstdDecoder {
    decoder: Unshared<Decoder<'static>>,
}

impl ZstdDecoder {
    pub fn new() -> Self {
        Self {
            decoder: Unshared::new(Decoder::new().unwrap()),
        }
    }

    pub fn new_with_params(params: impl Iterator<Item = DParameter>) -> Self {
        let mut decoder = Decoder::new().unwrap();
        for param in params {
            decoder.set_parameter(param).unwrap();
        }
        Self {
            decoder: Unshared::new(decoder),
        }
    }

    pub fn new_with_dict(dictionary: &[u8]) -> io::Result<Self> {
        let decoder = Decoder::with_dictionary(dictionary)?;
        Ok(Self {
            decoder: Unshared::new(decoder),
        })
    }
}

impl Default for ZstdDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decode for ZstdDecoder {
    fn reinit(&mut self) -> Result<()> {
        self.decoder.get_mut().reinit()?;
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        let status = self
            .decoder
            .get_mut()
            .run_on_buffers(input.unwritten(), output.unwritten_mut())?;
        input.advance(status.bytes_read);
        output.advance(status.bytes_written);
        Ok(status.remaining == 0)
    }

    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        let mut out_buf = zstd_safe::OutBuffer::around(output.unwritten_mut());
        let bytes_left = self.decoder.get_mut().flush(&mut out_buf)?;
        let len = out_buf.as_slice().len();
        output.advance(len);
        Ok(bytes_left == 0)
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        let mut out_buf = zstd_safe::OutBuffer::around(output.unwritten_mut());
        let bytes_left = self.decoder.get_mut().finish(&mut out_buf, true)?;
        let len = out_buf.as_slice().len();
        output.advance(len);
        Ok(bytes_left == 0)
    }
}
