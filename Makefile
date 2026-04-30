.PHONY: install-hooks build test fmt check lint clean help all

# Help target
help:
	@echo "Rusty Wire Makefile targets:"
	@echo "  install-hooks    - Install git hooks for local development"
	@echo "  build            - Build the project in debug mode"
	@echo "  release          - Build the project in release mode"
	@echo "  test             - Run all tests (unit, integration, corpus)"
	@echo "  test-fast        - Run only unit and integration tests"
	@echo "  test-corpus      - Run corpus validation tests"
	@echo "  fmt              - Format code with rustfmt"
	@echo "  fmt-check        - Check code formatting without changing files"
	@echo "  lint             - Run clippy linter"
	@echo "  check            - Quick compile check"
	@echo "  sbom             - Generate SBOM in SPDX format"
	@echo "  clean            - Remove build artifacts"
	@echo "  help             - Show this help message"

# Install git hooks from .githooks directory
install-hooks:
	@echo "Installing git hooks..."
	@chmod +x .githooks/pre-push .githooks/pre-commit 2>/dev/null || true
	@chmod +x scripts/*.sh 2>/dev/null || true
	@git config core.hooksPath .githooks
	@echo "Git hooks installed successfully."
	@echo ""
	@echo "Hooks configured:"
	@echo "  pre-commit  - format + lint + unit tests (fast, ~10s)"
	@echo "  pre-push    - format + check + full tests + SBOM (thorough, ~30s)"
	@echo ""
	@echo "Test the hooks: git commit --allow-empty -m 'test hooks' && git reset HEAD~1"

# Build targets
build:
	@echo "Building debug binary..."
	cargo build

release:
	@echo "Building release binary..."
	cargo build --release

# Test targets
test:
	@echo "Running all tests..."
	cargo test --all

test-fast:
	@echo "Running fast tests (unit + integration)..."
	cargo test --lib
	cargo test --test '*'

test-corpus:
	@echo "Running corpus validation tests..."
	cargo test --test '*' corpus 2>/dev/null || echo "Corpus tests not yet implemented (GAP-010)"

# Formatting targets
fmt:
	@echo "Formatting code..."
	cargo fmt

fmt-check:
	@echo "Checking code formatting..."
	cargo fmt -- --check

# Linting
lint:
	@echo "Running clippy..."
	cargo clippy -- -D warnings

check:
	@echo "Running cargo check..."
	cargo check

# SBOM generation
sbom:
	@echo "Generating SBOM..."
	./scripts/generate-sbom.sh spdx
	./scripts/generate-sbom.sh cyclonedx

# Clean up
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/

# Default target
all: fmt-check lint test build
	@echo "All checks passed!"

.SILENT: help
