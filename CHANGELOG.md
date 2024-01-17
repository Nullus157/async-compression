# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.4.6

- Flush available data in decoder even when there's no incoming input.
- Return errors instead of panicking in all encode and decode operations.

## 0.4.5

- Add `{Lzma, Xz}Decoder::with_mem_limit()` methods.

## 0.4.4

- Update `zstd` dependency to `0.13`.

## 0.4.3

- Implement `Default` for `brotli::EncoderParams`.

## 0.4.2

- Add top-level `brotli` module containing stable `brotli` crate wrapper types.
- Add `BrotliEncoder::with_quality_and_params()` constructors.
- Add `Deflate64Decoder` behind new crate feature `deflate64`.

## 0.4.1 - 2023-07-10

- Add `Zstd{Encoder,Decoder}::with_dict()` constructors.
- Add `zstdmt` crate feature that enables `zstd-safe/zstdmt`, allowing multi-threaded functionality to work as expected.

## 0.4.0 - 2023-05-10

- `Level::Precise` variant now takes a `i32` instead of `u32`.
- Add top-level `zstd` module containing stable `zstd` crate wrapper types.
- Add `ZstdEncoder::with_quality_and_params()` constructors.
- Update `zstd` dependency to `0.12`.
- Remove deprecated `stream`, `futures-bufread` and `futures-write` crate features.
- Remove Tokio 0.2.x and 0.3.x support (`tokio-02` and `tokio-03` crate features).

## 0.3.15 - 2022-10-08

- `Level::Default::into_zstd()` now returns zstd's default value `3`.
- Fix endianness when reading the `extra` field of a gzip header.
