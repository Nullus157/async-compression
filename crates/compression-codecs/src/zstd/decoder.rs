use crate::zstd::params::DParameter;
use crate::{DecodeV2, DecodedSize};
use compression_core::{
    unshared::Unshared,
    util::{PartialBuffer, WriteBuffer},
};
use libzstd::stream::raw::{Decoder, Operation};
use std::convert::TryInto;
use std::io;
use std::io::Result;
use zstd_safe::get_error_name;

#[derive(Debug)]
pub struct ZstdDecoder {
    decoder: Unshared<Decoder<'static>>,
    stream_ended: bool,
}

impl Default for ZstdDecoder {
    fn default() -> Self {
        Self {
            decoder: Unshared::new(Decoder::new().unwrap()),
            stream_ended: false,
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
            stream_ended: false,
        }
    }

    pub fn new_with_dict(dictionary: &[u8]) -> io::Result<Self> {
        let decoder = Decoder::with_dictionary(dictionary)?;
        Ok(Self {
            decoder: Unshared::new(decoder),
            stream_ended: false,
        })
    }

    fn call_fn_on_out_buffer(
        &mut self,
        output: &mut WriteBuffer<'_>,
        f: fn(&mut Decoder<'static>, &mut zstd_safe::OutBuffer<'_, [u8]>) -> io::Result<usize>,
    ) -> io::Result<bool> {
        output.initialize_unwritten();

        let mut out_buf = zstd_safe::OutBuffer::around(output.unwritten_initialized_mut());
        let res = f(self.decoder.get_mut(), &mut out_buf);
        let len = out_buf.as_slice().len();
        output.advance(len);

        res.map(|bytes_left| bytes_left == 0)
    }
}

impl DecodeV2 for ZstdDecoder {
    fn reinit(&mut self) -> Result<()> {
        self.decoder.get_mut().reinit()?;
        self.stream_ended = false;
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> Result<bool> {
        output.initialize_unwritten();

        let status = self
            .decoder
            .get_mut()
            .run_on_buffers(input.unwritten(), output.unwritten_initialized_mut())?;
        input.advance(status.bytes_read);
        output.advance(status.bytes_written);

        let finished = status.remaining == 0;
        if finished {
            self.stream_ended = true;
        }
        Ok(finished)
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> Result<bool> {
        // Note: stream_ended is not updated here because zstd's flush only flushes
        // buffered output and doesn't indicate stream completion. Stream completion
        // is detected in decode() when status.remaining == 0.
        self.call_fn_on_out_buffer(output, |decoder, out_buf| decoder.flush(out_buf))
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> Result<bool> {
        self.call_fn_on_out_buffer(output, |decoder, out_buf| decoder.finish(out_buf, true))?;

        if self.stream_ended {
            Ok(true)
        } else {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "zstd stream did not finish",
            ))
        }
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
