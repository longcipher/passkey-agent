# Default recipe to display help
default:
  @just --list

# Format all code: markdown, TOML formatting, rustfmt
format:
  rumdl fmt .
  taplo fmt
  cargo +nightly fmt --all

# Auto-fix what `format` cannot: markdown lint and clippy
fix:
  rumdl check --fix .
  RUSTC_WRAPPER= cargo +nightly clippy --fix --all --allow-dirty

# Run all lints (the CI gate: typos, rumdl, taplo, rustfmt, clippy, machete)
lint:
  typos
  rumdl check .
  taplo fmt --check
  cargo +nightly fmt --all -- --check
  RUSTC_WRAPPER= cargo +nightly clippy --all -- -D warnings
  cargo machete

# Run tests
test:
  cargo test --all-features

# Run every test suite in the workspace
test-all:
  cargo test --all-features

# Run mutation testing with cargo-mutants
mutation:
  cargo mutants

# Run tests with coverage
test-coverage:
  cargo tarpaulin --all-features --workspace --timeout 300

# Build entire workspace
build:
  cargo build --workspace

# Check all targets compile
check:
  cargo check --all-targets --all-features

# Publish all crates to crates.io (dry run)
publish-check:
  cargo publish --workspace --dry-run --allow-dirty

# Publish all crates to crates.io (irreversible; prompts only on an interactive terminal)
publish:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ -t 0 ]; then
    read -r -p "Type 'publish' to continue: " confirm
    if [ "$confirm" != "publish" ]; then
      echo "aborted"
      exit 1
    fi
  fi
  cargo publish --workspace

# Check for Chinese characters (the Chinese design blueprints are excluded)
check-cn:
  ! rg --line-number --column "\p{Han}" --glob '!docs/design.md' --glob '!specs/2026-06-12-04-graceful-shutdown/design.md'

# Full CI check (mirrors the `ci` GitHub workflow)
ci: lint test-all build

# ============================================================
# Maintenance & Tools
# ============================================================

# Clean build artifacts
clean:
  cargo clean

# Install all required development tools
setup:
  cargo install cargo-machete
  cargo install taplo-cli
  cargo install typos-cli
  cargo install rumdl
  cargo install cargo-mutants
  cargo install cargo-tarpaulin
  cargo install cargo-audit
  cargo install cargo-outdated
  cargo install cargo-nextest
  rustup component add miri --toolchain nightly

# Generate documentation for the workspace
docs:
  cargo doc --no-deps --open

# Run security audit
audit:
  cargo audit

# Check for outdated dependencies
outdated:
  cargo outdated

# Run clippy with pedantic, nursery and the cargo lint groups
clippy-all:
  RUSTC_WRAPPER= cargo +nightly clippy --all --all-targets --all-features -- -W clippy::pedantic -W clippy::nursery -W clippy::cargo

# Run tests with nextest
test-nextest:
  cargo nextest run --all-features

# Run miri on unsafe code
miri:
  cargo +nightly miri test --all-features

# Check for unused dependencies
unused-deps:
  cargo machete

# Update all dependencies
update-deps:
  cargo update

# Full check (lint + test + build + audit)
full-check: lint test build audit
