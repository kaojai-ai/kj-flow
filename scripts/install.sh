#!/usr/bin/env bash
set -euo pipefail

repository="kaojai-ai/kj-flow"
download_base="${KJ_FLOW_DOWNLOAD_BASE:-https://github.com/${repository}/releases/latest/download}"
install_directory="${KJ_INSTALL_DIR:-${HOME}/.local/bin}"

fail() {
  printf 'kj installer: %s\n' "$*" >&2
  exit 1
}

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) target="aarch64-apple-darwin" ;;
  Darwin:x86_64) target="x86_64-apple-darwin" ;;
  Linux:aarch64|Linux:arm64) target="aarch64-unknown-linux-gnu" ;;
  Linux:x86_64) target="x86_64-unknown-linux-gnu" ;;
  *) fail "unsupported platform $(uname -s) $(uname -m); download a supported release manually" ;;
esac

command -v curl >/dev/null 2>&1 || fail "curl is required to download KJ Flow"
command -v tar >/dev/null 2>&1 || fail "tar is required to unpack KJ Flow"

temporary_directory="$(mktemp -d)"
cleanup() {
  rm -rf "$temporary_directory"
}
trap cleanup EXIT

archive="kj-${target}.tar.gz"
checksums="kj-checksums.txt"
curl --fail --location --silent --show-error "${download_base}/${archive}" --output "${temporary_directory}/${archive}"
curl --fail --location --silent --show-error "${download_base}/${checksums}" --output "${temporary_directory}/${checksums}"

if command -v shasum >/dev/null 2>&1; then
  (
    cd "$temporary_directory"
    shasum -a 256 -c "$checksums" --ignore-missing
  )
elif command -v sha256sum >/dev/null 2>&1; then
  (
    cd "$temporary_directory"
    sha256sum -c "$checksums" --ignore-missing
  )
else
  fail "shasum or sha256sum is required to verify the download"
fi

tar -xzf "${temporary_directory}/${archive}" -C "$temporary_directory"
[[ -f "${temporary_directory}/kj" ]] || fail "release archive did not contain kj"

mkdir -p "$install_directory"
install -m 755 "${temporary_directory}/kj" "${install_directory}/kj"
printf 'Installed kj to %s/kj\n' "$install_directory"
case ":${PATH}:" in
  *":${install_directory}:"*) ;;
  *) printf 'Add %s to PATH, then open a new terminal.\n' "$install_directory" ;;
esac
