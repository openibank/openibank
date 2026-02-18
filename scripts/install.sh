#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="openibank"
DEFAULT_VERSION="v0.1.0"
RELEASE_BASE_DEFAULT="https://github.com/openibank/openibank/releases/download"
INSTALL_DIR_DEFAULT="${HOME}/.local/bin"

VERSION="${OPENIBANK_VERSION:-$DEFAULT_VERSION}"
RELEASE_BASE="${OPENIBANK_RELEASE_BASE:-$RELEASE_BASE_DEFAULT}"
INSTALL_DIR="${OPENIBANK_INSTALL_DIR:-$INSTALL_DIR_DEFAULT}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage:
  scripts/install.sh
  scripts/install.sh --dev

Environment variables:
  OPENIBANK_VERSION       Release tag (default: v0.1.0)
  OPENIBANK_RELEASE_BASE  Release base URL (default: GitHub Releases download URL)
  OPENIBANK_INSTALL_DIR   Install directory (default: ~/.local/bin)

Modes:
  default     Download release artifact, verify SHA-256 with checksums.txt, install binary
  --dev       Print sibling-repo development install instructions (expects ../maple)
USAGE
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}/${arch}" in
    Darwin/arm64|Darwin/aarch64) echo "aarch64-apple-darwin" ;;
    Darwin/x86_64) echo "x86_64-apple-darwin" ;;
    Linux/x86_64) echo "x86_64-unknown-linux-gnu" ;;
    Linux/aarch64) echo "aarch64-unknown-linux-gnu" ;;
    *)
      echo "unsupported platform: ${os}/${arch}" >&2
      return 1
      ;;
  esac
}

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return 0
  fi
  echo "missing sha256 tool (need sha256sum or shasum)" >&2
  return 1
}

dev_mode() {
  local maple_dir maple_rev
  maple_dir="$(cd "${REPO_ROOT}/.." && pwd)/maple"

  if [[ ! -d "${maple_dir}" ]]; then
    echo "Maple repo not found: ${maple_dir}" >&2
    echo "Expected sibling layout:"
    echo "  $(cd "${REPO_ROOT}/.." && pwd)/openibank"
    echo "  $(cd "${REPO_ROOT}/.." && pwd)/maple"
    exit 1
  fi

  maple_rev="unknown"
  if command -v git >/dev/null 2>&1; then
    maple_rev="$(git -C "${maple_dir}" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  fi

  cat <<EOF
Dev mode detected.
OpenIBank repo: ${REPO_ROOT}
Maple repo:    ${maple_dir}
Maple git rev: ${maple_rev}

Build/install with sibling path dependencies:
  cd "${REPO_ROOT}"
  cargo install --path crates/openibank-cli --force

Run demo:
  openibank demo --seed 42
EOF
}

release_mode() {
  local target artifact checksums_url artifact_url tmp_dir checksums_file artifact_file
  local expected actual extracted_bin

  target="$(detect_target)"
  artifact="${BIN_NAME}-${VERSION}-${target}.tar.gz"
  checksums_url="${RELEASE_BASE}/${VERSION}/checksums.txt"
  artifact_url="${RELEASE_BASE}/${VERSION}/${artifact}"

  tmp_dir="$(mktemp -d)"
  checksums_file="${tmp_dir}/checksums.txt"
  artifact_file="${tmp_dir}/${artifact}"

  trap 'rm -rf "${tmp_dir}"' EXIT

  echo "Downloading checksums: ${checksums_url}"
  curl -fsSL "${checksums_url}" -o "${checksums_file}"

  echo "Downloading artifact: ${artifact_url}"
  curl -fsSL "${artifact_url}" -o "${artifact_file}"

  expected="$(awk -v file="${artifact}" '$2==file {print $1}' "${checksums_file}" | head -n1)"
  if [[ -z "${expected}" ]]; then
    echo "checksum entry not found for ${artifact}" >&2
    exit 1
  fi

  actual="$(sha256_file "${artifact_file}")"
  if [[ "${expected}" != "${actual}" ]]; then
    echo "checksum mismatch for ${artifact}" >&2
    echo "expected: ${expected}" >&2
    echo "actual:   ${actual}" >&2
    exit 1
  fi
  echo "SHA-256 verified."

  tar -xzf "${artifact_file}" -C "${tmp_dir}"

  extracted_bin="$(find "${tmp_dir}" -type f -name "${BIN_NAME}" -perm -u+x | head -n1 || true)"
  if [[ -z "${extracted_bin}" ]]; then
    echo "binary ${BIN_NAME} not found inside artifact" >&2
    exit 1
  fi

  mkdir -p "${INSTALL_DIR}"
  install -m 0755 "${extracted_bin}" "${INSTALL_DIR}/${BIN_NAME}"

  echo "Installed: ${INSTALL_DIR}/${BIN_NAME}"
  if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    echo "Add to PATH if needed:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  fi
}

main() {
  case "${1:-}" in
    --help|-h)
      usage
      exit 0
      ;;
    --dev)
      dev_mode
      exit 0
      ;;
    "")
      release_mode
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
}

main "$@"

