#![allow(dead_code, unused_macros)] // Different tests use a different subset of functions

use bytes::Bytes;
use futures::{
    io::AsyncBufRead,
    stream::{self, Stream},
};
use futures_test::{io::AsyncReadTestExt, stream::StreamTestExt};
use proptest_derive::Arbitrary;
use std::io::{self, Cursor};

#[derive(Arbitrary, Debug)]
pub struct InputStream(Vec<Vec<u8>>);

impl InputStream {
    pub fn stream(&self) -> impl Stream<Item = io::Result<Bytes>> {
        // The resulting stream here will interleave empty chunks before and after each chunk, and
        // then interleave a `Poll::Pending` between each yielded chunk, that way we test the
        // handling of these two conditions in every point of the tested stream.
        stream::iter(
            self.0
                .clone()
                .into_iter()
                .map(Bytes::from)
                .flat_map(|bytes| vec![Bytes::new(), bytes])
                .chain(Some(Bytes::new()))
                .map(Ok),
        )
        .interleave_pending()
    }

    pub fn reader(&self) -> impl AsyncBufRead {
        // TODO: By using the stream here we ensure that each chunk will require a separate
        // read/poll_fill_buf call to process to help test reading multiple chunks. This is
        // blocked on fixing AsyncBufRead for IntoAsyncRead:
        // (https://github.com/rust-lang-nursery/futures-rs/pull/1595)
        Cursor::new(self.bytes()).interleave_pending()
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.0.iter().flatten().cloned().collect()
    }
}

// This happens to be the only dimension we're using
impl From<[[u8; 3]; 2]> for InputStream {
    fn from(input: [[u8; 3]; 2]) -> InputStream {
        InputStream(vec![Vec::from(&input[0][..]), Vec::from(&input[1][..])])
    }
}
impl From<Vec<Vec<u8>>> for InputStream {
    fn from(input: Vec<Vec<u8>>) -> InputStream {
        InputStream(input)
    }
}

mod prelude {
    pub use bytes::Bytes;
    pub use futures::{
        executor::{block_on, block_on_stream},
        io::{AsyncBufRead, AsyncRead, AsyncReadExt},
        stream::{self, Stream, TryStreamExt},
    };
    pub use futures_test::{io::AsyncReadTestExt, stream::StreamTestExt};
    pub use pin_utils::pin_mut;
    pub use std::io::{self, Read};

    pub fn read_to_vec(mut read: impl Read) -> Vec<u8> {
        let mut output = vec![];
        read.read_to_end(&mut output).unwrap();
        output
    }

    pub fn async_read_to_vec(read: impl AsyncRead) -> Vec<u8> {
        let mut output = vec![];
        pin_mut!(read);
        block_on(read.read_to_end(&mut output)).unwrap();
        output
    }

    pub fn stream_to_vec(stream: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
        pin_mut!(stream);
        block_on_stream(stream)
            .map(Result::unwrap)
            .flatten()
            .collect()
    }
}

pub mod brotli {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use brotli2::bufread::BrotliEncoder;
            read_to_vec(BrotliEncoder::new(bytes, 1))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use brotli2::bufread::BrotliDecoder;
            read_to_vec(BrotliDecoder::new(bytes))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::BrotliEncoder;
            pin_mut!(input);
            stream_to_vec(BrotliEncoder::new(input, 1))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::BrotliDecoder;
            pin_mut!(input);
            stream_to_vec(BrotliDecoder::new(input))
        }
    }
}

pub mod deflate {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use flate2::{bufread::DeflateEncoder, Compression};
            read_to_vec(DeflateEncoder::new(bytes, Compression::fast()))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use flate2::bufread::DeflateDecoder;
            read_to_vec(DeflateDecoder::new(bytes))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::{flate2::Compression, stream::DeflateEncoder};
            pin_mut!(input);
            stream_to_vec(DeflateEncoder::new(input, Compression::fast()))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::DeflateDecoder;
            pin_mut!(input);
            stream_to_vec(DeflateDecoder::new(input))
        }
    }

    pub mod bufread {
        use crate::utils::prelude::*;

        pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
            use async_compression::{bufread::DeflateEncoder, flate2::Compression};
            pin_mut!(input);
            async_read_to_vec(DeflateEncoder::new(input, Compression::fast()))
        }

        pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
            use async_compression::bufread::DeflateDecoder;
            pin_mut!(input);
            async_read_to_vec(DeflateDecoder::new(input))
        }
    }
}

pub mod zlib {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use flate2::{bufread::ZlibEncoder, Compression};
            read_to_vec(ZlibEncoder::new(bytes, Compression::fast()))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use flate2::bufread::ZlibDecoder;
            read_to_vec(ZlibDecoder::new(bytes))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::{flate2::Compression, stream::ZlibEncoder};
            pin_mut!(input);
            stream_to_vec(ZlibEncoder::new(input, Compression::fast()))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::ZlibDecoder;
            pin_mut!(input);
            stream_to_vec(ZlibDecoder::new(input))
        }
    }

    pub mod bufread {
        use crate::utils::prelude::*;

        pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
            use async_compression::{bufread::ZlibEncoder, flate2::Compression};
            pin_mut!(input);
            async_read_to_vec(ZlibEncoder::new(input, Compression::fast()))
        }

        pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
            use async_compression::bufread::ZlibDecoder;
            pin_mut!(input);
            async_read_to_vec(ZlibDecoder::new(input))
        }
    }
}

pub mod gzip {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use flate2::{bufread::GzEncoder, Compression};
            read_to_vec(GzEncoder::new(bytes, Compression::fast()))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use flate2::bufread::GzDecoder;
            read_to_vec(GzDecoder::new(bytes))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::{flate2::Compression, stream::GzipEncoder};
            pin_mut!(input);
            stream_to_vec(GzipEncoder::new(input, Compression::fast()))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::GzipDecoder;
            pin_mut!(input);
            stream_to_vec(GzipDecoder::new(input))
        }
    }
}

pub mod zstd {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use libzstd::stream::read::Encoder;
            use libzstd::DEFAULT_COMPRESSION_LEVEL;
            read_to_vec(Encoder::new(bytes, DEFAULT_COMPRESSION_LEVEL).unwrap())
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use libzstd::stream::read::Decoder;
            read_to_vec(Decoder::new(bytes).unwrap())
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::ZstdEncoder;
            pin_mut!(input);
            stream_to_vec(ZstdEncoder::new(input, 0))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::ZstdDecoder;
            pin_mut!(input);
            stream_to_vec(ZstdDecoder::new(input))
        }
    }
}

macro_rules! test_cases {
    (@ [ $variant:ident :: stream :: compress ]) => {
        mod compress {
            use crate::utils;
            use std::iter::FromIterator;

            #[test]
            fn empty() {
                // Can't use InputStream for this as it will inject extra empty chunks
                let compressed = utils::$variant::stream::compress(futures::stream::empty());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, &[][..]);
            }

            #[test]
            fn empty_chunk() {
                let input = utils::InputStream::from(vec![vec![]]);

                let compressed = utils::$variant::stream::compress(input.stream());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, input.bytes());
            }

            #[test]
            fn short() {
                let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

                let compressed = utils::$variant::stream::compress(input.stream());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
            }

            #[test]
            fn long() {
                let input = vec![
                    Vec::from_iter((0..20_000).map(|_| rand::random())),
                    Vec::from_iter((0..20_000).map(|_| rand::random())),
                ];
                let input = utils::InputStream::from(input);

                let compressed = utils::$variant::stream::compress(input.stream());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, input.bytes());
            }
        }
    };

    (@ [ $variant:ident :: stream :: decompress ]) => {
        mod decompress {
            use crate::utils;
            use std::iter::FromIterator;

            #[test]
            fn empty() {
                let compressed = utils::$variant::sync::compress(&[]);

                let stream = utils::InputStream::from(vec![compressed]);
                let output = utils::$variant::stream::decompress(stream.stream());

                assert_eq!(output, &[][..]);
            }

            #[test]
            fn short() {
                let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                let stream = utils::InputStream::from(vec![compressed]);
                let output = utils::$variant::stream::decompress(stream.stream());

                assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
            }

            #[test]
            fn long() {
                let input = Vec::from_iter((0..20_000).map(|_| rand::random()));
                let compressed = utils::$variant::sync::compress(&input);

                let stream = utils::InputStream::from(vec![compressed]);
                let output = utils::$variant::stream::decompress(stream.stream());

                assert_eq!(output, input);
            }
        }
    };

    (@ [ $variant:ident :: bufread :: compress ]) => {
        mod compress {
            use crate::utils;
            use std::iter::FromIterator;

            #[test]
            fn empty() {
                let mut input: &[u8] = &[];
                let compressed = utils::$variant::bufread::compress(&mut input);
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, &[][..]);
            }

            #[test]
            fn empty_chunk() {
                let input = utils::InputStream::from(vec![vec![]]);

                let compressed = utils::$variant::bufread::compress(input.reader());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, input.bytes());
            }

            #[test]
            fn short() {
                let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

                let compressed = utils::$variant::bufread::compress(input.reader());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
            }

            #[test]
            fn long() {
                let input = vec![
                    Vec::from_iter((0..20_000).map(|_| rand::random())),
                    Vec::from_iter((0..20_000).map(|_| rand::random())),
                ];
                let input = utils::InputStream::from(input);

                let compressed = utils::$variant::bufread::compress(input.reader());
                let output = utils::$variant::sync::decompress(&compressed);

                assert_eq!(output, input.bytes());
            }
        }
    };

    (@ [ $variant:ident :: bufread :: decompress ]) => {
        mod decompress {
            use crate::utils;
            use std::iter::FromIterator;

            #[test]
            fn empty() {
                let compressed = utils::$variant::sync::compress(&[]);

                let stream = utils::InputStream::from(vec![compressed]);
                let output = utils::$variant::bufread::decompress(stream.reader());

                assert_eq!(output, &[][..]);
            }

            #[test]
            fn short() {
                let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                let stream = utils::InputStream::from(vec![compressed]);
                let output = utils::$variant::bufread::decompress(stream.reader());

                assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
            }

            #[test]
            fn long() {
                let input = Vec::from_iter((0..20_000).map(|_| rand::random()));
                let compressed = utils::$variant::sync::compress(&input);

                let stream = utils::InputStream::from(vec![compressed]);
                let output = utils::$variant::bufread::decompress(stream.reader());

                assert_eq!(output, input);
            }
        }
    };

    (@ [ $variant:ident :: $io:ident :: $sub:ident ]) => {
        compile_error!(concat!("Unknown test cases ", stringify!($variant::$io::$sub)));
    };

    (@ [ $variant:ident :: $io:ident ] :: $sub:ident) => {
        test_cases!(@ [ $variant :: $io :: $sub ]);
    };

    (@ [ $variant:ident :: $io:ident ] :: { $($sub:tt),* $(,)? }) => {
        $(test_cases!(@ [ $variant :: $io ] :: $sub);)+
    };

    (@ [ $variant:ident ] :: $io:ident :: $($rest:tt)+) => {
        mod $io {
            test_cases!(@ [ $variant :: $io ] :: $($rest)+);
        }
    };

    (@ [ $variant:ident ] :: { $($io:ident :: $sub:tt),* $(,)? }) => {
        $(test_cases!(@ [ $variant ] :: $io :: $sub );)+
    };

    ($variant:ident :: $($rest:tt)+) => {
        mod $variant {
            test_cases!(@ [ $variant ] :: $($rest)+);
        }
    };
}
