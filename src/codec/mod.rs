use crate::util::PartialBuffer;
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
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()>;

    /// Returns whether the internal buffers are flushed and the end of the stream is written
    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool>;
}

pub trait Decode {
    /// Returns whether the end of the stream has been read
    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool>;

    /// Returns whether the internal buffers are flushed
    fn finish(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool>;
}
