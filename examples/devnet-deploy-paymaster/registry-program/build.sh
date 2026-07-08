#!/bin/bash
set -e
cd "$(dirname "$0")"
cargo build-sbf --manifest-path Cargo.toml
cp target/deploy/deploy_registry.so ./deploy_registry.so
echo "Program binary: ./deploy_registry.so"
