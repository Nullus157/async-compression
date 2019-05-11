//! Adaptors between compression crates and Rust's modern asynchronous IO types.
//!
//!
//! # Organization
//!
//! This crate is divided up into a number of modules based on the underlying asynchronous IO type
//! that will be wrapped:
//!
//!  * [`read`] provides types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead)
//!    streams
//!  * [`stream`] provides types which operate over [`Stream`](futures::stream::Stream)`<Item =
//!    `[`io::Result`](std::io::Result)`<`[`Bytes`](bytes::Bytes)`>>` streams

pub mod read;
pub mod stream;

/// Types to configure [`flate2`](::flate2) based encoders.
pub mod flate2 {
    pub use flate2::Compression;
}

/// Types to configure [`brotli2`](::brotli2) based encoders.
pub mod brotli2 {
    pub use brotli2::CompressParams;
}
