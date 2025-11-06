use crate::zstd::params::CParameter;
use crate::EncodeV2;
use compression_core::{
    unshared::Unshared,
    util::{PartialBuffer, WriteBuffer},
};
use libzstd::stream::raw::{Encoder, Operation};
use std::io;
use std::io::Result;

#[derive(Debug)]
pub struct ZstdEncoder {
    encoder: Unshared<Encoder<'static>>,
}

impl ZstdEncoder {
    pub fn new(level: i32) -> Self {
        Self {
            encoder: Unshared::new(Encoder::new(level).unwrap()),
        }
    }

    pub fn new_with_params(level: i32, params: &[CParameter]) -> Self {
        let mut encoder = Encoder::new(level).unwrap();
        for param in params {
            encoder.set_parameter(param.as_zstd()).unwrap();
        }
        Self {
            encoder: Unshared::new(encoder),
        }
    }

    pub fn new_with_dict(level: i32, dictionary: &[u8]) -> io::Result<Self> {
        let encoder = Encoder::with_dictionary(level, dictionary)?;
        Ok(Self {
            encoder: Unshared::new(encoder),
        })
    }

    fn call_fn_on_out_buffer(
        &mut self,
        output: &mut WriteBuffer<'_>,
        f: fn(&mut Encoder<'static>, &mut zstd_safe::OutBuffer<'_, [u8]>) -> io::Result<usize>,
    ) -> io::Result<bool> {
        let mut out_buf = zstd_safe::OutBuffer::around(output.initialize_unwritten());
        let res = f(self.encoder.get_mut(), &mut out_buf);
        let len = out_buf.as_slice().len();
        output.advance(len);

        res.map(|bytes_left| bytes_left == 0)
    }
}

impl EncodeV2 for ZstdEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> Result<()> {
        let status = self
            .encoder
            .get_mut()
            .run_on_buffers(input.unwritten(), output.initialize_unwritten())?;
        input.advance(status.bytes_read);
        output.advance(status.bytes_written);
        Ok(())
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> Result<bool> {
        self.call_fn_on_out_buffer(output, |encoder, out_buf| encoder.flush(out_buf))
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> Result<bool> {
        self.call_fn_on_out_buffer(output, |encoder, out_buf| encoder.finish(out_buf, true))
    }
}
