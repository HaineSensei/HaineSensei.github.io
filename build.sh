#!/bin/bash
set -e

echo "Installing zip utility..."
mkdir -p $HOME/bin
curl -L https://github.com/pmqs/zip/releases/download/v3.0/zip-3.0-linux-x86_64.tar.gz -o zip.tar.gz
tar -xzf zip.tar.gz -C $HOME/bin --strip-components=1
export PATH=$HOME/bin:$PATH
rm zip.tar.gz

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

echo "generating secret_lair zip"
zip -r -e -P Ch3dd4R dist/secret_lair.zip secret_lair

echo "Generating content manifest..."
python3 scripts/generate-manifest.py

echo "Build complete!"
