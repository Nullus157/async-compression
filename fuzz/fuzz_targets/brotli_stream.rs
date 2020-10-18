#![no_main]
use libfuzzer_sys::fuzz_target;

use futures::stream::{StreamExt as _};
use futures_test::stream::{StreamTestExt as _};
use async_compression::{Level, stream::{BrotliEncoder, BrotliDecoder}};
use bytes::Bytes;

fuzz_target!(|data: Vec<Vec<u8>>| {
    futures::executor::block_on(async move {
        let expected: Vec<u8> = data.iter().flatten().copied().collect();
        let stream = futures::stream::iter(
                data
                .into_iter()
                .map(Bytes::from)
                .map(Ok),
        );
        let encoder = BrotliEncoder::with_quality(stream.interleave_pending(), Level::Fastest);
        let decoder = BrotliDecoder::new(encoder.interleave_pending());
        let decoded: Vec<u8> = decoder.map(|b| futures::stream::iter(b.unwrap())).flatten().collect().await;
        assert_eq!(expected, decoded);
    });
});
