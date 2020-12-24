macro_rules! io_test_cases {
    ($impl:ident, $variant:ident) => {
        mod $impl {
            mod bufread {
                mod compress {
                    use crate::utils::{
                        algos::$variant::{
                            sync,
                            $impl::{bufread, read},
                        },
                        FromIterator, InputStream, Level,
                    };

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        let mut input: &[u8] = &[];
                        let compressed = bufread::compress(&mut input);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn to_full_output() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);
                        let mut output = [];

                        let encoder = bufread::Encoder::new(bufread::from(&input));
                        let result = read::poll_read(encoder, &mut output);
                        assert!(matches!(result, Ok(0)));
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty_chunk() {
                        let input = InputStream::from(vec![vec![]]);

                        let compressed = bufread::compress(bufread::from(&input));
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = bufread::compress(bufread::from(&input));
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let input = InputStream::from(vec![
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                        ]);

                        let compressed = bufread::compress(bufread::from(&input));
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    fn with_level_best() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder =
                            bufread::Encoder::with_quality(bufread::from(&input), Level::Best);
                        let compressed = read::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_default() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder = bufread::Encoder::new(bufread::from(&input));
                        let compressed = read::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_0() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder = bufread::Encoder::with_quality(
                            bufread::from(&input),
                            Level::Precise(0),
                        );
                        let compressed = read::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_max() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder = bufread::Encoder::with_quality(
                            bufread::from(&input),
                            Level::Precise(u32::max_value()),
                        );
                        let compressed = read::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }
                }

                mod decompress {
                    use crate::utils::{
                        algos::$variant::{
                            sync,
                            $impl::{bufread, read},
                        },
                        FromIterator, InputStream,
                    };

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        let compressed = sync::compress(&[]);

                        let input = InputStream::from(vec![compressed]);
                        let output = bufread::decompress(bufread::from(&input));

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn to_full_output() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);
                        let mut output = [];

                        let decoder = bufread::Decoder::new(bufread::from(&input));
                        let result = read::poll_read(decoder, &mut output);
                        assert!(matches!(result, Ok(0)));
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn zeros() {
                        let compressed = sync::compress(&[0; 10]);

                        let input = InputStream::from(vec![compressed]);
                        let output = bufread::decompress(bufread::from(&input));

                        assert_eq!(output, &[0; 10][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let input = InputStream::from(vec![compressed]);
                        let output = bufread::decompress(bufread::from(&input));

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short_chunks() {
                        let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let input =
                            InputStream::from(Vec::from_iter(compressed.chunks(2).map(Vec::from)));
                        let output = bufread::decompress(bufread::from(&input));

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn trailer() {
                        let mut compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        compressed.extend_from_slice(&[7, 8, 9, 10]);

                        let input = InputStream::from(vec![compressed]);
                        let mut reader = bufread::from(&input);
                        let output = bufread::decompress(&mut reader);
                        let trailer = read::to_vec(reader);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        assert_eq!(trailer, &[7, 8, 9, 10][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let bytes = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = sync::compress(&bytes);

                        let input = InputStream::from(vec![compressed]);
                        let output = bufread::decompress(bufread::from(&input));

                        assert_eq!(output, bytes);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long_chunks() {
                        let bytes = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = sync::compress(&bytes);

                        let input = InputStream::from(Vec::from_iter(
                            compressed.chunks(1024).map(Vec::from),
                        ));
                        let output = bufread::decompress(bufread::from(&input));

                        assert_eq!(output, bytes);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn multiple_members() {
                        let compressed = [
                            sync::compress(&[1, 2, 3, 4, 5, 6]),
                            sync::compress(&[6, 5, 4, 3, 2, 1]),
                        ]
                        .join(&[][..]);

                        let input = InputStream::from(vec![compressed]);

                        let mut decoder = bufread::Decoder::new(bufread::from(&input));
                        decoder.multiple_members(true);
                        let output = read::to_vec(decoder);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..]);
                    }
                }
            }

            mod write {
                mod compress {
                    use crate::utils::{
                        algos::$variant::{sync, $impl::write},
                        FromIterator, InputStream, Level,
                    };

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        let input = InputStream::from(vec![]);

                        let compressed = write::compress(input.as_ref(), 65_536);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty_chunk() {
                        let input = InputStream::from(vec![vec![]]);

                        let compressed = write::compress(input.as_ref(), 65_536);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = write::compress(input.as_ref(), 65_536);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short_chunk_output() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = write::compress(input.as_ref(), 2);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let input = InputStream::from(vec![
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                        ]);

                        let compressed = write::compress(input.as_ref(), 65_536);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long_chunk_output() {
                        let input = InputStream::from(vec![
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                        ]);

                        let compressed = write::compress(input.as_ref(), 20);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    fn with_level_best() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = write::to_vec(
                            input.as_ref(),
                            |input| Box::pin(write::Encoder::with_quality(input, Level::Best)),
                            65_536,
                        );
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_default() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = write::to_vec(
                            input.as_ref(),
                            |input| Box::pin(write::Encoder::new(input)),
                            65_536,
                        );
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_0() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = write::to_vec(
                            input.as_ref(),
                            |input| {
                                Box::pin(write::Encoder::with_quality(input, Level::Precise(0)))
                            },
                            65_536,
                        );
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_max() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = write::to_vec(
                            input.as_ref(),
                            |input| {
                                Box::pin(write::Encoder::with_quality(
                                    input,
                                    Level::Precise(u32::max_value()),
                                ))
                            },
                            65_536,
                        );
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }
                }

                mod decompress {
                    use crate::utils::{
                        algos::$variant::{sync, $impl::write},
                        FromIterator, InputStream,
                    };

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        let compressed = sync::compress(&[]);

                        let input = InputStream::from(vec![compressed]);
                        let output = write::decompress(input.as_ref(), 65_536);

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn zeros() {
                        let compressed = sync::compress(&[0; 10]);

                        let input = InputStream::from(vec![compressed]);
                        let output = write::decompress(input.as_ref(), 65_536);

                        assert_eq!(output, &[0; 10][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let input = InputStream::from(vec![compressed]);
                        let output = write::decompress(input.as_ref(), 65_536);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short_chunks() {
                        let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let input =
                            InputStream::from(Vec::from_iter(compressed.chunks(2).map(Vec::from)));
                        let output = write::decompress(input.as_ref(), 65_536);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let bytes = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = sync::compress(&bytes);

                        let input = InputStream::from(vec![compressed]);
                        let output = write::decompress(input.as_ref(), 65_536);

                        assert_eq!(output, bytes);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long_chunks() {
                        let bytes = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = sync::compress(&bytes);

                        let input = InputStream::from(Vec::from_iter(
                            compressed.chunks(1024).map(Vec::from),
                        ));
                        let output = write::decompress(input.as_ref(), 65_536);

                        assert_eq!(output, bytes);
                    }
                }
            }
        }
    };
}

macro_rules! test_cases {
    ($variant:ident) => {
        mod $variant {
            #[cfg(feature = "stream")]
            mod stream {
                mod compress {
                    use crate::utils::{
                        algos::$variant::{stream, sync},
                        block_on, FromIterator, InputStream, Level,
                    };
                    use futures::stream::StreamExt as _;

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        // Can't use InputStream for this as it will inject extra empty chunks
                        let compressed = stream::compress(futures::stream::empty());
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty_chunk() {
                        let input = InputStream::from(vec![vec![]]);

                        let compressed = stream::compress(input.bytes_05_stream());
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = stream::compress(input.bytes_05_stream());
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let input = InputStream::from(vec![
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                        ]);

                        let compressed = stream::compress(input.bytes_05_stream());
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn error() {
                        let err = std::io::Error::new(std::io::ErrorKind::Other, "failure");
                        let input = futures::stream::iter(vec![Err(err)]);

                        let mut stream = stream::Encoder::with_quality(input, Level::Fastest);

                        assert!(block_on(stream.next()).unwrap().is_err());
                        assert!(block_on(stream.next()).is_none());
                    }

                    #[test]
                    fn with_level_best() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder =
                            stream::Encoder::with_quality(input.bytes_05_stream(), Level::Best);
                        let compressed = stream::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_default() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder = stream::Encoder::new(input.bytes_05_stream());
                        let compressed = stream::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_0() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder = stream::Encoder::with_quality(
                            input.bytes_05_stream(),
                            Level::Precise(0),
                        );
                        let compressed = stream::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    fn with_level_max() {
                        let input = InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let encoder = stream::Encoder::with_quality(
                            input.bytes_05_stream(),
                            Level::Precise(u32::max_value()),
                        );
                        let compressed = stream::to_vec(encoder);
                        let output = sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }
                }

                mod decompress {
                    use crate::utils::{
                        algos::$variant::{stream, sync},
                        block_on, FromIterator, InputStream,
                    };
                    use bytes_05::Bytes;
                    use futures::stream::StreamExt as _;

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        let compressed = sync::compress(&[]);

                        let input = InputStream::from(vec![compressed]);
                        let output = stream::decompress(input.bytes_05_stream());

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let input = InputStream::from(vec![compressed]);
                        let output = stream::decompress(input.bytes_05_stream());

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let bytes = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = sync::compress(&bytes);

                        let input = InputStream::from(vec![compressed]);
                        let output = stream::decompress(input.bytes_05_stream());

                        assert_eq!(output, bytes);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long_chunks() {
                        let bytes = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = sync::compress(&bytes);

                        let input = InputStream::from(Vec::from_iter(
                            compressed.chunks(1024).map(Vec::from),
                        ));
                        let output = stream::decompress(input.bytes_05_stream());

                        assert_eq!(output, bytes);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn trailer() {
                        // Currently there is no way to get any partially consumed stream item from
                        // the decoder, for now we just guarantee that if the compressed frame
                        // exactly matches an item boundary we will not read the next item from the
                        // stream.
                        let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let input = InputStream::from(vec![compressed, vec![7, 8, 9, 10]]);

                        let mut stream = input.bytes_05_stream();
                        let output = stream::decompress(&mut stream);
                        let trailer = stream::to_vec(stream);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        assert_eq!(trailer, &[7, 8, 9, 10][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn multiple_members() {
                        let compressed = [
                            sync::compress(&[1, 2, 3, 4, 5, 6]),
                            sync::compress(&[6, 5, 4, 3, 2, 1]),
                        ]
                        .join(&[][..]);

                        let input = InputStream::from(vec![compressed]);

                        let mut decoder = stream::Decoder::new(input.bytes_05_stream());
                        decoder.multiple_members(true);
                        let output = stream::to_vec(decoder);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn multiple_members_chunked() {
                        let compressed = [
                            sync::compress(&[1, 2, 3, 4, 5, 6]),
                            sync::compress(&[6, 5, 4, 3, 2, 1]),
                        ]
                        .join(&[][..]);

                        let input =
                            InputStream::from(Vec::from_iter(compressed.chunks(1).map(Vec::from)));

                        let mut decoder = stream::Decoder::new(input.bytes_05_stream());
                        decoder.multiple_members(true);
                        let output = stream::to_vec(decoder);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn error() {
                        let err = std::io::Error::new(std::io::ErrorKind::Other, "failure");
                        let input = futures::stream::iter(vec![Err(err)]);

                        let mut stream = stream::Decoder::new(input);

                        assert!(block_on(stream.next()).unwrap().is_err());
                        assert!(block_on(stream.next()).is_none());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn invalid_data() {
                        let input =
                            futures::stream::iter(vec![Ok(Bytes::from(&[1, 2, 3, 4, 5, 6][..]))]);

                        let mut stream = stream::Decoder::new(input);

                        assert!(block_on(stream.next()).unwrap().is_err());
                        assert!(block_on(stream.next()).is_none());
                    }
                }
            }

            #[cfg(feature = "futures-io")]
            io_test_cases!(futures, $variant);

            #[cfg(feature = "tokio-02")]
            io_test_cases!(tokio_02, $variant);

            #[cfg(feature = "tokio-03")]
            io_test_cases!(tokio_03, $variant);
        }
    };
}
