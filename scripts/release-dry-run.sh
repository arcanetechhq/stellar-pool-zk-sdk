#!/usr/bin/env bash
# Dry-run the zk SDK release plan for a release line.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_JSON="$ROOT/client-sdk/package.json"

DRY_RUN="${DRY_RUN:-0}"
RELEASE_LINE="${RELEASE_LINE:-}"
COMMIT_MESSAGE="${COMMIT_MESSAGE:-}"

if [[ "$DRY_RUN" != "1" ]]; then
  echo "release-dry-run.sh requires DRY_RUN=1" >&2
  exit 1
fi

if [[ -z "$RELEASE_LINE" ]]; then
  echo "RELEASE_LINE is required" >&2
  exit 1
fi

if [[ -z "$COMMIT_MESSAGE" ]]; then
  echo "COMMIT_MESSAGE is required" >&2
  exit 1
fi

if [[ "$RELEASE_LINE" != "0" ]]; then
  echo "RELEASE_LINE=${RELEASE_LINE} is not supported yet" >&2
  exit 1
fi

current_version="$(python3 -c 'import json; print(json.load(open("'"$PACKAGE_JSON"'"))["version"])')"

COMMIT_MESSAGE="$COMMIT_MESSAGE" CURRENT_VERSION="$current_version" python3 - <<'PY'
import os
import re
import sys

message = os.environ["COMMIT_MESSAGE"]
version = os.environ["CURRENT_VERSION"]

breaking = bool(re.search(r"^(\w+)(\(.+\))?!:", message, re.M)) or (
    "BREAKING CHANGE:" in message
)
if breaking:
    print("breaking-commit: refuse")
    print("circuits-manifest: stellar/v0/circuits-manifest.json")
    sys.exit(1)

base = re.sub(r"[-+].*$", "", version)
parts = [int(p) for p in base.split(".")]
if len(parts) != 3:
    print(f"unsupported version: {version}", file=sys.stderr)
    sys.exit(1)
major, minor, patch = parts

first = message.split("\n", 1)[0]
kind_match = re.match(r"^(\w+)(\(.+\))?!?:", first)
kind = kind_match.group(1) if kind_match else ""

if kind == "feat":
    minor += 1
    patch = 0
elif kind == "fix":
    patch += 1
else:
    print(f"unsupported commit type in message: {message!r}", file=sys.stderr)
    sys.exit(1)

print(f"version: {major}.{minor}.{patch}")
print("dist-tags: v0 latest")
print("circuits-manifest: stellar/v0/circuits-manifest.json")
print("breaking-commit: allow")
PY

(
  cd "$ROOT/client-sdk"
  npm pack --dry-run
)

echo "dry-run: packed current package tree; skipped publish, git tag, artifact upload, and circuit keygen"
