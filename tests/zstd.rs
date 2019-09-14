#[macro_use]
mod utils;

test_cases!(zstd::{bufread::{compress, decompress}, stream::{compress, decompress}});
