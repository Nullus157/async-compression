use std::io::Result;

#[derive(Debug)]
pub struct ZlibDecoder {
    inner: crate::codec::FlateDecoder,
}

impl ZlibDecoder {
    pub(crate) fn new() -> Self {
        Self {
            inner: crate::codec::FlateDecoder::new(true),
        }
    }
}

impl crate::codec::Decode for ZlibDecoder {
    fn parse_header(&mut self, input: &[u8]) -> Option<Result<usize>> {
        self.inner.parse_header(input)
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)> {
        self.inner.decode(input, output)
    }

    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)> {
        self.inner.flush(output)
    }

    fn check_footer(&mut self, input: &[u8]) -> Result<()> {
        self.inner.check_footer(input)
    }
}
