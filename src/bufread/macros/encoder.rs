macro_rules! encoder {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[pin_project::unsafe_project(Unpin)]
        #[derive(Debug)]
        pub struct $name<R: futures::io::AsyncBufRead> {
            #[pin]
            inner: crate::bufread::Encoder<R, crate::codec::$name>,
        }

        impl<R: futures::io::AsyncBufRead> $name<R> {
            /// Acquires a reference to the underlying reader that this encoder is wrapping.
            pub fn get_ref(&self) -> &R {
                self.inner.get_ref()
            }

            /// Acquires a mutable reference to the underlying reader that this encoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the reader which
            /// may otherwise confuse this encoder.
            pub fn get_mut(&mut self) -> &mut R {
                self.inner.get_mut()
            }

            /// Acquires a pinned mutable reference to the underlying reader that this encoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the reader which
            /// may otherwise confuse this encoder.
            pub fn get_pin_mut<'a>(self: std::pin::Pin<&'a mut Self>) -> std::pin::Pin<&'a mut R> {
                self.project().inner.get_pin_mut()
            }

            /// Consumes this encoder returning the underlying reader.
            ///
            /// Note that this may discard internal state of this encoder, so care should be taken
            /// to avoid losing resources when this is called.
            pub fn into_inner(self) -> R {
                self.inner.into_inner()
            }
        }

        impl<R: futures::io::AsyncBufRead> futures::io::AsyncRead for $name<R> {
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
                use futures::io::AsyncBufRead;

                _assert_send::<$name<Pin<Box<dyn AsyncBufRead + Send>>>>();
                _assert_sync::<$name<Pin<Box<dyn AsyncBufRead + Sync>>>>();
            }
        };
    }
}
