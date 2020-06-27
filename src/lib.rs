//! Adaptors between compression crates and Rust's modern asynchronous IO types.
//!

//! # Feature Organization
//!
//! This crate is divided up along two axes, which can each be individually selected via Cargo
//! features.
//!
//! All features are disabled by default, you should enable just the ones you need from the lists
//! below.
//!
//! If you want to pull in everything there are three group features defined:
//!

//!  Feature | Does
//! ---------|------
//!  `all`   | Activates all implementations and algorithms.
//!  `all-implementations` | Activates all implementations, needs to be paired with a selection of algorithms
//!  `all-algorithms` | Activates all algorithms, needs to be paired with a selection of implementations
//!

//! ## IO implementation
//!
//! The first division is which underlying asynchronous IO trait will be wrapped, these are
//! available as separate features that have corresponding top-level modules:
//!

//!  Feature | Type
//! ---------|------
// TODO: Kill rustfmt on this section, `#![rustfmt::skip::attributes(cfg_attr)]` should do it, but
// that's unstable
#![cfg_attr(
    feature = "futures-bufread",
    doc = "[`futures-bufread`](crate::futures::bufread) | [`futures::io::AsyncBufRead`](futures_io::AsyncBufRead)"
)]
#![cfg_attr(
    not(feature = "futures-bufread"),
    doc = "`futures-bufread` (*inactive*) | `futures::io::AsyncBufRead`"
)]
#![cfg_attr(
    feature = "futures-write",
    doc = "[`futures-write`](crate::futures::write) | [`futures::io::AsyncWrite`](futures_io::AsyncWrite)"
)]
#![cfg_attr(
    not(feature = "futures-write"),
    doc = "`futures-write` (*inactive*) | `futures::io::AsyncWrite`"
)]
#![cfg_attr(
    feature = "stream",
    doc = "[`stream`] | [`futures::stream::Stream`](futures_core::stream::Stream)`<Item = `[`std::io::Result`]`<`[`bytes::Bytes`]`>>`"
)]
#![cfg_attr(
    not(feature = "stream"),
    doc = "`stream` (*inactive*) | `futures::stream::Stream<Item = std::io::Result<bytes::Bytes>>`"
)]
#![cfg_attr(
    feature = "tokio-02-bufread",
    doc = "[`tokio-02-bufread`](crate::tokio_02::bufread) | [`tokio::io::AsyncBufRead`](::tokio_02::io::AsyncBufRead)"
)]
#![cfg_attr(
    not(feature = "tokio-02-bufread"),
    doc = "`tokio-02-bufread` (*inactive*) | `tokio::io::AsyncBufRead`"
)]
#![cfg_attr(
    feature = "tokio-02-write",
    doc = "[`tokio-02-write`](crate::tokio_02::write) | [`tokio::io::AsyncWrite`](::tokio_02::io::AsyncWrite)"
)]
#![cfg_attr(
    not(feature = "tokio-02-write"),
    doc = "`tokio-02-write` (*inactive*) | `tokio::io::AsyncWrite`"
)]
//!

//! ## Compression algorithm
//!
//! The second division is which compression schemes to support, there are currently a few
//! available choices, these determine which types will be available inside the above modules:
//!

//!  Feature | Types
//! ---------|------
#![cfg_attr(
    feature = "brotli",
    doc = "`brotli` | [`BrotliEncoder`](?search=BrotliEncoder), [`BrotliDecoder`](?search=BrotliDecoder)"
)]
#![cfg_attr(
    not(feature = "brotli"),
    doc = "`brotli` (*inactive*) | `BrotliEncoder`, `BrotliDecoder`"
)]
#![cfg_attr(
    feature = "bzip2",
    doc = "`bzip2` | [`BzEncoder`](?search=BzEncoder), [`BzDecoder`](?search=BzDecoder)"
)]
#![cfg_attr(
    not(feature = "bzip2"),
    doc = "`bzip2` (*inactive*) | `BzEncoder`, `BzDecoder`"
)]
#![cfg_attr(
    feature = "deflate",
    doc = "`deflate` | [`DeflateEncoder`](?search=DeflateEncoder), [`DeflateDecoder`](?search=DeflateDecoder)"
)]
#![cfg_attr(
    not(feature = "deflate"),
    doc = "`deflate` (*inactive*) | `DeflateEncoder`, `DeflateDecoder`"
)]
#![cfg_attr(
    feature = "gzip",
    doc = "`gzip` | [`GzipEncoder`](?search=GzipEncoder), [`GzipDecoder`](?search=GzipDecoder)"
)]
#![cfg_attr(
    not(feature = "gzip"),
    doc = "`gzip` (*inactive*) | `GzipEncoder`, `GzipDecoder`"
)]
#![cfg_attr(
    feature = "lzma",
    doc = "`lzma` | [`LzmaEncoder`](?search=LzmaEncoder), [`LzmaDecoder`](?search=LzmaDecoder)"
)]
#![cfg_attr(
    not(feature = "lzma"),
    doc = "`lzma` (*inactive*) | `LzmaEncoder`, `LzmaDecoder`"
)]
#![cfg_attr(
    feature = "xz",
    doc = "`xz` | [`XzEncoder`](?search=XzEncoder), [`XzDecoder`](?search=XzDecoder)"
)]
#![cfg_attr(
    not(feature = "xz"),
    doc = "`xz` (*inactive*) | `XzEncoder`, `XzDecoder`"
)]
#![cfg_attr(
    feature = "zlib",
    doc = "`zlib` | [`ZlibEncoder`](?search=ZlibEncoder), [`ZlibDecoder`](?search=ZlibDecoder)"
)]
#![cfg_attr(
    not(feature = "zlib"),
    doc = "`zlib` (*inactive*) | `ZlibEncoder`, `ZlibDecoder`"
)]
#![cfg_attr(
    feature = "zstd",
    doc = "`zstd` | [`ZstdEncoder`](?search=ZstdEncoder), [`ZstdDecoder`](?search=ZstdDecoder)"
)]
#![cfg_attr(
    not(feature = "zstd"),
    doc = "`zstd` (*inactive*) | `ZstdEncoder`, `ZstdDecoder`"
)]
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_copy_implementations,
    missing_debug_implementations
)]
#![cfg_attr(not(all), allow(unused))]

#[macro_use]
mod macros;
mod codec;

#[cfg(any(feature = "futures-bufread", feature = "futures-write"))]
pub mod futures;
#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
pub mod stream;
#[cfg(any(feature = "tokio-02-bufread", feature = "tokio-02-write"))]
pub mod tokio_02;

mod unshared;
mod util;

#[cfg(feature = "brotli")]
use brotli::enc::backward_references::BrotliEncoderParams;

/// Level of compression data should be compressed with.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum Level {
    /// Fastest quality of compression, usually produces bigger size.
    Fastest,
    /// Best quality of compression, usually produces the smallest size.
    Best,
    /// Default quality of compression defined by the selected compression algorithm.
    Default,
    /// Precise quality based on the underlying compression algorithms'
    /// qualities. The interpretation of this depends on the algorithm chosen
    /// and the specific implementation backing it.
    /// Qualities are implicitly clamped to the algorithm's maximum.
    ///
    /// For bzip2, this is treated as default.
    Precise(u32),
}

impl Level {
    #[cfg(feature = "brotli")]
    fn into_brotli(self, mut params: BrotliEncoderParams) -> BrotliEncoderParams {
        match self {
            Self::Fastest => params.quality = 0,
            Self::Best => params.quality = 11,
            Self::Precise(quality) => params.quality = std::cmp::min(quality, 11) as i32,
            Self::Default => (),
        }

        params
    }

    #[cfg(feature = "bzip2")]
    fn into_bzip2(self) -> bzip2::Compression {
        match self {
            Self::Fastest => bzip2::Compression::Fastest,
            Self::Best => bzip2::Compression::Best,
            Self::Precise(_) => bzip2::Compression::Default,
            Self::Default => bzip2::Compression::Default,
        }
    }

    #[cfg(feature = "flate2")]
    fn into_flate2(self) -> flate2::Compression {
        match self {
            Self::Fastest => flate2::Compression::fast(),
            Self::Best => flate2::Compression::best(),
            Self::Precise(quality) => flate2::Compression::new(std::cmp::min(quality, 10) as u32),
            Self::Default => flate2::Compression::default(),
        }
    }

    #[cfg(feature = "zstd")]
    fn into_zstd(self) -> i32 {
        match self {
            Self::Fastest => 1,
            Self::Best => 21,
            Self::Precise(quality) => std::cmp::min(quality, 21) as i32,
            Self::Default => 0,
        }
    }

    #[cfg(feature = "xz2")]
    fn into_xz2(self) -> u32 {
        match self {
            Self::Fastest => 0,
            Self::Best => 9,
            Self::Precise(quality) => std::cmp::min(quality, 9),
            Self::Default => 5,
        }
    }
}
