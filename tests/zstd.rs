#[macro_use]
mod utils;

test_cases!(zstd::stream::{compress, decompress});
