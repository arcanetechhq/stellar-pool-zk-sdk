#!/usr/bin/env bash
# Materialize only powersOfTau28_hez_final_21.ptau from the pinned hermez-ptau submodule.
# Does not clone the rest of Aptos LFS objects (zkeys, wasm, r1cs).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PTAU_DIR="${PTAU_DIR:-$ROOT/hermez-ptau}"
PTAU_FILE="powersOfTau28_hez_final_21.ptau"
PTAU_PATH="$PTAU_DIR/$PTAU_FILE"
EXPECTED_B2SUM="9aef0573cef4ded9c4a75f148709056bf989f80dad96876aadeb6f1c6d062391f07a394a9e756d16f7eb233198d5b69407cca44594c763ab4a5b67ae73254678"

cd "$ROOT"

pin="$(git ls-tree HEAD hermez-ptau | awk '{print $3}')"
if [[ -z "$pin" ]]; then
  echo "hermez-ptau gitlink is missing from HEAD" >&2
  exit 1
fi

export GIT_LFS_SKIP_SMUDGE=1
git submodule update --init --depth 1 -- hermez-ptau || true
if [[ ! -e "$PTAU_DIR/.git" ]]; then
  url="$(git config -f .gitmodules --get submodule.hermez-ptau.url)"
  GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1 "$url" "$PTAU_DIR"
fi
git -C "$PTAU_DIR" fetch --depth 1 origin "$pin"
git -C "$PTAU_DIR" checkout --detach "$pin"

if [[ ! -f "$PTAU_PATH" ]]; then
  echo "missing $PTAU_PATH after submodule checkout" >&2
  exit 1
fi

git -C "$PTAU_DIR" lfs pull --include="$PTAU_FILE" --exclude=""

if head -n 1 "$PTAU_PATH" | grep -q 'git-lfs'; then
  echo "$PTAU_PATH is still a Git LFS pointer" >&2
  exit 1
fi

if ! command -v b2sum >/dev/null 2>&1; then
  echo "b2sum is required to verify the Hermez ptau" >&2
  exit 1
fi

actual="$(b2sum "$PTAU_PATH" | awk '{print $1}')"
if [[ "$actual" != "$EXPECTED_B2SUM" ]]; then
  echo "Hermez ptau BLAKE2b mismatch" >&2
  echo "expected $EXPECTED_B2SUM" >&2
  echo "actual   $actual" >&2
  exit 1
fi

echo "hermez ptau ready: $PTAU_PATH"
