#![allow(dead_code, unused_macros)] // Different tests use a different subset of functions

use bytes::Bytes;
use futures::{
    io::AsyncBufRead,
    stream::{self, Stream, TryStreamExt},
};
use futures_test::stream::StreamTestExt;
use proptest_derive::Arbitrary;
use std::io;

#[derive(Arbitrary, Debug, Clone)]
pub struct InputStream(Vec<Vec<u8>>);

impl InputStream {
    pub fn as_ref(&self) -> &[Vec<u8>] {
        &self.0
    }

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
        // read/poll_fill_buf call to process to help test reading multiple chunks.
        self.stream().into_async_read()
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.0.iter().flatten().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(Vec::len).sum()
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

pub mod prelude {
    pub use bytes::Bytes;
    pub use futures::{
        executor::{block_on, block_on_stream},
        io::{
            copy_buf, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite,
            AsyncWriteExt, BufReader, Cursor,
        },
        pin_mut,
        stream::{self, Stream, TryStreamExt},
    };
    pub use futures_test::{
        io::{AsyncReadTestExt, AsyncWriteTestExt},
        stream::StreamTestExt,
    };
    pub use std::{
        io::{self, Read},
        pin::Pin,
    };

    pub fn read_to_vec(mut read: impl Read) -> Vec<u8> {
        let mut output = vec![];
        read.read_to_end(&mut output).unwrap();
        output
    }

    pub fn async_read_to_vec(read: impl AsyncRead) -> Vec<u8> {
        // TODO: https://github.com/rust-lang-nursery/futures-rs/issues/1510
        // All current test cases are < 100kB
        let mut output = Cursor::new(vec![0; 102_400]);
        pin_mut!(read);
        let len = block_on(copy_buf(BufReader::with_capacity(2, read), &mut output)).unwrap();
        let mut output = output.into_inner();
        output.truncate(len as usize);
        output
    }

    pub fn async_write_to_vec(
        input: &[Vec<u8>],
        create_writer: impl for<'a> FnOnce(
            &'a mut (dyn AsyncWrite + Unpin),
        ) -> Pin<Box<dyn AsyncWrite + 'a>>,
        limit: usize,
    ) -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut test_writer = (&mut output)
                .limited_write(limit)
                .interleave_pending_write();
            let mut writer = create_writer(&mut test_writer);
            for chunk in input {
                block_on(writer.write_all(chunk)).unwrap();
                block_on(writer.flush()).unwrap();
            }
            block_on(writer.close()).unwrap();
        }
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
            use brotli::{enc::backward_references::BrotliEncoderParams, CompressorReader};
            let mut params = BrotliEncoderParams::default();
            params.quality = 1;
            read_to_vec(CompressorReader::with_params(bytes, 0, &params))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use brotli::Decompressor;
            read_to_vec(Decompressor::new(bytes, 0))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;
        pub use async_compression::stream::{BrotliDecoder as Decoder, BrotliEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            use async_compression::stream::BrotliDecoder;
            pin_mut!(input);
            stream_to_vec(BrotliDecoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;
            pub use async_compression::futures::bufread::BrotliDecoder as Decoder;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::BrotliEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(BrotliEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                pin_mut!(input);
                async_read_to_vec(Decoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::BrotliEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(BrotliEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::BrotliDecoder;
                async_write_to_vec(input, |input| Box::pin(BrotliDecoder::new(input)), limit)
            }
        }
    }
}

pub mod bzip2 {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use bzip2::{bufread::BzEncoder, Compression};
            read_to_vec(BzEncoder::new(bytes, Compression::Fastest))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use bzip2::bufread::BzDecoder;
            read_to_vec(BzDecoder::new(bytes))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;
        pub use async_compression::stream::{BzDecoder as Decoder, BzEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;
            pub use async_compression::futures::bufread::BzDecoder as Decoder;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::BzEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(BzEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                pin_mut!(input);
                async_read_to_vec(Decoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::BzEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(BzEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::BzDecoder;
                async_write_to_vec(input, |input| Box::pin(BzDecoder::new(input)), limit)
            }
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
        pub use async_compression::stream::{DeflateDecoder as Decoder, DeflateEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;
            pub use async_compression::futures::bufread::DeflateDecoder as Decoder;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::DeflateEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(DeflateEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                pin_mut!(input);
                async_read_to_vec(Decoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::DeflateEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(DeflateEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::DeflateDecoder;
                async_write_to_vec(input, |input| Box::pin(DeflateDecoder::new(input)), limit)
            }
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
        pub use async_compression::stream::{ZlibDecoder as Decoder, ZlibEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;
            pub use async_compression::futures::bufread::ZlibDecoder as Decoder;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::ZlibEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(ZlibEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                pin_mut!(input);
                async_read_to_vec(Decoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::ZlibEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(ZlibEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::ZlibDecoder;
                async_write_to_vec(input, |input| Box::pin(ZlibDecoder::new(input)), limit)
            }
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
        pub use async_compression::stream::{GzipDecoder as Decoder, GzipEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;
            pub use async_compression::futures::bufread::GzipDecoder as Decoder;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::GzipEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(GzipEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                pin_mut!(input);
                async_read_to_vec(Decoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::GzipEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(GzipEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::GzipDecoder;
                async_write_to_vec(input, |input| Box::pin(GzipDecoder::new(input)), limit)
            }
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
        pub use async_compression::stream::{ZstdDecoder as Decoder, ZstdEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;
            pub use async_compression::futures::bufread::ZstdDecoder as Decoder;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::ZstdEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(ZstdEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                pin_mut!(input);
                async_read_to_vec(Decoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::ZstdEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(ZstdEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::ZstdDecoder;
                async_write_to_vec(input, |input| Box::pin(ZstdDecoder::new(input)), limit)
            }
        }
    }
}

pub mod xz {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use xz2::bufread::XzEncoder;

            read_to_vec(XzEncoder::new(bytes, 0))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use xz2::bufread::XzDecoder;

            read_to_vec(XzDecoder::new(bytes))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;
        pub use async_compression::stream::{XzDecoder as Decoder, XzEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::XzEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(XzEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::futures::bufread::XzDecoder;
                pin_mut!(input);
                async_read_to_vec(XzDecoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::XzEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(XzEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::XzDecoder;
                async_write_to_vec(input, |input| Box::pin(XzDecoder::new(input)), limit)
            }
        }
    }
}

pub mod lzma {
    pub mod sync {
        use crate::utils::prelude::*;

        pub fn compress(bytes: &[u8]) -> Vec<u8> {
            use xz2::bufread::XzEncoder;
            use xz2::stream::{LzmaOptions, Stream};

            read_to_vec(XzEncoder::new_stream(
                bytes,
                Stream::new_lzma_encoder(&LzmaOptions::new_preset(0).unwrap()).unwrap(),
            ))
        }

        pub fn decompress(bytes: &[u8]) -> Vec<u8> {
            use xz2::bufread::XzDecoder;
            use xz2::stream::Stream;

            read_to_vec(XzDecoder::new_stream(
                bytes,
                Stream::new_lzma_decoder(u64::max_value()).unwrap(),
            ))
        }
    }

    pub mod stream {
        use crate::utils::prelude::*;
        pub use async_compression::stream::{LzmaDecoder as Decoder, LzmaEncoder as Encoder};

        pub fn compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Encoder::with_quality(
                input,
                async_compression::Level::Fastest,
            ))
        }

        pub fn decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
            pin_mut!(input);
            stream_to_vec(Decoder::new(input))
        }
    }

    pub mod futures {
        pub mod bufread {
            use crate::utils::prelude::*;

            pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::{futures::bufread::LzmaEncoder, Level};
                pin_mut!(input);
                async_read_to_vec(LzmaEncoder::with_quality(input, Level::Fastest))
            }

            pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                use async_compression::futures::bufread::LzmaDecoder;
                pin_mut!(input);
                async_read_to_vec(LzmaDecoder::new(input))
            }
        }

        pub mod write {
            use crate::utils::prelude::*;

            pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::{futures::write::LzmaEncoder, Level};
                async_write_to_vec(
                    input,
                    |input| Box::pin(LzmaEncoder::with_quality(input, Level::Fastest)),
                    limit,
                )
            }

            pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                use async_compression::futures::write::LzmaDecoder;
                async_write_to_vec(input, |input| Box::pin(LzmaDecoder::new(input)), limit)
            }
        }
    }
}

macro_rules! test_cases {
    ($variant:ident) => {
        mod $variant {
            mod stream {
                mod compress {
                    use crate::utils;
                    use futures::{executor::block_on, stream::StreamExt as _};
                    use std::iter::FromIterator;

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        // Can't use InputStream for this as it will inject extra empty chunks
                        let compressed =
                            utils::$variant::stream::compress(futures::stream::empty());
                        let output = utils::$variant::sync::decompress(&compressed);

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty_chunk() {
                        let input = utils::InputStream::from(vec![vec![]]);

                        let compressed = utils::$variant::stream::compress(input.stream());
                        let output = utils::$variant::sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

                        let compressed = utils::$variant::stream::compress(input.stream());
                        let output = utils::$variant::sync::decompress(&compressed);

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let input = vec![
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                            Vec::from_iter((0..32_768).map(|_| rand::random())),
                        ];
                        let input = utils::InputStream::from(input);

                        let compressed = utils::$variant::stream::compress(input.stream());
                        let output = utils::$variant::sync::decompress(&compressed);

                        assert_eq!(output, input.bytes());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn error() {
                        let err = std::io::Error::new(std::io::ErrorKind::Other, "failure");
                        let input = futures::stream::iter(vec![Err(err)]);

                        let mut stream = utils::$variant::stream::Encoder::with_quality(
                            input,
                            async_compression::Level::Fastest,
                        );

                        assert!(block_on(stream.next()).unwrap().is_err());
                        assert!(block_on(stream.next()).is_none());
                    }
                }

                mod decompress {
                    use crate::utils;
                    use futures::{executor::block_on, stream::StreamExt as _};
                    use std::iter::FromIterator;

                    #[test]
                    #[ntest::timeout(1000)]
                    fn empty() {
                        let compressed = utils::$variant::sync::compress(&[]);

                        let stream = utils::InputStream::from(vec![compressed]);
                        let output = utils::$variant::stream::decompress(stream.stream());

                        assert_eq!(output, &[][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn short() {
                        let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                        let stream = utils::InputStream::from(vec![compressed]);
                        let output = utils::$variant::stream::decompress(stream.stream());

                        assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long() {
                        let input = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = utils::$variant::sync::compress(&input);

                        let stream = utils::InputStream::from(vec![compressed]);
                        let output = utils::$variant::stream::decompress(stream.stream());

                        assert_eq!(output, input);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn long_chunks() {
                        let input = Vec::from_iter((0..65_536).map(|_| rand::random()));
                        let compressed = utils::$variant::sync::compress(&input);

                        let stream = utils::InputStream::from(
                            compressed.chunks(1024).map(Vec::from).collect::<Vec<_>>(),
                        );
                        let output = utils::$variant::stream::decompress(stream.stream());

                        assert_eq!(output, input);
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn error() {
                        let err = std::io::Error::new(std::io::ErrorKind::Other, "failure");
                        let input = futures::stream::iter(vec![Err(err)]);

                        let mut stream = utils::$variant::stream::Decoder::new(input);

                        assert!(block_on(stream.next()).unwrap().is_err());
                        assert!(block_on(stream.next()).is_none());
                    }

                    #[test]
                    #[ntest::timeout(1000)]
                    fn invalid_data() {
                        let input = futures::stream::iter(vec![Ok(bytes::Bytes::from(
                            &[1, 2, 3, 4, 5, 6][..],
                        ))]);

                        let mut stream = utils::$variant::stream::Decoder::new(input);

                        assert!(block_on(stream.next()).unwrap().is_err());
                        assert!(block_on(stream.next()).is_none());
                    }
                }
            }

            mod futures {
                mod bufread {
                    mod compress {
                        use crate::utils;
                        use std::iter::FromIterator;

                        #[test]
                        #[ntest::timeout(1000)]
                        fn empty() {
                            let mut input: &[u8] = &[];
                            let compressed =
                                utils::$variant::futures::bufread::compress(&mut input);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, &[][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn empty_chunk() {
                            let input = utils::InputStream::from(vec![vec![]]);

                            let compressed =
                                utils::$variant::futures::bufread::compress(input.reader());
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, input.bytes());
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short() {
                            let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

                            let compressed =
                                utils::$variant::futures::bufread::compress(input.reader());
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long() {
                            let input = vec![
                                Vec::from_iter((0..32_768).map(|_| rand::random())),
                                Vec::from_iter((0..32_768).map(|_| rand::random())),
                            ];
                            let input = utils::InputStream::from(input);

                            let compressed =
                                utils::$variant::futures::bufread::compress(input.reader());
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, input.bytes());
                        }
                    }

                    mod decompress {
                        use crate::utils;
                        use std::iter::FromIterator;

                        #[test]
                        #[ntest::timeout(1000)]
                        fn empty() {
                            let compressed = utils::$variant::sync::compress(&[]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output =
                                utils::$variant::futures::bufread::decompress(stream.reader());

                            assert_eq!(output, &[][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn zeros() {
                            let compressed = utils::$variant::sync::compress(&[0; 10]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output =
                                utils::$variant::futures::bufread::decompress(stream.reader());

                            assert_eq!(output, &[0; 10][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short() {
                            let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output =
                                utils::$variant::futures::bufread::decompress(stream.reader());

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short_chunks() {
                            let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                            let stream = utils::InputStream::from(
                                compressed.chunks(2).map(Vec::from).collect::<Vec<_>>(),
                            );
                            let output =
                                utils::$variant::futures::bufread::decompress(stream.reader());

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn trailer() {
                            let mut compressed =
                                utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                            compressed.extend_from_slice(&[7, 8, 9, 10]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let mut reader = stream.reader();
                            let output = utils::$variant::futures::bufread::decompress(&mut reader);
                            let trailer = utils::prelude::async_read_to_vec(reader);

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                            assert_eq!(trailer, &[7, 8, 9, 10][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long() {
                            let input = Vec::from_iter((0..65_536).map(|_| rand::random()));
                            let compressed = utils::$variant::sync::compress(&input);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output =
                                utils::$variant::futures::bufread::decompress(stream.reader());

                            assert_eq!(output, input);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long_chunks() {
                            let input = Vec::from_iter((0..65_536).map(|_| rand::random()));
                            let compressed = utils::$variant::sync::compress(&input);

                            let stream = utils::InputStream::from(
                                compressed.chunks(1024).map(Vec::from).collect::<Vec<_>>(),
                            );
                            let output =
                                utils::$variant::futures::bufread::decompress(stream.reader());

                            assert_eq!(output, input);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn multiple_members() {
                            let compressed = [
                                utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]),
                                utils::$variant::sync::compress(&[6, 5, 4, 3, 2, 1]),
                            ]
                            .join(&[][..]);

                            let stream = utils::InputStream::from(vec![compressed]);

                            let mut decoder =
                                utils::$variant::futures::bufread::Decoder::new(stream.reader());
                            decoder.multiple_members(true);
                            let output = utils::prelude::async_read_to_vec(decoder);

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..]);
                        }
                    }
                }

                mod write {
                    mod compress {
                        use crate::utils;
                        use std::iter::FromIterator;

                        #[test]
                        #[ntest::timeout(1000)]
                        fn empty() {
                            let input = utils::InputStream::from(vec![]);
                            let compressed =
                                utils::$variant::futures::write::compress(input.as_ref(), 65_536);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, &[][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn empty_chunk() {
                            let input = utils::InputStream::from(vec![vec![]]);

                            let compressed =
                                utils::$variant::futures::write::compress(input.as_ref(), 65_536);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, input.bytes());
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short() {
                            let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

                            let compressed =
                                utils::$variant::futures::write::compress(input.as_ref(), 65_536);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short_chunk_output() {
                            let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

                            let compressed =
                                utils::$variant::futures::write::compress(input.as_ref(), 2);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long() {
                            let input = vec![
                                Vec::from_iter((0..32_768).map(|_| rand::random())),
                                Vec::from_iter((0..32_768).map(|_| rand::random())),
                            ];
                            let input = utils::InputStream::from(input);

                            let compressed =
                                utils::$variant::futures::write::compress(input.as_ref(), 65_536);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, input.bytes());
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long_chunk_output() {
                            let input = vec![
                                Vec::from_iter((0..32_768).map(|_| rand::random())),
                                Vec::from_iter((0..32_768).map(|_| rand::random())),
                            ];
                            let input = utils::InputStream::from(input);

                            let compressed =
                                utils::$variant::futures::write::compress(input.as_ref(), 20);
                            let output = utils::$variant::sync::decompress(&compressed);

                            assert_eq!(output, input.bytes());
                        }
                    }

                    mod decompress {
                        use crate::utils;
                        use std::iter::FromIterator;

                        #[test]
                        #[ntest::timeout(1000)]
                        fn empty() {
                            let compressed = utils::$variant::sync::compress(&[]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output = utils::$variant::futures::write::decompress(
                                stream.as_ref(),
                                65_536,
                            );

                            assert_eq!(output, &[][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn zeros() {
                            let compressed = utils::$variant::sync::compress(&[0; 10]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output = utils::$variant::futures::write::decompress(
                                stream.as_ref(),
                                65_536,
                            );

                            assert_eq!(output, &[0; 10][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short() {
                            let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output = utils::$variant::futures::write::decompress(
                                stream.as_ref(),
                                65_536,
                            );

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn short_chunks() {
                            let compressed = utils::$variant::sync::compress(&[1, 2, 3, 4, 5, 6]);

                            let stream = utils::InputStream::from(
                                compressed.chunks(2).map(Vec::from).collect::<Vec<_>>(),
                            );
                            let output = utils::$variant::futures::write::decompress(
                                stream.as_ref(),
                                65_536,
                            );

                            assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long() {
                            let input = Vec::from_iter((0..65_536).map(|_| rand::random()));
                            let compressed = utils::$variant::sync::compress(&input);

                            let stream = utils::InputStream::from(vec![compressed]);
                            let output = utils::$variant::futures::write::decompress(
                                stream.as_ref(),
                                65_536,
                            );

                            assert_eq!(output, input);
                        }

                        #[test]
                        #[ntest::timeout(1000)]
                        fn long_chunks() {
                            let input = Vec::from_iter((0..65_536).map(|_| rand::random()));
                            let compressed = utils::$variant::sync::compress(&input);

                            let stream = utils::InputStream::from(
                                compressed.chunks(1024).map(Vec::from).collect::<Vec<_>>(),
                            );
                            let output = utils::$variant::futures::write::decompress(
                                stream.as_ref(),
                                65_536,
                            );

                            assert_eq!(output, input);
                        }
                    }
                }
            }
        }
    };
}
