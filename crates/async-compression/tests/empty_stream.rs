//! Test that bufread decoders handle empty input streams (immediate EOF).

#[macro_use]
mod utils;

#[tokio::test]
async fn zstd_empty_stream() {
    use async_compression::tokio::bufread::ZstdDecoder;
    use std::io::Cursor;
    use tokio::io::AsyncReadExt;

    let empty: &[u8] = &[];
    let mut decoder = ZstdDecoder::new(Cursor::new(empty));
    let mut output = Vec::new();
    let result = decoder.read_to_end(&mut output).await;
    // Empty input should return Ok(0), not error with "zstd stream did not finish"
    assert!(result.is_ok(), "empty stream failed: {:?}", result);
    assert!(output.is_empty());
}
