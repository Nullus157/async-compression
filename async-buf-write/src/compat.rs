#[allow(unused_macros)]
macro_rules! make_compat {
    ($($AsyncWrite:ident)::+ { $poll_close:ident }) => {
        pin_project_lite::pin_project! {
            #[derive(Debug)]
            pub struct Compat<W> { #[pin] inner: W, }
        }

        impl<W> Compat<W> {
            pub fn new(inner: W) -> Self {
                Self { inner }
            }

            /// Gets a reference to the underlying writer.
            pub fn get_ref(&self) -> &W {
                &self.inner
            }

            /// Gets a mutable reference to the underlying writer.
            pub fn get_mut(&mut self) -> &mut W {
                &mut self.inner
            }

            /// Gets a pinned mutable reference to the underlying writer.
            pub fn get_pin_mut(self: core::pin::Pin<&mut Self>) -> core::pin::Pin<&mut W> {
                self.project().inner
            }

            /// Consumes this `Compat`, returning the underlying writer.
            pub fn into_inner(self) -> W {
                self.inner
            }
        }

        impl<W: $($AsyncWrite)::+> crate::AsyncWrite for Compat<W> {
            fn poll_write(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
                buf: &[u8],
            ) -> core::task::Poll<std::io::Result<usize>> {
                $($AsyncWrite)::+::poll_write(self.get_pin_mut(), cx, buf)
            }

            fn poll_flush(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<std::io::Result<()>> {
                $($AsyncWrite)::+::poll_flush(self.get_pin_mut(), cx)
            }

            fn poll_close(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<std::io::Result<()>> {
                $($AsyncWrite)::+::$poll_close(self.get_pin_mut(), cx)
            }
        }

        pub trait CompatExt: Sized {
            fn compat(self) -> Compat<Self>;
        }

        impl<W: $($AsyncWrite)::+> CompatExt for W {
            fn compat(self) -> Compat<Self> { Compat::new(self) }
        }

        impl<W: crate::AsyncWrite> $($AsyncWrite)::+ for Compat<W> {
            fn poll_write(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
                buf: &[u8],
            ) -> core::task::Poll<std::io::Result<usize>> {
                crate::AsyncWrite::poll_write(self.get_pin_mut(), cx, buf)
            }

            fn poll_flush(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<std::io::Result<()>> {
                crate::AsyncWrite::poll_flush(self.get_pin_mut(), cx)
            }

            fn $poll_close(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<std::io::Result<()>> {
                crate::AsyncWrite::poll_close(self.get_pin_mut(), cx)
            }
        }
    };
}
