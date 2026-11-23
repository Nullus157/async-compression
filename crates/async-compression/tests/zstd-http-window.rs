//! `new_for_http` rejects frames whose window is larger than the 8 MiB RFC 9659 allows for the
//! HTTP `zstd` content coding, while `new` keeps accepting them.

use std::io::Write as _;
use tokio::io::{AsyncRead, AsyncReadExt as _};

/// A frame of `len` zero bytes whose header declares a `2^window_log` byte window. The content
/// size is left out, so the encoder cannot shrink the window to fit the input.
fn compress(window_log: u32, len: usize) -> Vec<u8> {
    let mut encoder = libzstd::stream::write::Encoder::new(Vec::new(), 3).unwrap();
    encoder
        .set_parameter(libzstd::stream::raw::CParameter::WindowLog(window_log))
        .unwrap();
    encoder.include_contentsize(false).unwrap();
    encoder.write_all(&vec![0; len]).unwrap();
    encoder.finish().unwrap()
}

async fn decode(mut decoder: impl AsyncRead + Unpin) -> std::io::Result<usize> {
    let mut output = Vec::new();
    decoder.read_to_end(&mut output).await?;
    Ok(output.len())
}

const LEN: usize = 32 << 20;

macro_rules! window_tests {
    ($mod:ident, $decoder:ident) => {
        mod $mod {
            use super::*;
            use async_compression::tokio::bufread::$decoder;

            #[tokio::test]
            async fn http_accepts_8_mib_window() {
                let compressed = compress(23, LEN);
                assert_eq!(
                    decode($decoder::new_for_http(&compressed[..]))
                        .await
                        .unwrap(),
                    LEN
                );
            }

            #[tokio::test]
            async fn http_rejects_16_mib_window() {
                let compressed = compress(24, LEN);
                decode($decoder::new_for_http(&compressed[..]))
                    .await
                    .unwrap_err();
            }

            #[tokio::test]
            async fn default_rejects_256_mib_window() {
                let compressed = compress(28, LEN);
                decode($decoder::new(&compressed[..])).await.unwrap_err();
            }

            #[tokio::test]
            async fn default_accepts_128_mib_window() {
                let compressed = compress(27, LEN);
                assert_eq!(decode($decoder::new(&compressed[..])).await.unwrap(), LEN);
            }
        }
    };
}

#[cfg(feature = "zstd")]
window_tests!(zstd, ZstdDecoder);

#[cfg(all(async_compression_unstable, feature = "zstd-mzstd"))]
window_tests!(mzstd, MzstdDecoder);

#[cfg(all(async_compression_unstable, feature = "zstd-zrip"))]
window_tests!(zrip, ZripDecoder);
