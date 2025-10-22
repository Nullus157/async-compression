//! Adaptors for various compression algorithms.

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::io::Result;

pub use compression_core as core;

#[cfg(feature = "brotli")]
pub mod brotli;
#[cfg(feature = "bzip2")]
pub mod bzip2;
#[cfg(feature = "deflate")]
pub mod deflate;
#[cfg(feature = "deflate64")]
pub mod deflate64;
#[cfg(feature = "flate2")]
pub mod flate;
#[cfg(feature = "gzip")]
pub mod gzip;
#[cfg(feature = "lz4")]
pub mod lz4;
#[cfg(feature = "lzma")]
pub mod lzma;
#[cfg(feature = "xz")]
pub mod xz;
#[cfg(feature = "lzma")]
pub mod xz2;
#[cfg(feature = "zlib")]
pub mod zlib;
#[cfg(feature = "zstd")]
pub mod zstd;

use compression_core::util::PartialBuffer;

#[cfg(feature = "brotli")]
pub use self::brotli::{BrotliDecoder, BrotliEncoder};
#[cfg(feature = "bzip2")]
pub use self::bzip2::{BzDecoder, BzEncoder};
#[cfg(feature = "deflate")]
pub use self::deflate::{DeflateDecoder, DeflateEncoder};
#[cfg(feature = "deflate64")]
pub use self::deflate64::Deflate64Decoder;
#[cfg(feature = "flate2")]
pub use self::flate::{FlateDecoder, FlateEncoder};
#[cfg(feature = "gzip")]
pub use self::gzip::{GzipDecoder, GzipEncoder};
#[cfg(feature = "lz4")]
pub use self::lz4::{Lz4Decoder, Lz4Encoder};
#[cfg(feature = "lzma")]
pub use self::lzma::{LzmaDecoder, LzmaEncoder};
#[cfg(feature = "xz")]
pub use self::xz::{XzDecoder, XzEncoder};
#[cfg(feature = "lzma")]
pub use self::xz2::{Xz2Decoder, Xz2Encoder, Xz2FileFormat};
#[cfg(feature = "zlib")]
pub use self::zlib::{ZlibDecoder, ZlibEncoder};
#[cfg(feature = "zstd")]
pub use self::zstd::{ZstdDecoder, ZstdEncoder};

fn forward_output<R>(
    output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    f: FnOnce(&mut PartialBuffer<&mut [u8]>) -> R,
) -> R {
    let written_len = output.written_len();
 
    let mut partial_buffer = PartialBuffer::new(output.get_mut().as_mut());
    partial_buffer.advance(written_len);

    let result = f(&mut partial_buffer);
    let new_written_len = partial_buffer.written_len();
    output.advance(new_written_len - written_len);
    result
}


fn forward_input_output<R>(
    input: &mut PartialBuffer<impl AsRef<[u8]>>,
    output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    f: FnOnce(&mut PartialBuffer<&[u8]>, &mut PartialBuffer<&mut [u8]>) -> R,
) -> R {
    let written_len = input.written_len();
 
    let mut partial_buffer = PartialBuffer::new(input.get_mut().as_ref());
    partial_buffer.advance(written_len);

    let result = forward_output(output, |output| f(&mut partial_buffer, output));
    let new_written_len = partial_buffer.written_len();
    input.advance(new_written_len - written_len);
    result
}

pub trait Encode {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<()>;

    /// Returns whether the internal buffers are flushed
    fn flush(&mut self, output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>)
        -> Result<bool>;

    /// Returns whether the internal buffers are flushed and the end of the stream is written
    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool>;
}
impl<T: EncodeV2 + ?Sized> Encode for T {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<()> {
        forward_input_output(input, output, |input, output| self.encode_init(input, output))
    }

    fn flush(&mut self, output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>)
        -> Result<bool>
    {
        forward_output(output, |output| self.flush_init(output))
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool>
    {
        forward_output(output, |output| self.finish_init(output))
    }
}

/// Object safe version of [`Encode`], its method takes a `&mut [u8]` as output buffer.
pub trait EncodeV2 {
    /// same as [`Encode::encode`]
    fn encode_init(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<()>;

    /// same as [`Encode::flush`]
    fn flush_init(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool>;

    /// same as [`Encode::finish`]
    fn finish_init(
        &mut self,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool>;
}

pub trait Decode {
    /// Reinitializes this decoder ready to decode a new member/frame of data.
    fn reinit(&mut self) -> Result<()>;

    /// Returns whether the end of the stream has been read
    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool>;
 
    /// Returns whether the internal buffers are flushed
    fn flush(&mut self, output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>)
        -> Result<bool>;

    /// Returns whether the internal buffers are flushed
    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool>;
}

impl<T: DecodeV2 + ?Sized> Decode for T {
    fn reinit(&mut self) -> Result<()> {
        DecodeV2::reinit(self)
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        forward_input_output(input, output, |input, output| self.decode_init(input, output))
    }
 
    /// Returns whether the internal buffers are flushed
    fn flush(&mut self, output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>)
        -> Result<bool>
    {
        forward_output(output, |output| self.flush_init(output))
    }

    /// Returns whether the internal buffers are flushed
    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool>
    {
        forward_output(output, |output| self.finish_init(output))
    }
}

/// Object safe version of [`Decode`], its method takes a `&mut [u8]` as output buffer.
pub trait DecodeV2 {
    /// same as [`Decode::reinit`]
    fn reinit(&mut self) -> Result<()>;

    /// same as [`Decode::decode`]
    fn decode_init(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool>;

    /// same as [`Decode::flush`]
    fn flush_init(&mut self, output: &mut PartialBuffer<&mut [u8]>) -> Result<bool>;

    /// same as [`Decode::finish`]
    fn finish_init(
        &mut self,
        output: &mut PartialBuffer<&mut [u8]>,
    ) -> Result<bool>;
}
