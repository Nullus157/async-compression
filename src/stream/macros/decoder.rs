macro_rules! decoder {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[pin_project::pin_project]
        #[derive(Debug)]
        ///
        /// This structure implements a [`Stream`](futures_core::stream::Stream) interface and will read
        /// compressed data from an underlying stream and emit a stream of uncompressed data.
        pub struct $name<S: futures_core::stream::Stream<Item = std::io::Result<bytes::Bytes>>> {
            #[pin]
            inner: crate::stream::generic::Decoder<S, crate::codec::$name>,
        }

        impl<S: futures_core::stream::Stream<Item = std::io::Result<bytes::Bytes>>> $name<S> {
            /// Creates a new decoder which will read compressed data from the given stream and
            /// emit an uncompressed stream.
            pub fn new(stream: S) -> Self {
                Self {
                    inner: crate::stream::Decoder::new(
                        stream,
                        crate::codec::$name::new(),
                    ),
                }
            }

            /// Acquires a reference to the underlying stream that this decoder is wrapping.
            pub fn get_ref(&self) -> &S {
                self.inner.get_ref()
            }

            /// Acquires a mutable reference to the underlying stream that this decoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the stream which
            /// may otherwise confuse this decoder.
            pub fn get_mut(&mut self) -> &mut S {
                self.inner.get_mut()
            }

            /// Acquires a pinned mutable reference to the underlying stream that this decoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the stream which
            /// may otherwise confuse this decoder.
            pub fn get_pin_mut(self: std::pin::Pin<&mut Self>) -> std::pin::Pin<&mut S> {
                self.project().inner.get_pin_mut()
            }

            /// Consumes this decoder returning the underlying stream.
            ///
            /// Note that this may discard internal state of this decoder, so care should be taken
            /// to avoid losing resources when this is called.
            pub fn into_inner(self) -> S {
                self.inner.into_inner()
            }
        }

        impl<S: futures_core::stream::Stream<Item = std::io::Result<bytes::Bytes>>>
            futures_core::stream::Stream for $name<S>
        {
            type Item = std::io::Result<bytes::Bytes>;

            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<std::io::Result<bytes::Bytes>>> {
                self.project().inner.poll_next(cx)
            }
        }

        const _: () = {
            fn _assert() {
                use std::{pin::Pin, io::Result};
                use bytes::Bytes;
                use futures_core::stream::Stream;
                use crate::util::{_assert_send, _assert_sync};

                _assert_send::<$name<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>>();
                _assert_sync::<$name<Pin<Box<dyn Stream<Item = Result<Bytes>> + Sync>>>>();
            }
        };
    }
}
