//! Adaptors between compression crates and Rust's modern asynchronous IO types.
//!
//!
//! # Feature Organization
//!
//! This crate is divided up along two axes, which can each be individually selected via Cargo
//! features.
//!
//! All features are default active, it's recommended you use this crate with `default-features =
//! false` and enable just the features you need. (We're considering disabling this and shipping
//! with no features active by default, please [leave a comment][#46] if you have an opinion either
//! way).
//!
//! [#46]: https://github.com/rustasync/async-compression/issues/46
//!
//! ## IO type
//!
//! The first division is which underlying asynchronous IO type will be wrapped, these are
//! available as two separate features that have corresponding top-level modules:
//!
//!  Feature | Type
//! ---------|------
#![cfg_attr(feature = "bufread", doc = "[`bufread`] | [`futures::io::AsyncBufRead`](futures_io::AsyncBufRead)")]
#![cfg_attr(not(feature = "bufread"), doc = "`bufread` (*inactive*) | `futures::io::AsyncBufRead`")]
#![cfg_attr(feature = "write", doc = "[`write`](crate::write) | [`futures::io::AsyncWrite`](futures_io::AsyncWrite)")]
#![cfg_attr(not(feature = "write"), doc = "`write` (*inactive*) | `futures::io::AsyncWrite`")]
#![cfg_attr(feature = "stream", doc = "[`stream`] | [`futures::stream::Stream`](futures_core::stream::Stream)`<Item = `[`std::io::Result`]`<`[`bytes::Bytes`]`>>`")]
#![cfg_attr(not(feature = "stream"), doc = "`stream` (*inactive*) | `futures::stream::Stream<Item = std::io::Result<bytes::Bytes>>`")]
//!
//!
//! ## Compression implementation
//!
//! The second division is which compression scheme to use, there are currently a few available
//! choices, these determine which types will be available inside the above modules:
//!
#![cfg_attr(feature = "brotli", doc = "* `brotli`")]
#![cfg_attr(not(feature = "brotli"), doc = "* `brotli` (*inactive*)")]
#![cfg_attr(feature = "bzip", doc = "* `bzip`")]
#![cfg_attr(not(feature = "bzip"), doc = "* `bzip` (*inactive*)")]
#![cfg_attr(feature = "deflate", doc = "* `deflate`")]
#![cfg_attr(not(feature = "deflate"), doc = "* `deflate` (*inactive*)")]
#![cfg_attr(feature = "gzip", doc = "* `gzip`")]
#![cfg_attr(not(feature = "gzip"), doc = "* `gzip` (*inactive*)")]
#![cfg_attr(feature = "zlib", doc = "* `zlib`")]
#![cfg_attr(not(feature = "zlib"), doc = "* `zlib` (*inactive*)")]
#![cfg_attr(feature = "zstd", doc = "* `zstd`")]
#![cfg_attr(not(feature = "zstd"), doc = "* `zstd` (*inactive*)")]

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_copy_implementations,
    missing_debug_implementations
)]

mod codec;

#[cfg(feature = "bufread")]
#[cfg_attr(docsrs, doc(cfg(feature = "bufread")))]
pub mod bufread;
#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
pub mod stream;
#[cfg(feature = "write")]
#[cfg_attr(docsrs, doc(cfg(feature = "write")))]
pub mod write;

/// Types to configure [`flate2`](::flate2) based encoders.
#[cfg(feature = "flate2")]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "deflate", feature = "zlib", feature = "gzip")))
)]
pub mod flate2 {
    pub use flate2::Compression;
}

/// Types to configure [`brotli2`](::brotli2) based encoders.
#[cfg(feature = "brotli")]
#[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
pub mod brotli2 {
    pub use brotli2::CompressParams;
}

/// Types to configure [`bzip2`](::bzip2) based encoders.
#[cfg(feature = "bzip")]
#[cfg_attr(docsrs, doc(cfg(feature = "bzip")))]
pub mod bzip2 {
    pub use bzip2::Compression;
}
mod unshared;
mod util;
