use core::{
    marker::{PhantomData, Unpin},
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use bytes::{Bytes, BytesMut};
pub use flate2::Compression;
use flate2::{Compress, Crc, FlushCompress};
use futures::{
    ready,
    stream::{self, Stream, StreamExt},
};
use pin_project::unsafe_project;

pub struct GzipStream<S: Stream<Item = Result<Bytes>> + Unpin + 'static> {
    inner: Box<dyn Stream<Item = Result<Bytes>> + Unpin>,
    _marker: PhantomData<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for GzipStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

#[unsafe_project(Unpin)]
struct GzipBodyStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flushing: bool,
    input_buffer: Bytes,
    output_buffer: BytesMut,
    crc: Crc,
    footer_appended: bool,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for GzipBodyStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

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

impl<S: Stream<Item = Result<Bytes>> + Unpin> GzipStream<S> {
    pub fn new(stream: S, level: Compression) -> GzipStream<S> {
        let header_stream = stream::iter(vec![Ok(get_header(level))]);
        let body_stream = GzipBodyStream {
            inner: stream,
            flushing: false,
            input_buffer: Bytes::new(),
            output_buffer: BytesMut::new(),
            crc: Crc::new(),
            footer_appended: false,
            compress: Compress::new(level, false),
        };

        let final_stream = header_stream.chain(body_stream);
        GzipStream {
            inner: Box::new(final_stream),
            _marker: PhantomData,
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
