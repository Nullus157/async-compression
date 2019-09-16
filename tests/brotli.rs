#[macro_use]
mod utils;

test_cases!(brotli::{bufread::{compress, decompress}, stream::{compress, decompress}});
