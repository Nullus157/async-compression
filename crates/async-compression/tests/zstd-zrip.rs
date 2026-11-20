#![cfg(async_compression_unstable)]

#[macro_use]
mod utils;

test_cases!(zrip);

#[test]
pub fn zrip_params() {
    let _ = async_compression::zrip::CParameter::window_log(20);
    let _ = async_compression::zrip::DParameter::window_log_max(27);
}

#[cfg(feature = "tokio")]
mod params {
    use async_compression::{tokio::bufread::ZripEncoder, zrip::CParameter, Level};
    use tokio::io::AsyncReadExt as _;

    use crate::utils::{algos::zrip::sync, block_on};

    fn input() -> Vec<u8> {
        include_bytes!("./artifacts/lib.rs").to_vec()
    }

    fn compress(input: &[u8], level: Level, params: &[CParameter]) -> Vec<u8> {
        let mut encoder = ZripEncoder::with_quality_and_params(input, level, params);
        let mut output = Vec::new();
        block_on(encoder.read_to_end(&mut output)).unwrap();
        output
    }

    #[test]
    #[ntest::timeout(60000)]
    fn every_quality_roundtrips() {
        let input = input();
        for quality in [-131073, -8, -7, -1, 0, 1, 3, 4, 5, 19, 22] {
            let compressed = compress(&input, Level::Precise(quality), &[]);
            assert_eq!(sync::decompress(&compressed), input, "quality {quality}");
        }
        for level in [Level::Fastest, Level::Default, Level::Best] {
            let compressed = compress(&input, level, &[]);
            assert_eq!(sync::decompress(&compressed), input, "{level:?}");
        }
    }

    #[test]
    #[ntest::timeout(10000)]
    fn tuned_params_roundtrip() {
        let input = input();
        let params = [
            CParameter::window_log(20),
            CParameter::enable_long_distance_matching(true),
        ];
        let compressed = compress(&input, Level::Precise(3), &params);
        assert_eq!(sync::decompress(&compressed), input);

        let compressed = compress(
            &input,
            Level::Fastest,
            &[CParameter::enable_long_distance_matching(false)],
        );
        assert_eq!(sync::decompress(&compressed), input);
    }

    #[test]
    #[ntest::timeout(10000)]
    fn zero_selects_zstd_default_level() {
        // zstd takes level 0 as its default, 3, where zrip takes it as its own, 1.
        let input = input();
        let default = compress(&input, Level::Default, &[]);
        assert_eq!(compress(&input, Level::Precise(0), &[]), default);
        assert_eq!(compress(&input, Level::Precise(3), &[]), default);
        assert_ne!(compress(&input, Level::Precise(1), &[]), default);
    }
}

// The artifacts are text, so on Windows they may be checked out with CRLF line endings and no
// longer match the compressed artifacts, as in `zstd-dict.rs` and `zstd-window-size.rs`.
#[cfg(all(feature = "tokio", not(windows)))]
mod artifacts {
    use async_compression::{
        tokio::{
            bufread::{self, ZripEncoder},
            write::ZripDecoder,
        },
        zrip::DParameter,
        Level,
    };
    use std::io::Read as _;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    use crate::utils::block_on;

    const SOURCE: &[u8] = include_bytes!("./artifacts/lib.rs");
    const DICT: &[u8] = include_bytes!("./artifacts/dictionary-rust");
    const OTHER_DICT: &[u8] = include_bytes!("./artifacts/dictionary-rust-other");
    const COMPRESSED: &[u8] = include_bytes!("./artifacts/lib.rs.zst");

    #[tokio::test]
    async fn decode_no_dict() {
        let mut decoder = ZripDecoder::new(Vec::new());
        decoder.write_all(COMPRESSED).await.unwrap_err();
    }

    #[tokio::test]
    async fn decode_with_dict() {
        let mut decoder = ZripDecoder::with_dict(Vec::new(), DICT).unwrap();
        decoder.write_all(COMPRESSED).await.unwrap();
        decoder.shutdown().await.unwrap();

        assert_eq!(decoder.into_inner(), SOURCE);
    }

    #[tokio::test]
    async fn decode_with_wrong_dict() {
        let mut decoder = ZripDecoder::with_dict(Vec::new(), OTHER_DICT).unwrap();
        decoder.write_all(COMPRESSED).await.unwrap_err();
    }

    #[test]
    fn encode_with_dict() {
        for level in [Level::Fastest, Level::Default, Level::Precise(19)] {
            let mut encoder = ZripEncoder::with_dict(SOURCE, level, DICT).unwrap();
            let mut compressed = Vec::new();
            block_on(encoder.read_to_end(&mut compressed)).unwrap();

            // Both backends decode it with the dictionary.
            let mut decoded = Vec::new();
            libzstd::stream::read::Decoder::with_dictionary(compressed.as_slice(), DICT)
                .unwrap()
                .read_to_end(&mut decoded)
                .unwrap();
            assert_eq!(decoded, SOURCE, "{level:?}");

            block_on(async {
                let mut decoder = ZripDecoder::with_dict(Vec::new(), DICT).unwrap();
                decoder.write_all(&compressed).await.unwrap();
                decoder.shutdown().await.unwrap();
                assert_eq!(decoder.into_inner(), SOURCE, "{level:?}");

                let mut decoder = ZripDecoder::new(Vec::new());
                decoder.write_all(&compressed).await.unwrap_err();
            });
        }
    }

    #[test]
    #[ntest::timeout(10000)]
    fn window_log_max() {
        let compressed = include_bytes!("./artifacts/long-window-size-lib.rs.zst");

        let decompress = |params: &[DParameter]| {
            let mut decoder = bufread::ZripDecoder::with_params(&compressed[..], params);
            let mut output = Vec::new();
            block_on(decoder.read_to_end(&mut output)).map(|_| output)
        };

        // The default window limit and an explicitly small one are both too low.
        assert!(decompress(&[]).is_err());
        assert!(decompress(&[DParameter::window_log_max(16)]).is_err());
        assert_eq!(
            decompress(&[DParameter::window_log_max(31)]).unwrap(),
            SOURCE
        );
        // `0` restores the default limit.
        assert!(decompress(&[
            DParameter::window_log_max(31),
            DParameter::window_log_max(0)
        ])
        .is_err());
    }
}

/// Explicit interoperability with the `zstd` crate in both directions: streams written by
/// `ZripEncoder` must decode with `zstd`, and streams written by `zstd` must decode with
/// `ZripDecoder`.
#[cfg(feature = "tokio")]
mod zstd_interop {
    use async_compression::{
        tokio::bufread::{ZripDecoder, ZripEncoder},
        Level,
    };
    use tokio::io::AsyncReadExt as _;

    use crate::utils::block_on;

    pub(super) fn inputs() -> Vec<(&'static str, Vec<u8>)> {
        // Deterministic pseudo-random bytes, so the stream contains poorly compressible data.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let noise = (0..256 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect();

        vec![
            ("empty", Vec::new()),
            ("single byte", vec![42]),
            (
                "text",
                b"the quick brown fox jumps over the lazy dog\n".repeat(1024),
            ),
            ("noise", noise),
        ]
    }

    #[test]
    #[ntest::timeout(60000)]
    fn zrip_to_zstd() {
        for (name, input) in inputs() {
            for level in [-7, -1, 1, 3, 4, 19] {
                let mut compressed = Vec::new();
                block_on(
                    ZripEncoder::with_quality(input.as_slice(), Level::Precise(level))
                        .read_to_end(&mut compressed),
                )
                .unwrap();
                assert_eq!(
                    libzstd::decode_all(compressed.as_slice()).unwrap(),
                    input,
                    "{name}, level {level}"
                );
            }
        }
    }

    #[test]
    #[ntest::timeout(60000)]
    fn zstd_to_zrip() {
        for (name, input) in inputs() {
            for level in [-5, 1, 3, 7, 12, 19] {
                let compressed = libzstd::encode_all(input.as_slice(), level).unwrap();
                let mut output = Vec::new();
                block_on(ZripDecoder::new(compressed.as_slice()).read_to_end(&mut output)).unwrap();
                assert_eq!(output, input, "{name}, level {level}");
            }
        }
    }
}

#[cfg(feature = "tokio")]
mod roundtrip {
    use async_compression::{
        tokio::{
            bufread::{ZripDecoder, ZripEncoder},
            write,
        },
        zrip::{CParameter, DParameter},
        Level,
    };
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, BufReader};

    use super::zstd_interop::inputs;
    use crate::utils::block_on;

    const DICT: &[u8] = include_bytes!("./artifacts/dictionary-rust");
    const SOURCE: &[u8] = include_bytes!("./artifacts/lib.rs");

    const LEVELS: [Level; 9] = [
        Level::Fastest,
        Level::Default,
        Level::Best,
        Level::Precise(-7),
        Level::Precise(-5),
        Level::Precise(1),
        Level::Precise(3),
        Level::Precise(4),
        Level::Precise(19),
    ];

    fn compress(input: &[u8], level: Level, params: &[CParameter]) -> Vec<u8> {
        let mut output = Vec::new();
        block_on(
            ZripEncoder::with_quality_and_params(input, level, params).read_to_end(&mut output),
        )
        .unwrap();
        output
    }

    fn decompress(input: &[u8], params: &[DParameter]) -> Vec<u8> {
        let mut output = Vec::new();
        block_on(ZripDecoder::with_params(input, params).read_to_end(&mut output)).unwrap();
        output
    }

    #[test]
    #[ntest::timeout(60000)]
    fn levels() {
        for (name, input) in inputs() {
            for level in LEVELS {
                let compressed = compress(&input, level, &[]);
                assert_eq!(decompress(&compressed, &[]), input, "{name}, {level:?}");
            }
        }
    }

    #[test]
    #[ntest::timeout(60000)]
    fn params() {
        let ldm = [
            CParameter::window_log(24),
            CParameter::enable_long_distance_matching(true),
        ];
        let cases: [(&str, &[CParameter], &[DParameter]); 3] = [
            ("window", &[CParameter::window_log(20)], &[]),
            ("ldm", &ldm, &[DParameter::window_log_max(24)]),
            (
                "ldm disabled",
                &[CParameter::enable_long_distance_matching(false)],
                &[],
            ),
        ];

        for (name, input) in inputs() {
            for (params_name, cparams, dparams) in cases {
                for level in [Level::Fastest, Level::Precise(3), Level::Best] {
                    let compressed = compress(&input, level, cparams);
                    assert_eq!(
                        decompress(&compressed, dparams),
                        input,
                        "{name}, {params_name}, {level:?}"
                    );
                }
            }
        }
    }

    #[test]
    #[ntest::timeout(60000)]
    fn dictionary() {
        for level in LEVELS {
            let mut compressed = Vec::new();
            block_on(
                ZripEncoder::with_dict(SOURCE, level, DICT)
                    .unwrap()
                    .read_to_end(&mut compressed),
            )
            .unwrap();

            let mut decoded = Vec::new();
            block_on(
                ZripDecoder::with_dict(compressed.as_slice(), DICT)
                    .unwrap()
                    .read_to_end(&mut decoded),
            )
            .unwrap();
            assert_eq!(decoded, SOURCE, "{level:?}");
        }
    }

    #[test]
    #[ntest::timeout(60000)]
    fn multiple_members() {
        let members: Vec<_> = inputs().into_iter().map(|(_, input)| input).collect();
        let compressed: Vec<u8> = members
            .iter()
            .zip(LEVELS)
            .flat_map(|(input, level)| compress(input, level, &[]))
            .collect();

        let mut decoder = ZripDecoder::new(compressed.as_slice());
        decoder.multiple_members(true);
        let mut decoded = Vec::new();
        block_on(decoder.read_to_end(&mut decoded)).unwrap();
        assert_eq!(decoded, members.concat());
    }

    /// Both sides see the data one byte at a time, so every state is suspended and resumed.
    #[test]
    #[ntest::timeout(60000)]
    fn byte_by_byte() {
        let input = SOURCE;
        for level in [Level::Fastest, Level::Best] {
            let compressed = block_on(async {
                let mut encoder = write::ZripEncoder::with_quality(Vec::new(), level);
                for byte in input {
                    encoder.write_all(std::slice::from_ref(byte)).await.unwrap();
                }
                encoder.shutdown().await.unwrap();
                encoder.into_inner()
            });

            let mut decoder = ZripDecoder::new(BufReader::with_capacity(1, compressed.as_slice()));
            let mut decoded = Vec::new();
            let mut byte = [0];
            while block_on(decoder.read(&mut byte)).unwrap() == 1 {
                decoded.push(byte[0]);
            }
            assert_eq!(decoded, input, "{level:?}");
        }
    }
}

#[cfg(any(feature = "tokio", feature = "futures-io"))]
mod pending {
    use std::{io, task::Poll};

    // Repeated polling while waiting for input must not make the decoder fail or lose data.
    fn chunks(
        input: &[u8],
        chunk_size: usize,
    ) -> impl ::futures::Stream<Item = io::Result<Vec<u8>>> + '_ {
        let mut chunks = input.chunks(chunk_size);
        let mut pending = 32;
        ::futures::stream::poll_fn(move |cx| {
            if pending > 0 {
                pending -= 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            pending = 32;
            Poll::Ready(chunks.next().map(|chunk| Ok(chunk.to_vec())))
        })
    }

    macro_rules! pending_tests {
        ($runtime:ident, $feature:literal, |$stream:ident| $reader:block) => {
            #[cfg(feature = $feature)]
            mod $runtime {
                use super::chunks;
                use ::$runtime::io::AsyncReadExt as _;
                use async_compression::$runtime::bufread::ZripDecoder;
                use std::io;

                async fn decode(
                    input: &[u8],
                    chunk_size: usize,
                    output_size: usize,
                    multiple_members: bool,
                ) -> io::Result<Vec<u8>> {
                    let $stream = chunks(input, chunk_size);
                    let reader = $reader;
                    let mut decoder = ZripDecoder::new(reader);
                    decoder.multiple_members(multiple_members);
                    let mut output = Vec::new();
                    let mut buffer = vec![0; output_size];
                    loop {
                        let len = decoder.read(&mut buffer).await?;
                        if len == 0 {
                            return Ok(output);
                        }
                        output.extend_from_slice(&buffer[..len]);
                    }
                }

                #[test]
                #[ntest::timeout(10000)]
                fn repeated_pending() {
                    ::futures::executor::block_on(async {
                        let data: Vec<u8> = (0u32..4096)
                            .flat_map(|i| i.wrapping_mul(2654435761).to_le_bytes())
                            .collect();
                        for data in [vec![], vec![b'x'; 65536], data] {
                            let compressed = libzstd::encode_all(data.as_slice(), 0).unwrap();
                            for chunk_size in [1, 1024] {
                                for output_size in [1, 8192] {
                                    for multiple_members in [false, true] {
                                        let output = decode(
                                            &compressed,
                                            chunk_size,
                                            output_size,
                                            multiple_members,
                                        )
                                        .await
                                        .unwrap();
                                        assert_eq!(output, data);
                                    }
                                }
                            }
                        }
                    });
                }

                #[test]
                #[ntest::timeout(10000)]
                fn repeated_pending_between_members() {
                    ::futures::executor::block_on(async {
                        let members = [vec![b'a'; 65536], vec![], vec![b'b'; 65536]];
                        let compressed: Vec<u8> = members
                            .iter()
                            .flat_map(|data| libzstd::encode_all(data.as_slice(), 0).unwrap())
                            .collect();
                        let output = decode(&compressed, 1, 7, true).await.unwrap();
                        assert_eq!(output, members.concat());

                        // Without multiple members only the first frame is decoded.
                        let output = decode(&compressed, 1, 7, false).await.unwrap();
                        assert_eq!(output, members[0]);
                    });
                }

                #[test]
                #[ntest::timeout(10000)]
                fn repeated_pending_truncated_input() {
                    ::futures::executor::block_on(async {
                        let compressed = libzstd::encode_all(&[b'a'; 65536][..], 0).unwrap();
                        for end in [1, compressed.len() / 2, compressed.len() - 1] {
                            let error =
                                decode(&compressed[..end], 1, 8192, true).await.unwrap_err();
                            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
                        }
                    });
                }

                #[test]
                #[ntest::timeout(10000)]
                fn repeated_pending_invalid_input() {
                    ::futures::executor::block_on(async {
                        assert!(decode(b"not a zstd frame", 1, 8192, true).await.is_err());
                    });
                }
            }
        };
    }

    pending_tests!(tokio, "tokio", |stream| {
        use ::futures::TryStreamExt as _;
        tokio_util::io::StreamReader::new(stream.map_ok(std::io::Cursor::new))
    });
    pending_tests!(futures, "futures-io", |stream| {
        use ::futures::TryStreamExt as _;
        stream.into_async_read()
    });
}

/// Skippable frames carry user metadata that decoders skip. `ZripDecoder` must treat them as
/// `ZstdDecoder` does: each one is a complete frame, before, between or after Zstandard frames, or
/// on its own.
#[cfg(all(feature = "tokio", feature = "zstd"))]
mod skippable_frames {
    use std::convert::TryFrom;

    use async_compression::tokio::bufread::{ZripDecoder, ZstdDecoder};
    use tokio::io::AsyncReadExt as _;

    use crate::utils::block_on;

    fn skippable(payload: &[u8]) -> Vec<u8> {
        let mut frame = 0x184D_2A50_u32.to_le_bytes().to_vec();
        frame.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
        frame.extend_from_slice(payload);
        frame
    }

    fn frame(content: &[u8]) -> Vec<u8> {
        libzstd::encode_all(content, 3).unwrap()
    }

    fn cases() -> Vec<(&'static str, Vec<u8>)> {
        let meta = skippable(b"metadata");
        let empty_meta = skippable(b"");
        let hello = frame(b"hello ");
        let world = frame(b"world");
        vec![
            ("skippable only", meta.clone()),
            ("empty skippable only", empty_meta.clone()),
            (
                "two skippables",
                [meta.clone(), empty_meta.clone()].concat(),
            ),
            ("skippable, frame", [meta.clone(), hello.clone()].concat()),
            ("frame, skippable", [hello.clone(), meta.clone()].concat()),
            (
                "frame, skippable, frame",
                [hello.clone(), meta.clone(), world.clone()].concat(),
            ),
            (
                "skippable, frame, skippable, frame, skippable",
                [meta.clone(), hello, empty_meta, world, meta].concat(),
            ),
        ]
    }

    fn decode<R: tokio::io::AsyncRead + Unpin>(mut decoder: R) -> Result<Vec<u8>, String> {
        let mut output = Vec::new();
        block_on(decoder.read_to_end(&mut output))
            .map(|_| output)
            .map_err(|err| format!("{:?}", err.kind()))
    }

    fn check(multiple_members: bool) {
        let mismatches: Vec<_> = cases()
            .into_iter()
            .filter_map(|(name, input)| {
                let mut zstd = ZstdDecoder::new(input.as_slice());
                zstd.multiple_members(multiple_members);
                let mut zrip = ZripDecoder::new(input.as_slice());
                zrip.multiple_members(multiple_members);
                let (zstd, zrip) = (decode(zstd), decode(zrip));
                (zstd != zrip).then(|| format!("{name}: zstd {zstd:?}, zrip {zrip:?}"))
            })
            .collect();
        assert!(mismatches.is_empty(), "{:#?}", mismatches);
    }

    #[test]
    #[ntest::timeout(10000)]
    fn single_member() {
        check(false);
    }

    #[test]
    #[ntest::timeout(10000)]
    fn multiple_members() {
        check(true);
    }
}

/// A frame end that a flush drains still ends the stream: as with `ZstdDecoder`, writing a second
/// frame after it fails.
#[cfg(feature = "tokio")]
mod flush {
    use async_compression::tokio::write::ZripDecoder;
    use tokio::io::AsyncWriteExt as _;

    use crate::utils::block_on;

    #[test]
    #[ntest::timeout(10000)]
    fn frame_end_drained_by_flush() {
        // The output is still pending when the writer has taken all of the frame.
        let content = vec![b'a'; 100_000];
        let first = libzstd::encode_all(content.as_slice(), 3).unwrap();
        let second = libzstd::encode_all(&b"b"[..], 3).unwrap();
        block_on(async {
            let mut decoder = ZripDecoder::new(Vec::new());
            decoder.write_all(&first).await.unwrap();
            decoder.flush().await.unwrap();
            decoder.write_all(&second).await.unwrap_err();
            decoder.shutdown().await.unwrap();
            assert_eq!(decoder.into_inner(), content);
        });
    }
}
