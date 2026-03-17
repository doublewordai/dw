# DW CLI development commands

# Build both crates
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Run the CLI with arguments
run *args:
    cargo run --bin dw -- {{args}}

# Run all tests
test:
    cargo test --workspace

# Lint (clippy + fmt check)
lint:
    cargo fmt --check
    cargo clippy --workspace -- -D warnings

# Format code
fmt:
    cargo fmt --all

# CI: lint + test
ci: lint test

# Check compilation without building
check:
    cargo check --workspace

# Clean build artifacts
clean:
    cargo clean
