//! Adaptors between compression crates and Rust's modern asynchronous IO types.
//!
//!
//! # Feature Organization
//!
//! This crate is divided up along two axes, which can each be individually selected via Cargo
//! features.
//!
//! ## IO type
//!
//! The first division is which underlying asynchronous IO type will be wrapped, these are
//! available as two separate features that have corresponding top-level modules:
//!
//!  * [`bufread`] provides types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead)
//!    streams
//!
//!  * [`stream`] provides types which operate over [`Stream`](futures::stream::Stream)`<Item =
//!    `[`io::Result`](std::io::Result)`<`[`Bytes`](bytes::Bytes)`>>` streams
//!
//! ## Compression implementation
//!
//! The second division is which compression scheme to use, there are currently a few available
//! choices, these determine which types will be available inside the above modules:
//!
//!  * `brotli`
//!  * `deflate`
//!  * `gzip`
//!  * `zlib`
//!  * `zstd`

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_copy_implementations,
    missing_debug_implementations
)]

#[cfg(feature = "bufread")]
#[cfg_attr(docsrs, doc(cfg(feature = "bufread")))]
pub mod bufread;
#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
pub mod stream;

/// Types to configure [`flate2`](::flate2) based encoders.
#[cfg(any(feature = "deflate", feature = "zlib", feature = "gzip"))]
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

mod unshared;
