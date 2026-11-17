//! This module contains zrip-specific types for async-compression.
//!
//! The parameter types mirror `zstd::params` where the two backends overlap, so switching between
//! them only needs a type change.

use compression_core::Level;

/// Lowest level zrip encodes.
const MIN_LEVEL: i32 = -7;
/// Highest level zrip encodes.
const MAX_LEVEL: i32 = 4;

/// A compression parameter for the `zrip` backend.
///
/// As with the `zstd` backend, the last value given for a parameter wins.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CParameter(CParameterInner);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CParameterInner {
    WindowLog(u32),
    EnableLongDistanceMatching(bool),
}

impl CParameter {
    pub fn quality(level: Level) -> i32 {
        // Same numbers as the `zstd` backend, clamped to the levels zrip encodes.
        match level {
            Level::Fastest => 1,
            Level::Best => MAX_LEVEL,
            Level::Precise(quality) => quality.clamp(MIN_LEVEL, MAX_LEVEL),
            _ => 3,
        }
    }

    /// Window size in bytes (as a power of two)
    pub fn window_log(value: u32) -> Self {
        Self(CParameterInner::WindowLog(value))
    }

    /// Enable long distance matching mode to look for and emit long-distance references.
    ///
    /// This increases the default window size.
    pub fn enable_long_distance_matching(value: bool) -> Self {
        Self(CParameterInner::EnableLongDistanceMatching(value))
    }
}

pub(crate) fn options(params: &[CParameter]) -> zrip::Options {
    params
        .iter()
        .fold(zrip::Options::default(), |opts, param| match param.0 {
            CParameterInner::WindowLog(value) => opts.window_log(value),
            CParameterInner::EnableLongDistanceMatching(value) => opts.ldm(value),
        })
}

/// A decompression parameter for the `zrip` backend.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DParameter(DParameterInner);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DParameterInner {
    WindowLogMax(u32),
}

impl DParameter {
    /// Maximum window size in bytes (as a power of two). The default is 128 MiB.
    pub fn window_log_max(value: u32) -> Self {
        Self(DParameterInner::WindowLogMax(value))
    }
}

/// The largest window the parameters allow, if they set one. As with the `zstd` backend, `0`
/// selects the default.
pub(crate) fn max_window(params: &[DParameter]) -> Option<u64> {
    params.iter().fold(None, |_, param| match param.0 {
        DParameterInner::WindowLogMax(0) => None,
        DParameterInner::WindowLogMax(value) => Some(1u64 << value.min(63)),
    })
}
