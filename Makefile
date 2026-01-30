.PHONY: build build-release build-linux test lint clean install run watch

# Default target
build:
	cargo build

# Optimized release build
build-release:
	cargo build --release

# Cross-compile for Linux (from macOS)
build-linux:
	cargo build --release --target x86_64-unknown-linux-gnu

# Run all tests
test:
	cargo test

# Run clippy lints
lint:
	cargo clippy

# Run pedantic clippy (stricter)
lint-pedantic:
	cargo clippy -- -W clippy::pedantic

# Clean build artifacts
clean:
	cargo clean

# Install to ~/.cargo/bin
install:
	cargo install --path .

# Run against example fixture
run:
	cargo run -- ./fixtures/example-monorepo/packages --exclude dist

# Run in watch mode against example fixture
watch:
	cargo run -- --watch ./fixtures/example-monorepo/packages --exclude dist

# Format code
fmt:
	cargo fmt

# Check formatting without modifying
fmt-check:
	cargo fmt -- --check

# Run all checks (CI)
ci: fmt-check lint test
	@echo "All checks passed!"
