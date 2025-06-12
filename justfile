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
