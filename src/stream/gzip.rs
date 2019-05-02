use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use bytes::{Bytes, BytesMut};
pub use flate2::Compression;
use flate2::{Compress, Crc, Decompress, FlushCompress};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

#[unsafe_project(Unpin)]
pub struct GzipStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    output_buffer: BytesMut,
    crc: Crc,
    header_appended: bool,
    footer_appended: bool,
    compress: Compress,
    level: Compression,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for GzipStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

        if !*this.header_appended {
            let header = get_header(*this.level);
            *this.header_appended = true;
            return Poll::Ready(Some(Ok(header)));
        }

        if this.input_buffer.is_empty() {
            if *this.flushing {
                if !*this.footer_appended {
                    let mut footer = Bytes::from(&this.crc.sum().to_le_bytes()[..]);
                    let length_read = &this.crc.amount().to_le_bytes()[..];
                    footer.extend_from_slice(length_read);
                    *this.footer_appended = true;
                    return Poll::Ready(Some(Ok(footer)));
                } else {
                    return Poll::Ready(None);
                }
            } else if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
                *this.input_buffer = bytes?;
            } else {
                *this.flushing = true;
            }
        }

        this.output_buffer.resize(OUTPUT_BUFFER_SIZE, 0);

        let flush = if *this.flushing {
            FlushCompress::Finish
        } else {
            FlushCompress::None
        };

        let (prior_in, prior_out) = (this.compress.total_in(), this.compress.total_out());
        this.compress
            .compress(this.input_buffer, this.output_buffer, flush)?;
        let input = this.compress.total_in() - prior_in;
        let output = this.compress.total_out() - prior_out;

        this.crc.update(&this.input_buffer.slice(0, input as usize));
        this.input_buffer.advance(input as usize);
        Poll::Ready(Some(Ok(this
            .output_buffer
            .split_to(output as usize)
            .freeze())))
    }
}

impl<S: Stream<Item = Result<Bytes>>> GzipStream<S> {
    pub fn new(stream: S, level: Compression) -> GzipStream<S> {
        GzipStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            output_buffer: BytesMut::new(),
            crc: Crc::new(),
            header_appended: false,
            footer_appended: false,
            compress: Compress::new(level, false),
            level,
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
    footer_stripped: bool,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for DecompressedGzipStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

        if this.input_buffer.is_empty() {
            if *this.flushing {
                return Poll::Ready(None);
            } else if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
                *this.input_buffer = bytes?;
            } else {
                *this.flushing = true;
            }
        }

        unimplemented!()
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
            footer_stripped: false,
            decompress: Decompress::new(false),
        }
    }
}
