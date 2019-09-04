#[macro_use]
mod utils;

test_cases!(zlib::{stream::{compress, decompress}, bufread::compress});
