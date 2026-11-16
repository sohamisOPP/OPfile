#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_DIR="$PROJECT_ROOT/rust-core"
JNI_LIBS_DIR="$PROJECT_ROOT/android/app/src/main/jniLibs"

export ANDROID_NDK_HOME="/home/soham/Android/Sdk/ndk/26.3.11579264"
export NDK_HOME="$ANDROID_NDK_HOME"
export RUSTFLAGS="--remap-path-prefix=${HOME}=~ --remap-path-prefix=${SCRIPT_DIR}/..=. --remap-path-prefix=/home/soham=~"

echo "=== Building OPfile Rust Core with cargo-ndk ==="
echo "NDK: $ANDROID_NDK_HOME"
echo "Output: $JNI_LIBS_DIR"
echo "RUSTFLAGS: $RUSTFLAGS"

mkdir -p "$JNI_LIBS_DIR/arm64-v8a"
mkdir -p "$JNI_LIBS_DIR/armeabi-v7a"
mkdir -p "$JNI_LIBS_DIR/x86_64"

cd "$RUST_DIR"

cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$JNI_LIBS_DIR" build --release
# Remove dead-weight cdylib artifacts that are already statically linked into libopfile_core
rm -f "$JNI_LIBS_DIR"/*/libsevenz_rust.so

echo "=== Verification of Compiled Libraries ==="
ls -lh "$JNI_LIBS_DIR"/*/*.so
echo "=== Rust Core Multi-ABI Build Complete ==="
