#!/bin/bash
set -e

echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

echo "Adding wasm32 target..."
rustup target add wasm32-unknown-unknown

echo "Installing wasm-pack..."
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

echo "Creating dist directory..."
mkdir -p dist/pkg

echo "Building WASM..."
cd rust
wasm-pack build --target web --out-dir ../dist/pkg
cd ..

echo "Copying site files..."
cp -r site/* dist/

echo "Generating content manifest..."
python3 scripts/generate-manifest.py

echo "Build complete!"
