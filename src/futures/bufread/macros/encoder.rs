macro_rules! encoder {
    ($(#[$attr:meta])* $name:ident<$inner:ident> $({ $($constructor:tt)* })*) => {
        pin_project_lite::pin_project! {
            $(#[$attr])*
            #[derive(Debug)]
            ///
            /// This structure implements an [`AsyncRead`](futures_io::AsyncRead) interface and will
            /// read uncompressed data from an underlying stream and emit a stream of compressed data.
            pub struct $name<$inner> {
                #[pin]
                inner: crate::futures::bufread::Encoder<$inner, crate::codec::$name>,
            }
        }

        impl<$inner: futures_io::AsyncBufRead> $name<$inner> {
            $(
                /// Creates a new encoder which will read uncompressed data from the given stream
                /// and emit a compressed stream.
                ///
                $($constructor)*
            )*
        }

        impl<$inner: futures_io::AsyncBufRead> futures_io::AsyncRead for $name<$inner> {
            fn poll_read(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &mut [u8],
            ) -> std::task::Poll<std::io::Result<usize>> {
                self.project().inner.poll_read(cx, buf)
            }
        }

        const _: () = {
            fn _assert() {
                use crate::util::{_assert_send, _assert_sync};
                use core::pin::Pin;
                use futures_io::AsyncBufRead;

                _assert_send::<$name<Pin<Box<dyn AsyncBufRead + Send>>>>();
                _assert_sync::<$name<Pin<Box<dyn AsyncBufRead + Sync>>>>();
            }
        };
    }
}
