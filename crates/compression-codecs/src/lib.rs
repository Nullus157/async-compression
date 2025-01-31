#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

// Re-export `compression_core` for convenience.
pub use compression_core::*;

mod unshared;

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
#[cfg(feature = "lzma")]
pub mod lzma;
#[cfg(feature = "xz")]
pub mod xz;
#[cfg(feature = "xz2")]
pub mod xz2;
#[cfg(feature = "zlib")]
pub mod zlib;
#[cfg(feature = "zstd")]
pub mod zstd;
