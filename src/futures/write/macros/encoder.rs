macro_rules! encoder {
    ($(#[$attr:meta])* $name:ident<$inner:ident> $({ $($constructor:tt)* })*) => {
        pin_project_lite::pin_project! {
            $(#[$attr])*
            #[derive(Debug)]
            ///
            /// This structure implements an [`AsyncWrite`](futures_io::AsyncWrite) interface and will
            /// take in uncompressed data and write it compressed to an underlying stream.
            pub struct $name<$inner> {
                #[pin]
                inner: async_buf_write::futures_io_03::Compat<
                    crate::generic::write::Encoder<
                        async_buf_write::futures_io_03::Compat<$inner>,
                        crate::codec::$name,
                    >,
                >,
            }
        }

        impl<$inner: futures_io::AsyncWrite> $name<$inner> {
            $(
                /// Creates a new encoder which will take in uncompressed data and write it
                /// compressed to the given stream.
                ///
                $($constructor)*
            )*

            pub(crate) fn with_encoder(
                writer: $inner,
                encoder: crate::codec::$name,
            ) -> $name<$inner> {
                $name {
                    inner: async_buf_write::futures_io_03::Compat::new(
                        crate::generic::write::Encoder::new(
                            async_buf_write::futures_io_03::Compat::new(writer),
                            encoder,
                        ),
                    ),
                }
            }

            /// Acquires a reference to the underlying writer that this encoder is wrapping.
            pub fn get_ref(&self) -> &$inner {
                self.inner.get_ref().get_ref().get_ref()
            }

            /// Acquires a mutable reference to the underlying writer that this encoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the writer which
            /// may otherwise confuse this encoder.
            pub fn get_mut(&mut self) -> &mut $inner {
                self.inner.get_mut().get_mut().get_mut()
            }

            /// Acquires a pinned mutable reference to the underlying writer that this encoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the writer which
            /// may otherwise confuse this encoder.
            pub fn get_pin_mut(self: std::pin::Pin<&mut Self>) -> std::pin::Pin<&mut $inner> {
                self.project().inner.get_pin_mut().get_pin_mut().get_pin_mut()
            }

            /// Consumes this encoder returning the underlying writer.
            ///
            /// Note that this may discard internal state of this encoder, so care should be taken
            /// to avoid losing resources when this is called.
            pub fn into_inner(self) -> $inner {
                self.inner.into_inner().into_inner().into_inner()
            }
        }

        impl<$inner: futures_io::AsyncWrite> futures_io::AsyncWrite for $name<$inner> {
            fn poll_write(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &[u8],
            ) -> std::task::Poll<std::io::Result<usize>> {
                self.project().inner.poll_write(cx, buf)
            }

            fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                self.project().inner.poll_flush(cx)
            }

            fn poll_close(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                self.project().inner.poll_close(cx)
            }
        }

        const _: () = {
            fn _assert() {
                use crate::util::{_assert_send, _assert_sync};
                use core::pin::Pin;
                use futures_io::AsyncWrite;

                _assert_send::<$name<Pin<Box<dyn AsyncWrite + Send>>>>();
                _assert_sync::<$name<Pin<Box<dyn AsyncWrite + Sync>>>>();
            }
        };
    }
}
