# SSD1677 Driver - Justfile
# Common development tasks

# Default recipe - show available commands
default:
    @just --list

# Setup development environment (install rust components)
setup:
    @echo "🔧 Setting up development environment..."
    @echo "Installing required rustup components..."
    rustup component add rustfmt clippy
    @echo "✅ Setup complete! Run 'just all' to verify everything works."

# Run all checks (format, lint, type-check, test, doc, doctest)
all: format lint type-check test doc-test doc
    @echo "✅ All checks passed!"

# Format code with rustfmt
format:
    @echo "🎨 Formatting code..."
    cargo fmt

# Check formatting without modifying files
check-format:
    @echo "🔍 Checking code formatting..."
    cargo fmt -- --check

# Run clippy lints
lint:
    @echo "🔍 Running clippy..."
    cargo clippy --all-features -- -D warnings

# Run clippy with pedantic lints (stricter)
lint-strict:
    @echo "🔍 Running clippy (strict)..."
    cargo clippy --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery

# Type check the code
type-check:
    @echo "🔍 Type checking..."
    cargo check --all-features

# Type check without graphics feature
type-check-minimal:
    @echo "🔍 Type checking (minimal)..."
    cargo check --no-default-features

# Build the crate
build:
    @echo "🔨 Building..."
    cargo build --all-features

# Build release version
build-release:
    @echo "🔨 Building release..."
    cargo build --release --all-features

# Run tests
test:
    @echo "🧪 Running tests..."
    cargo test --all-features

# Run doctests (docs.rs-like examples)
doc-test:
    @echo "🧪 Running doc tests..."
    cargo test --doc

# Run doctests marked as ignored (if any)
doc-test-ignored:
    @echo "🧪 Running ignored doc tests..."
    cargo test --doc -- --ignored

# Run tests without graphics
test-minimal:
    @echo "🧪 Running tests (minimal)..."
    cargo test --no-default-features

# Build documentation
doc:
    @echo "📚 Building documentation..."
    cargo doc --all-features --no-deps

# Build and open documentation
doc-open:
    @echo "📚 Building and opening documentation..."
    cargo doc --all-features --no-deps --open

# Clean build artifacts
clean:
    @echo "🧹 Cleaning..."
    cargo clean


# Dry run publish to verify everything is ready
publish-dry:
    @echo "📦 Testing publish (dry run)..."
    cargo publish --dry-run

# Full CI simulation locally
ci: check-format lint type-check test doc-test doc publish-dry
    @echo "✅ CI checks passed!"

# Fix common issues automatically
fix: format
    @echo "🔧 Running cargo fix..."
    cargo fix --all-features --allow-dirty --allow-staged

# Update dependencies
update:
    @echo "📦 Updating dependencies..."
    cargo update

# Show dependency tree
tree:
    cargo tree

# Show dependency tree with features
tree-features:
    cargo tree -e features

# Generate coverage report (requires cargo-tarpaulin)
coverage:
    @echo "📊 Generating coverage report..."
    cargo tarpaulin --all-features --out Html

# Check code size (requires cargo-bloat)
size:
    @echo "📊 Checking code size..."
    cargo bloat --all-features --release

# Run benchmarks (if any)
bench:
    @echo "⚡ Running benchmarks..."
    cargo bench --all-features

# Check for outdated dependencies (requires cargo-outdated)
outdated:
    @echo "📦 Checking for outdated dependencies..."
    cargo outdated

# Quick development cycle: check + test
dev:
    @echo "🔄 Quick dev cycle..."
    cargo check --all-features && cargo test --all-features
