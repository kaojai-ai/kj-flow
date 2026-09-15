#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "$0")/.." && pwd -P)"
temporary_directory="$(mktemp -d)"
cleanup() {
  rm -rf "$temporary_directory"
}
trap cleanup EXIT

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) target="aarch64-apple-darwin" ;;
  Darwin:x86_64) target="x86_64-apple-darwin" ;;
  Linux:aarch64|Linux:arm64) target="aarch64-unknown-linux-gnu" ;;
  Linux:x86_64) target="x86_64-unknown-linux-gnu" ;;
  *)
    printf 'Unsupported test host: %s %s\n' "$(uname -s)" "$(uname -m)" >&2
    exit 1
    ;;
esac

"${repository_root}/scripts/package-release.sh" "$target" "${temporary_directory}/release"
(
  cd "${temporary_directory}/release"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 kj-*.tar.gz > kj-checksums.txt
  else
    sha256sum kj-*.tar.gz > kj-checksums.txt
  fi
)

KJ_FLOW_DOWNLOAD_BASE="file://${temporary_directory}/release" \
  KJ_INSTALL_DIR="${temporary_directory}/bin" \
  "${repository_root}/scripts/install.sh"
"${temporary_directory}/bin/kj" --version

printf '0000000000000000000000000000000000000000000000000000000000000000  kj-%s.tar.gz\n' "$target" \
  > "${temporary_directory}/release/kj-checksums.txt"
if KJ_FLOW_DOWNLOAD_BASE="file://${temporary_directory}/release" \
  KJ_INSTALL_DIR="${temporary_directory}/bad-checksum-bin" \
  "${repository_root}/scripts/install.sh"; then
  printf 'Installer accepted an invalid checksum.\n' >&2
  exit 1
fi
