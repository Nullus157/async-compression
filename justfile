_list:
    @just --list

# Format project.
fmt:
    just --unstable --fmt
    cargo +nightly fmt

# Check project.
[group("lint")]
check: && clippy
    just --unstable --fmt --check
    cargo +nightly fmt -- --check

# Lint workspace with Clippy.
clippy:
    cargo clippy --workspace --all-targets --no-default-features
    cargo clippy --workspace --all-targets --all-features

# Document crates in workspace.
doc *args:
    RUSTDOCFLAGS="--cfg=docsrs -Dwarnings" cargo +nightly doc --workspace --all-features {{ args }}

# Checks feature combinations for `tokio|futures-io` async runtime.
check-features async_runtime:
    cargo hack check \
        -p=compression-codecs \
        --feature-powerset \
        --no-dev-deps \
        --skip "xz-parallel,zstdmt" \
        --depth 4
    skipped_features="$(echo 'futures-io,tokio,' | sed 's/{{ async_runtime }},//')" \
    && cargo hack check \
        -p=async-compression \
        --feature-powerset \
        --features {{ async_runtime }} \
        --no-dev-deps \
        --skip "all,all-algorithms,${skipped_features}all-implementations,xz-parallel,zstdmt" \
        --depth 4
    cargo check --features {{ async_runtime }},xz-parallel,zstdmt

# Checks feature combinations for for `tokio|futures-io` async runtime for all targets.
check-test-features async_runtime:
    cargo hack check \
        -p=compression-codecs \
        --feature-powerset \
        --all-targets \
        --skip "xz-parallel,zstdmt" \
        --depth 4
    skipped_features="$(echo 'futures-io,tokio,' | sed 's/{{ async_runtime }},//')" \
    && cargo hack check \
        -p=async-compression \
        --feature-powerset \
        --features {{ async_runtime }} \
        --all-targets \
        --skip "all,all-algorithms,${skipped_features}all-implementations,xz-parallel,zstdmt" \
        --depth 4
    cargo check --all-targets --features {{ async_runtime }},xz-parallel,zstdmt
