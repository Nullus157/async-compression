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

#[unsafe_project(Unpin)]
pub struct DecompressedGzipStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    output_buffer: BytesMut,
    crc: Crc,
    header_stripped: bool,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for DecompressedGzipStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

        if this.input_buffer.len() <= 8 {
            if *this.flushing {
                // check crc and len in the footer
                let crc = &this.crc.sum().to_le_bytes()[..];
                let bytes_read = &this.crc.amount().to_le_bytes()[..];
                if crc != this.input_buffer.slice(0, 4) {
                    return Poll::Ready(Some(Err(Error::new(
                        ErrorKind::InvalidData,
                        "CRC computed does not match",
                    ))));
                } else if bytes_read != this.input_buffer.slice(4, 8) {
                    return Poll::Ready(Some(Err(Error::new(
                        ErrorKind::InvalidData,
                        "amount of bytes read does not match",
                    ))));
                }
                return Poll::Ready(None);
            } else if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
                this.input_buffer.extend_from_slice(&bytes?);
            } else {
                *this.flushing = true;
            }
        }

        if !*this.header_stripped {
            let header = this.input_buffer.split_to(10);
            if header[0..3] != [0x1f, 0x8b, 0x08] {
                return Poll::Ready(Some(Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid file header",
                ))));
            }
            *this.header_stripped = true;
        }

        this.output_buffer.resize(OUTPUT_BUFFER_SIZE, 0);

        let flush = if *this.flushing {
            FlushDecompress::Finish
        } else {
            FlushDecompress::None
        };

        let (prior_in, prior_out) = (this.decompress.total_in(), this.decompress.total_out());
        this.decompress
            .decompress(this.input_buffer, this.output_buffer, flush)?;
        let input = this.decompress.total_in() - prior_in;
        let output = this.decompress.total_out() - prior_out;

        this.crc.update(&this.output_buffer[..output as usize]);
        this.input_buffer.advance(input as usize);

        Poll::Ready(Some(Ok(this
            .output_buffer
            .split_to(output as usize)
            .freeze())))
    }
}

impl<S: Stream<Item = Result<Bytes>>> DecompressedGzipStream<S> {
    pub fn new(stream: S) -> DecompressedGzipStream<S> {
        DecompressedGzipStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            output_buffer: BytesMut::new(),
            crc: Crc::new(),
            header_stripped: false,
            decompress: Decompress::new(false),
        }
    }
}
