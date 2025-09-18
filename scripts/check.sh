#!/bin/bash
# Strict clippy check for the jacket-finder project
# This enforces high code quality standards

echo "Running strict clippy check..."
cargo clippy --workspace --release --all-targets --all-features -- \
    --deny warnings \
    -D warnings \
    -W clippy::correctness \
    -W clippy::suspicious \
    -W clippy::complexity \
    -W clippy::perf \
    -W clippy::style \
    -W clippy::pedantic

if [ $? -eq 0 ]; then
    echo "✅ All clippy checks passed!"
else
    echo "❌ Clippy checks failed - please fix the issues above"
    exit 1
fi
