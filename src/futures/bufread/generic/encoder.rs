use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use crate::{codec::Encode, util::PartialBuffer};
use futures_io::{AsyncBufRead, AsyncRead};
use futures_util::future::poll_fn;
use pin_project_lite::pin_project;
use pin_utils::pin_mut;
use poll_fill_buf::FillBufExt as _;
use std::fmt::Debug;

pub type Encoder<R, E: Encode> = impl AsyncRead + Debug;

pub fn new<R: AsyncBufRead, E: Encode>(reader: R, encoder: E) -> Encoder<R, E> {
    async_io_macros::async_read! {
        let reader = reader;
        let mut encoder = encoder;
        pin_mut!(reader);

        loop {
            let input = reader.fill_buf().await?;
            if input.is_empty() {
                break;
            }
            let mut input = PartialBuffer::new(input);
            yield |buf| {
                let mut output = PartialBuffer::new(buf);
                encoder.encode(&mut input, &mut output)?;
                let len = input.written().len();
                reader.as_mut().consume(len);
                Ok(output.written().len())
            }
        }

        let mut done = false;
        while !done {
            yield |buf| {
                let mut output = PartialBuffer::new(buf);
                done = encoder.finish(&mut output)?;
                Ok(output.written().len())
            }
        }

        Ok(())
    }
}

mod poll_fill_buf {
    use futures_io::AsyncBufRead;
    use std::future::Future;
    use std::io::Result;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    pub struct FillBuf<'a, R: ?Sized> {
        buf: Option<&'a mut R>,
    }

    pub trait FillBufExt: AsyncBufRead {
        fn fill_buf(&mut self) -> FillBuf<'_, Self> {
            FillBuf { buf: Some(self) }
        }
    }

    impl<T: AsyncBufRead + ?Sized> FillBufExt for T {}

    impl<'a, R: AsyncBufRead + Unpin> Future for FillBuf<'a, R> {
        type Output = Result<&'a [u8]>;
        fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<&'a [u8]>> {
            let this = &mut *self;
            let buf = this.buf.take().expect("Poll after completion.");

            match Pin::new(&mut *buf).poll_fill_buf(ctx) {
                Poll::Ready(Ok(_slice)) => match Pin::new(buf).poll_fill_buf(ctx) {
                    Poll::Ready(Ok(slice)) => Poll::Ready(Ok(slice)),
                    _ => unreachable!(),
                },
                Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
                Poll::Pending => {
                    this.buf = Some(buf);
                    Poll::Pending
                }
            }
        }
    }
}
