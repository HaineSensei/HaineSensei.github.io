#!/bin/bash
set -e

echo "Installing zip utility..."
mkdir -p $HOME/bin
curl -L http://ftp.uk.debian.org/debian/pool/main/z/zip/zip_3.0-13_amd64.deb -o zip.deb
ar x zip.deb data.tar.xz
tar -xf data.tar.xz
cp usr/bin/zip $HOME/bin/
chmod +x $HOME/bin/zip
export PATH=$HOME/bin:$PATH
rm -rf zip.deb data.tar.xz usr

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
zip -r -e -P my_password dist/secret_lair.zip secret_lair

echo "Generating content manifest..."
python3 scripts/generate-manifest.py

echo "Build complete!"
