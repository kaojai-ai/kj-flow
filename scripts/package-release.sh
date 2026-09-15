#!/usr/bin/env bash
set -euo pipefail

target="${1:?usage: scripts/package-release.sh <target> [output-directory]}"
output_directory="${2:-dist}"
repository_root="$(cd "$(dirname "$0")/.." && pwd -P)"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin|aarch64-unknown-linux-gnu|x86_64-unknown-linux-gnu) ;;
  *)
    printf 'Unsupported release target: %s\n' "$target" >&2
    exit 1
    ;;
esac

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) native_target="aarch64-apple-darwin" ;;
  Darwin:x86_64) native_target="x86_64-apple-darwin" ;;
  Linux:aarch64|Linux:arm64) native_target="aarch64-unknown-linux-gnu" ;;
  Linux:x86_64) native_target="x86_64-unknown-linux-gnu" ;;
  *)
    printf 'Unsupported build host: %s %s\n' "$(uname -s)" "$(uname -m)" >&2
    exit 1
    ;;
esac

if [[ "$target" != "$native_target" ]]; then
  printf 'This script builds native targets only (host: %s).\n' "$native_target" >&2
  exit 1
fi

cd "$repository_root"
cargo build --release --locked

mkdir -p "$output_directory"
staging_directory="$(mktemp -d)"
cleanup() {
  rm -rf "$staging_directory"
}
trap cleanup EXIT

install -m 755 target/release/kj "$staging_directory/kj"
cp LICENSE NOTICE README.md "$staging_directory"
tar -C "$staging_directory" -czf "$output_directory/kj-${target}.tar.gz" kj LICENSE NOTICE README.md
