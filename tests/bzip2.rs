#[macro_use]
mod utils;

test_cases!(bzip2::{bufread::{compress, decompress}, stream::{compress, decompress}});
