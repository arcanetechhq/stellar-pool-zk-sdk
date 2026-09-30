#!/usr/bin/env bash
# Dry-run the zk SDK release plan for a release line.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_JSON="$ROOT/client-sdk/package.json"

DRY_RUN="${DRY_RUN:-0}"
RELEASE_LINE="${RELEASE_LINE:-}"
COMMIT_MESSAGE="${COMMIT_MESSAGE:-}"
PROMOTE_STABLE="${PROMOTE_STABLE:-0}"
STABLE_1X_PUBLISHED="${STABLE_1X_PUBLISHED:-0}"

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

if [[ "$RELEASE_LINE" != "0" && "$RELEASE_LINE" != "1" ]]; then
  echo "RELEASE_LINE=${RELEASE_LINE} is not supported" >&2
  exit 1
fi

current_version="$(python3 -c 'import json; print(json.load(open("'"$PACKAGE_JSON"'"))["version"])')"

COMMIT_MESSAGE="$COMMIT_MESSAGE" \
CURRENT_VERSION="$current_version" \
RELEASE_LINE="$RELEASE_LINE" \
PROMOTE_STABLE="$PROMOTE_STABLE" \
STABLE_1X_PUBLISHED="$STABLE_1X_PUBLISHED" \
python3 - <<'PY'
import os
import re
import sys

message = os.environ["COMMIT_MESSAGE"]
version = os.environ["CURRENT_VERSION"]
release_line = os.environ["RELEASE_LINE"]
promote_stable = os.environ.get("PROMOTE_STABLE", "0") == "1"
stable_1x_published = os.environ.get("STABLE_1X_PUBLISHED", "0") == "1"

breaking = bool(re.search(r"^(\w+)(\(.+\))?!:", message, re.M)) or (
    "BREAKING CHANGE:" in message
)

def parse_semver(raw: str):
    base = re.sub(r"[-+].*$", "", raw)
    parts = [int(p) for p in base.split(".")]
    if len(parts) != 3:
        print(f"unsupported version: {raw}", file=sys.stderr)
        sys.exit(1)
    return parts[0], parts[1], parts[2]

def commit_kind(msg: str) -> str:
    first = msg.split("\n", 1)[0]
    kind_match = re.match(r"^(\w+)(\(.+\))?!?:", first)
    return kind_match.group(1) if kind_match else ""

if release_line == "0":
    if breaking:
        print("breaking-commit: refuse")
        print("circuits-manifest: stellar/v0/circuits-manifest.json")
        sys.exit(1)

    major, minor, patch = parse_semver(version)
    kind = commit_kind(message)
    if kind == "feat":
        minor += 1
        patch = 0
    elif kind == "fix":
        patch += 1
    else:
        print(f"unsupported commit type in message: {message!r}", file=sys.stderr)
        sys.exit(1)

    print(f"version: {major}.{minor}.{patch}")
    if stable_1x_published:
        print("dist-tags: v0")
    else:
        print("dist-tags: v0 latest")
    print("circuits-manifest: stellar/v0/circuits-manifest.json")
    print("breaking-commit: allow")
    sys.exit(0)

# release_line == "1"
if promote_stable:
    print("version: 1.0.0")
    print("dist-tags: latest")
    print("circuits-manifest: stellar/v1/circuits-manifest.json")
    print("breaking-commit: allow")
    sys.exit(0)

if not stable_1x_published:
    # Before promotion: any conventional commit (including breaking) plans the next 1.0.0-rc.N.
    # Package is still on 0.x (e.g. 0.11.1-rc.4), so the first line-1 RC is 1.0.0-rc.0.
    kind = commit_kind(message)
    if kind not in ("fix", "feat"):
        print(f"unsupported commit type in message: {message!r}", file=sys.stderr)
        sys.exit(1)
    major, _, _ = parse_semver(version)
    if major >= 1 and version.startswith("1.0.0-rc."):
        rc_n = int(version.rsplit(".", 1)[-1]) + 1
        next_version = f"1.0.0-rc.{rc_n}"
    else:
        next_version = "1.0.0-rc.0"
    print(f"version: {next_version}")
    print("dist-tags: next")
    print("circuits-manifest: stellar/v1/circuits-manifest.json")
    print("breaking-commit: allow")
    sys.exit(0)

# After promotion: stable 1.x bumps on latest (fix=patch, feat=minor).
kind = commit_kind(message)
major, minor, patch = parse_semver(version)
if major < 1:
    # Dry-run does not mutate package.json; when stable 1.x is signaled but the
    # tree is still on 0.x, bumps start from the promoted 1.0.0.
    major, minor, patch = 1, 0, 0

if kind == "feat":
    minor += 1
    patch = 0
elif kind == "fix":
    patch += 1
else:
    print(f"unsupported commit type in message: {message!r}", file=sys.stderr)
    sys.exit(1)

print(f"version: {major}.{minor}.{patch}")
print("dist-tags: latest")
print("circuits-manifest: stellar/v1/circuits-manifest.json")
print("breaking-commit: allow")
PY

(
  cd "$ROOT/client-sdk"
  npm pack --dry-run
)

echo "dry-run: packed current package tree; skipped publish, git tag, artifact upload, and circuit keygen"
