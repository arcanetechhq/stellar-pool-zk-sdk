import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const script = resolve(root, "scripts/release-dry-run.sh");
const packageJsonPath = resolve(root, "client-sdk/package.json");

function runReleaseDryRun(commitMessage) {
  return spawnSync(script, [], {
    cwd: root,
    encoding: "utf8",
    env: {
      ...process.env,
      DRY_RUN: "1",
      RELEASE_LINE: "0",
      COMMIT_MESSAGE: commitMessage,
    },
  });
}

function tarballCount() {
  const roots = [root, resolve(root, "client-sdk")];
  let count = 0;
  for (const dir of roots) {
    count += readdirSync(dir).filter((name) => name.endsWith(".tgz")).length;
  }
  return count;
}

const packageBefore = readFileSync(packageJsonPath, "utf8");
const tagsBefore = spawnSync("git", ["tag", "--list"], {
  cwd: root,
  encoding: "utf8",
}).stdout;
const tarballsBefore = tarballCount();

function assertNoLocalSideEffects() {
  assert.equal(readFileSync(packageJsonPath, "utf8"), packageBefore);
  assert.equal(
    spawnSync("git", ["tag", "--list"], { cwd: root, encoding: "utf8" }).stdout,
    tagsBefore,
  );
  assert.equal(tarballCount(), tarballsBefore);
}

function assertLine0SuccessPlan(result, expectedVersion) {
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(
    result.stdout,
    new RegExp(`\\b${expectedVersion.replaceAll(".", "\\.")}\\b`),
  );
  assert.match(result.stdout, /\bv0\b/);
  assert.match(result.stdout, /\blatest\b/);
  assert.match(result.stdout, /stellar\/v0\/circuits-manifest\.json/);
  assert.match(result.stdout, /^breaking-commit: allow$/m);
  assert.match(result.stdout, /\.tgz\b/);
  assert.match(result.stdout, /packed current package tree; skipped publish/);
  assert.doesNotMatch(result.stdout, /\bnpm publish\b/);
  assertNoLocalSideEffects();
}

function assertLine0BreakingRefusal(result) {
  assert.notEqual(result.status, 0);
  assert.notEqual(result.status, null);
  assert.match(result.stdout, /^breaking-commit: refuse$/m);
  assert.doesNotMatch(result.stdout, /\bversion:\s*\d+\.\d+\.\d+\b/);
  assert.doesNotMatch(result.stdout, /\b0\.12\.0\b/);
  assert.doesNotMatch(result.stdout, /\b1\.0\.0\b/);
  assertNoLocalSideEffects();
}

assertLine0SuccessPlan(runReleaseDryRun("fix: example"), "0.11.2");
assertLine0SuccessPlan(runReleaseDryRun("feat: example"), "0.12.0");
assertLine0BreakingRefusal(runReleaseDryRun("feat!: example"));
assertLine0BreakingRefusal(
  runReleaseDryRun("feat: example\n\nBREAKING CHANGE: shapes"),
);

console.log("release-dry-run line 0 plan ok");
