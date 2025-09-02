#!/bin/bash

# Build script for the transfer hook example program

set -e

echo "Building transfer-hook-example program..."

# Build the program
cargo build-sbf --manifest-path Cargo.toml

# Copy the .so file to the expected location for makefile
cp target/deploy/transfer_hook_example.so ./transfer_hook_example.so

# The output will be in target/deploy/transfer_hook_example.so
echo "Build complete! Program binary at: target/deploy/transfer_hook_example.so"
echo "Program binary copied to: ./transfer_hook_example.so"