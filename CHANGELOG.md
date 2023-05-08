# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.3.15 - 2022-10-08

- `Level::Default::into_zstd()` now returns libzstd's default value `3`.
- Fix endianness when reading the `extra` field of a gzip header.
