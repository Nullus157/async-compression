use crate::zstd::params::DParameter;
use crate::{Decode, DecodedSize};
use compression_core::unshared::Unshared;
use compression_core::util::PartialBuffer;
use libzstd::stream::raw::{Decoder, Operation};
use std::convert::TryInto;
use std::io;
use std::io::Result;
use zstd_safe::get_error_name;

#[derive(Debug)]
pub struct ZstdDecoder {
    decoder: Unshared<Decoder<'static>>,
}

impl Default for ZstdDecoder {
    fn default() -> Self {
        Self {
            decoder: Unshared::new(Decoder::new().unwrap()),
        }
    }
}

impl ZstdDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_params(params: &[DParameter]) -> Self {
        let mut decoder = Decoder::new().unwrap();
        for param in params {
            decoder.set_parameter(param.as_zstd()).unwrap();
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

        // See docs on remaining field
        let finished_frame = status.remaining == 0;
        // ...                    && "no more data"
        let done = finished_frame && status.bytes_read == 0;
        Ok(done)
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

impl DecodedSize for ZstdDecoder {
    fn decoded_size(input: &[u8]) -> Result<u64> {
        zstd_safe::find_frame_compressed_size(input)
            .map_err(|error_code| io::Error::other(get_error_name(error_code)))
            .and_then(|size| {
                size.try_into()
                    .map_err(|_| io::Error::from(io::ErrorKind::FileTooLarge))
            })
    }
}
