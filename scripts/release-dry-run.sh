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
MERGE_FROM="${MERGE_FROM:-}"
MERGE_INTO="${MERGE_INTO:-}"
MERGE_REPO="${MERGE_REPO:-$ROOT}"

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

if [[ -n "$MERGE_FROM" || -n "$MERGE_INTO" ]]; then
  if [[ -z "$MERGE_FROM" || -z "$MERGE_INTO" ]]; then
    echo "MERGE_FROM and MERGE_INTO are required together" >&2
    exit 1
  fi
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
        print("protected-path: allow")
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

if [[ -z "$MERGE_FROM" && -z "$MERGE_INTO" ]]; then
  echo "protected-path: allow"
elif [[ -n "$MERGE_FROM" && -n "$MERGE_INTO" ]]; then
  MERGE_FROM="$MERGE_FROM" \
  MERGE_INTO="$MERGE_INTO" \
  MERGE_REPO="$MERGE_REPO" \
  python3 - <<'PY'
import json
import os
import subprocess
import sys

merge_repo = os.environ["MERGE_REPO"]
merge_from = os.environ["MERGE_FROM"]
merge_into = os.environ["MERGE_INTO"]

PACKAGE_JSON = "client-sdk/package.json"
ZK_DEP = "@arcanetech/stellar-privacy-pool-zk-sdk"

PROTECTED_PATHS = [
    ("shapes catalog", "shapes.json"),
    ("circuit sources", "circuits"),
    ("contracts submodule commit", "soroban-privacy-pools"),
    ("release manifest", "artifacts/circuits-manifest.json"),
]


def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", merge_repo, *args],
        text=True,
        capture_output=True,
    )


def blob_at(commit: str, path: str) -> str | None:
    result = git("rev-parse", f"{commit}:{path}")
    if result.returncode != 0:
        return None
    return result.stdout.strip()


def show_at(commit: str, path: str) -> str | None:
    result = git("show", f"{commit}:{path}")
    if result.returncode != 0:
        return None
    return result.stdout


def paths_under(commit: str, prefix: str) -> list[str]:
    result = git("ls-tree", "-r", "--name-only", commit)
    if result.returncode != 0:
        return []
    paths = [
        line
        for line in result.stdout.splitlines()
        if line == prefix or line.startswith(prefix + "/")
    ]
    if blob_at(commit, prefix) is not None and prefix not in paths:
        paths.append(prefix)
    return sorted(set(paths))


def package_json(commit: str) -> dict | None:
    raw = show_at(commit, PACKAGE_JSON)
    if raw is None:
        return None
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        return None
    return data if isinstance(data, dict) else None


def package_version(commit: str):
    data = package_json(commit)
    return None if data is None else data.get("version")


def package_zk_dep_range(commit: str):
    data = package_json(commit)
    if data is None:
        return None
    deps = data.get("dependencies") or {}
    return deps.get(ZK_DEP)


merge = git(
    "merge-tree",
    "--write-tree",
    "--name-only",
    "--messages",
    merge_into,
    merge_from,
)
merge_out = (merge.stdout or "") + (merge.stderr or "")
lines = [ln for ln in merge_out.splitlines() if ln.strip()]
result_tree = lines[0] if lines and merge.returncode in (0, 1) else None
conflicted: set[str] = set()
if merge.returncode != 0:
    for ln in lines[1:]:
        if ln.startswith("CONFLICT"):
            marker = " in "
            if marker in ln:
                conflicted.add(ln.split(marker, 1)[1].strip())
        elif "\t" in ln:
            conflicted.add(ln.split("\t")[-1].strip())
        elif " " not in ln and ln != result_tree:
            conflicted.add(ln)


def path_in_conflict(path: str) -> bool:
    return any(
        c == path or c.startswith(path + "/") or path.startswith(c + "/")
        for c in conflicted
    )


def path_replaced_with_from(path: str) -> bool:
    into_paths = paths_under(merge_into, path)
    from_paths = paths_under(merge_from, path)
    all_paths = sorted(set(into_paths) | set(from_paths)) or [path]
    for p in all_paths:
        into_blob = blob_at(merge_into, p)
        from_blob = blob_at(merge_from, p)
        if into_blob == from_blob:
            continue
        if path_in_conflict(p):
            return True
        if result_tree is None:
            continue
        result_blob = blob_at(result_tree, p)
        if (
            result_blob is not None
            and from_blob is not None
            and result_blob == from_blob
            and result_blob != into_blob
        ):
            return True
    return False


def result_package_json() -> dict | None:
    if result_tree is None:
        return None
    return package_json(result_tree)


pkg_conflict = path_in_conflict(PACKAGE_JSON)
pkg_taken_from = path_replaced_with_from(PACKAGE_JSON) and not pkg_conflict
into_ver = package_version(merge_into)
from_ver = package_version(merge_from)
into_dep = package_zk_dep_range(merge_into)
from_dep = package_zk_dep_range(merge_from)

if pkg_conflict or pkg_taken_from:
    result_pkg = result_package_json() if pkg_taken_from else None
    result_ver = None if result_pkg is None else result_pkg.get("version")
    result_dep = None
    if result_pkg is not None:
        result_dep = (result_pkg.get("dependencies") or {}).get(ZK_DEP)

    version_becomes_from = into_ver != from_ver and (
        pkg_conflict or result_ver == from_ver
    )
    dep_becomes_from = into_dep != from_dep and (
        pkg_conflict or result_dep == from_dep
    )
    if version_becomes_from:
        print("protected-path: refuse package version")
        sys.exit(1)
    if dep_becomes_from:
        print("protected-path: refuse zk SDK dependency range")
        sys.exit(1)

for label, path in PROTECTED_PATHS:
    if path_replaced_with_from(path):
        print(f"protected-path: refuse {label}")
        sys.exit(1)

if merge.returncode != 0:
    print("protected-path: allow")
    print("merge: conflict")
    sys.exit(1)

print("protected-path: allow")
PY
fi

(
  cd "$ROOT/client-sdk"
  npm pack --dry-run
)

echo "dry-run: packed current package tree; skipped publish, git tag, artifact upload, and circuit keygen"
