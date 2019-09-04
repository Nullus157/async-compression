#[macro_use]
mod utils;

test_cases!(deflate::{stream::{compress, decompress}, bufread::compress});
