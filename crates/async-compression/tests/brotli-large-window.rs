//! The `br` content coding is RFC 7932 brotli, whose window is at most 16 MiB. The non-standard
//! large-window format lets a tiny stream declare a window of up to 1 GiB, which the decoder
//! would allocate up front, so it must be rejected.

use async_compression::tokio::bufread::BrotliDecoder;
use tokio::io::AsyncReadExt as _;

fn compress(input: &[u8], lgwin: i32, large_window: bool) -> Vec<u8> {
    let params = brotli::enc::BrotliEncoderParams {
        lgwin,
        large_window,
        ..Default::default()
    };
    let mut output = Vec::new();
    brotli::BrotliCompress(&mut &input[..], &mut output, &params).unwrap();
    output
}

/// A large-window stream starts with the 7-bit marker `0b0010001`, followed by the window bits.
fn is_large_window(stream: &[u8]) -> bool {
    stream[0] & 0x7f == 0x11
}

async fn decode(input: &[u8], multiple_members: bool) -> std::io::Result<Vec<u8>> {
    let mut decoder = BrotliDecoder::new(input);
    decoder.multiple_members(multiple_members);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output).await?;
    Ok(output)
}

#[tokio::test]
async fn standard_window_is_accepted() {
    let input = b"hello world, ".repeat(100);
    let compressed = compress(&input, 24, false);
    assert!(!is_large_window(&compressed));
    assert_eq!(decode(&compressed, false).await.unwrap(), input);
}

#[tokio::test]
async fn large_window_is_rejected() {
    let input = b"hello world, ".repeat(100);
    for lgwin in [25, 30] {
        let compressed = compress(&input, lgwin, true);
        assert!(is_large_window(&compressed), "lgwin {}", lgwin);
        decode(&compressed, false).await.unwrap_err();
    }
}

#[tokio::test]
async fn large_window_is_rejected_after_reinit() {
    let input = b"hello world, ".repeat(100);
    let mut compressed = compress(&input, 24, false);
    compressed.extend(compress(&input, 30, true));
    decode(&compressed, true).await.unwrap_err();
}

async fn decode_large_window(input: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = BrotliDecoder::with_large_window(input);
    decoder.multiple_members(true);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output).await?;
    Ok(output)
}

#[tokio::test]
async fn large_window_is_accepted_when_enabled() {
    let input = b"hello world, ".repeat(100);
    let mut compressed = compress(&input, 24, false);
    compressed.extend(compress(&input, 30, true));
    // The second member also checks that `reinit()` keeps the setting.
    compressed.extend(compress(&input, 25, true));
    assert_eq!(
        decode_large_window(&compressed).await.unwrap(),
        input.repeat(3)
    );
}
