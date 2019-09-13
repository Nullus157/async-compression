use std::io::Result;

#[cfg(feature = "brotli")]
mod brotli;
#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "flate2")]
mod flate;
#[cfg(feature = "gzip")]
mod gzip;
#[cfg(feature = "zlib")]
mod zlib;
#[cfg(feature = "zstd")]
mod zstd;

#[cfg(feature = "brotli")]
pub(crate) use self::brotli::{BrotliDecoder, BrotliEncoder};
#[cfg(feature = "deflate")]
pub(crate) use self::deflate::{DeflateDecoder, DeflateEncoder};
#[cfg(feature = "flate2")]
pub(crate) use self::flate::{FlateDecoder, FlateEncoder};
#[cfg(feature = "gzip")]
pub(crate) use self::gzip::{GzipDecoder, GzipEncoder};
#[cfg(feature = "zlib")]
pub(crate) use self::zlib::{ZlibDecoder, ZlibEncoder};
#[cfg(feature = "zstd")]
pub(crate) use self::zstd::{ZstdDecoder, ZstdEncoder};

pub trait Encode {
    /// Return `Ok(bytes_produced)` when header was written
    /// Return `Err(_)` if writing fails
    fn write_header(&mut self, output: &mut [u8]) -> Result<usize>;

    /// Return `Ok((done, input_consumed, output_produced))`
    fn encode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)>;

    /// Return `Ok(done, output_produced)`
    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)>;

    /// Return `Ok(bytes_produced)` if footer was written successfully
    /// Return `Err(_)` if writing fails
    fn write_footer(&mut self, ouput: &mut [u8]) -> Result<usize>;
}

pub trait Decode {
    /// Return `Ok(Some(bytes_consumed)` if header was finished
    /// Return `Ok(None)` if more bytes needed
    /// Return `Err(_)` if parsing fails
    fn parse_header(&mut self, input: &[u8]) -> Result<Option<usize>> {
        let _ = input;
        Ok(Some(0))
    }

    /// Return `Ok((done, input_consumed, output_produced))`
    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Result<(bool, usize, usize)>;

    /// Return `Ok(done, output_produced)`
    fn flush(&mut self, output: &mut [u8]) -> Result<(bool, usize)>;

    /// Return `Ok(Some(bytes_consumed))` if trailer was checked successfully
    /// Return `Ok(None)` if more bytes needed
    /// Return `Err(_)` if checking fails
    fn check_footer(&mut self, input: &[u8]) -> Result<Option<usize>> {
        let _ = input;
        Ok(Some(0))
    }
}
