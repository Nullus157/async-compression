use std::{
    io::{Error, ErrorKind, Result},
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{BufMut, Bytes, BytesMut};
pub use flate2::Compression;
use flate2::{Compress, Crc, Decompress, FlushCompress, FlushDecompress, Status};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    WritingHeader(Compression),
    Reading,
    WritingChunk(Bytes),
    FlushingData,
    WritingFooter,
    Done,
    Invalid,
}

#[unsafe_project(Unpin)]
#[derive(Debug)]
pub struct GzipStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: State,
    output: BytesMut,
    crc: Crc,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for GzipStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn compress(
            compress: &mut Compress,
            input: &mut Bytes,
            output: &mut BytesMut,
            crc: &mut Crc,
            flush: FlushCompress,
        ) -> Result<(Status, Bytes)> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let (prior_in, prior_out) = (compress.total_in(), compress.total_out());
            let status = compress.compress(input, output, flush)?;
            let input_len = compress.total_in() - prior_in;
            let output_len = compress.total_out() - prior_out;

            crc.update(&input[0..input_len as usize]);
            input.advance(input_len as usize);
            Ok((status, output.split_to(output_len as usize).freeze()))
        }

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, State::Invalid) {
                State::WritingHeader(level) => {
                    *this.state = State::Reading;
                    Poll::Ready(Some(Ok(get_header(level))))
                }

                State::Reading => {
                    *this.state = State::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => State::WritingChunk(chunk?),
                        None => State::FlushingData,
                    };
                    continue;
                }

                State::WritingChunk(mut input) => {
                    if input.is_empty() {
                        *this.state = State::Reading;
                        continue;
                    }

                    let (status, chunk) = compress(
                        &mut this.compress,
                        &mut input,
                        &mut this.output,
                        &mut this.crc,
                        FlushCompress::None,
                    )?;

                    *this.state = match status {
                        Status::Ok => State::WritingChunk(input),
                        Status::StreamEnd => unreachable!(),
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::FlushingData => {
                    let (status, chunk) = compress(
                        &mut this.compress,
                        &mut Bytes::new(),
                        &mut this.output,
                        &mut this.crc,
                        FlushCompress::Finish,
                    )?;

                    *this.state = match status {
                        Status::StreamEnd => State::WritingFooter,
                        Status::Ok => State::FlushingData,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::WritingFooter => {
                    let mut footer = BytesMut::with_capacity(8);

                    footer.put(this.crc.sum().to_le_bytes().as_ref());
                    footer.put(this.crc.amount().to_le_bytes().as_ref());

                    *this.state = State::Done;

                    Poll::Ready(Some(Ok(footer.freeze())))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("GzipStream reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>>> GzipStream<S> {
    pub fn new(stream: S, level: Compression) -> GzipStream<S> {
        GzipStream {
            inner: stream,
            state: State::WritingHeader(level),
            output: BytesMut::new(),
            crc: Crc::new(),
            compress: Compress::new(level, false),
        }
    }
}

fn get_header(level: Compression) -> Bytes {
    let mut header = vec![0u8; 10];
    header[0] = 0x1f;
    header[1] = 0x8b;
    header[2] = 0x08;
    header[8] = if level.level() >= Compression::best().level() {
        0x02
    } else if level.level() <= Compression::fast().level() {
        0x04
    } else {
        0x00
    };
    header[9] = 0xff;

    Bytes::from(header)
}

#[derive(Debug)]
enum DeState {
    ReadingHeader,
    Reading,
    Writing,
    ReadingFooter,
    Invalid,
}

#[unsafe_project(Unpin)]
pub struct DecompressedGzipStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: DeState,
    input: BytesMut,
    output: BytesMut,
    crc: Crc,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for DecompressedGzipStream<S> {
    type Item = Result<Bytes>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn decompress(
            decompress: &mut Decompress,
            input: &mut BytesMut,
            output: &mut BytesMut,
            crc: &mut Crc,
            flush: FlushDecompress,
        ) -> Result<(Status, Bytes)> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let (prior_in, prior_out) = (decompress.total_in(), decompress.total_out());
            let status = decompress.decompress(input, output, flush)?;
            let input_len = decompress.total_in() - prior_in;
            let output_len = decompress.total_out() - prior_out;

            crc.update(&output[0..output_len as usize]);
            input.advance(input_len as usize);
            Ok((status, output.split_to(output_len as usize).freeze()))
        }

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match dbg!(mem::replace(this.state, DeState::Invalid)) {
                DeState::ReadingHeader => {
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            this.input.extend_from_slice(&chunk?);
                            if this.input.len() >= 10 {
                                let header = this.input.split_to(10);
                                if header[0..3] != [0x1f, 0x8b, 0x08] {
                                    return Poll::Ready(Some(Err(Error::new(
                                        ErrorKind::InvalidData,
                                        "Invalid file header",
                                    ))));
                                }
                                DeState::Reading
                            } else {
                                DeState::ReadingHeader
                            }
                        }
                        None => {
                            return Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "A valid header was not found",
                            ))));
                        }
                    };
                    continue;
                }
                DeState::Reading => {
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            this.input.extend_from_slice(&chunk?);
                            DeState::Writing
                        }
                        None => DeState::ReadingFooter,
                    };
                    continue;
                }

                DeState::Writing => {
                    if this.input.len() <= 8 {
                        *this.state = DeState::Reading;
                        continue;
                    }
                    let (status, chunk) = decompress(
                        &mut this.decompress,
                        &mut this.input,
                        &mut this.output,
                        &mut this.crc,
                        FlushDecompress::None,
                    )?;

                    *this.state = match status {
                        Status::Ok => DeState::Writing,
                        Status::StreamEnd => DeState::Reading,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                DeState::ReadingFooter => {
                    if this.input.len() == 8 {
                        let crc = &this.crc.sum().to_le_bytes()[..];
                        let bytes_read = &this.crc.amount().to_le_bytes()[..];
                        if crc != &this.input[0..4] {
                            return Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "CRC computed does not match",
                            ))));
                        } else if bytes_read != &this.input[4..8] {
                            return Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "amount of bytes read does not match",
                            ))));
                        }
                        Poll::Ready(None)
                    } else {
                        unreachable!() // this should be unreachable
                    }
                }

                DeState::Invalid => panic!("DecompressedGzipStream reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>>> DecompressedGzipStream<S> {
    pub fn new(stream: S) -> DecompressedGzipStream<S> {
        DecompressedGzipStream {
            inner: stream,
            state: DeState::ReadingHeader,
            input: BytesMut::new(),
            output: BytesMut::new(),
            crc: Crc::new(),
            decompress: Decompress::new(false),
        }
    }
}
