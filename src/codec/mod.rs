use std::io::Result;

mod gzip;

pub(crate) use gzip::GzipDecoder;

pub(crate) trait Decoder {
    /// Return `Some(Ok(bytes_consumed)` when header was finished
    /// Return `Some(Err(_))` if parsing fails
    /// Return `None` when more bytes needed
    fn parse_header(&mut self, input: &[u8]) -> Option<Result<usize>>;

    /// Return `Ok((more_buffers_needed, input_consumed, output_produced))`
    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)>;

    /// Return `Ok(more_buffer_needed, output_produced)`
    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)>;

    /// Return `Ok(())` if trailer was checked successfully
    /// Return `Err(_)` if checking fails
    fn check_footer(&mut self, input: &[u8]) -> Result<()>;
}
