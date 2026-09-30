import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const script = resolve(root, "scripts/release-dry-run.sh");
const packageJsonPath = resolve(root, "client-sdk/package.json");

function runReleaseDryRun({
  releaseLine,
  commitMessage,
  promoteStable,
  stable1xPublished,
  mergeFrom,
  mergeInto,
  mergeRepo,
} = {}) {
  const env = {
    ...process.env,
    DRY_RUN: "1",
    RELEASE_LINE: releaseLine,
    COMMIT_MESSAGE: commitMessage,
  };
  if (promoteStable !== undefined) {
    env.PROMOTE_STABLE = promoteStable;
  }
  if (stable1xPublished !== undefined) {
    env.STABLE_1X_PUBLISHED = stable1xPublished;
  }
  if (mergeFrom !== undefined) {
    env.MERGE_FROM = mergeFrom;
  }
  if (mergeInto !== undefined) {
    env.MERGE_INTO = mergeInto;
  }
  if (mergeRepo !== undefined) {
    env.MERGE_REPO = mergeRepo;
  }
  return spawnSync(script, [], {
    cwd: root,
    encoding: "utf8",
    env,
  });
}

function git(cwd, args) {
  const result = spawnSync(
    "git",
    ["-c", "core.hooksPath=/dev/null", "-c", "init.templateDir=", ...args],
    {
      cwd,
      encoding: "utf8",
      env: {
        ...process.env,
        GIT_CONFIG_NOSYSTEM: "1",
        GIT_CONFIG_GLOBAL: "/dev/null",
        GIT_TEMPLATE_DIR: "",
      },
    },
  );
  assert.equal(
    result.status,
    0,
    `git ${args.join(" ")} failed: ${result.stderr || result.stdout}`,
  );
  return result.stdout.trim();
}

function writeTree(dir, files) {
  for (const [rel, content] of Object.entries(files)) {
    const abs = join(dir, rel);
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, content);
  }
}

function createMergeFixture(mutate) {
  const scratch = join(root, ".scratch");
  mkdirSync(scratch, { recursive: true });
  const dir = mkdtempSync(join(scratch, "zk-sdk-merge-"));
  git(dir, ["init", "-q"]);
  git(dir, ["config", "user.email", "merge-fixture@test"]);
  git(dir, ["config", "user.name", "merge-fixture"]);

  const baseFiles = {
    "client-sdk/package.json":
      '{\n  "name": "@arcanetech/stellar-privacy-pool-zk-sdk",\n  "version": "1.0.0-rc.0",\n  "description": "base"\n}\n',
    "shapes.json": '{"shapes":[{"id":"2x2"}]}\n',
    "circuits/main.circom": "base circuit\n",
    "soroban-privacy-pools": "submodule-commit-base\n",
    "artifacts/circuits-manifest.json": '{"circuits":{}}\n',
    "sdk/unprotected.js": "base unprotected\n",
  };
  writeTree(dir, baseFiles);
  git(dir, ["add", "."]);
  git(dir, ["commit", "-qm", "base"]);
  const base = git(dir, ["rev-parse", "HEAD"]);

  git(dir, ["checkout", "-qb", "line0"]);
  const line0Files = { ...baseFiles };
  mutate.line0?.(line0Files);
  writeTree(dir, line0Files);
  git(dir, ["add", "-A"]);
  git(dir, ["commit", "-qm", "line0 tip"]);
  const line0 = git(dir, ["rev-parse", "HEAD"]);

  git(dir, ["checkout", "-q", base]);
  git(dir, ["checkout", "-qb", "line1"]);
  const line1Files = { ...baseFiles };
  mutate.line1?.(line1Files);
  writeTree(dir, line1Files);
  git(dir, ["add", "-A"]);
  git(dir, ["commit", "--allow-empty", "-qm", "line1 tip"]);
  const line1 = git(dir, ["rev-parse", "HEAD"]);

  return {
    dir,
    line0,
    line1,
    cleanup() {
      rmSync(dir, { recursive: true, force: true });
    },
  };
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
  assert.match(result.stdout, /dist-tags: sdk-v0 latest/);
  assert.match(result.stdout, /\blatest\b/);
  assert.match(result.stdout, /stellar\/v0\/circuits-manifest\.json/);
  assert.match(result.stdout, /^breaking-commit: allow$/m);
  assert.match(result.stdout, /^protected-path: allow$/m);
  assert.match(result.stdout, /\.tgz\b/);
  assert.match(result.stdout, /packed current package tree; skipped publish/);
  assert.doesNotMatch(result.stdout, /\bnpm publish\b/);
  assertNoLocalSideEffects();
}

function assertLine0BreakingRefusal(result) {
  assert.notEqual(result.status, 0);
  assert.notEqual(result.status, null);
  assert.match(result.stdout, /^breaking-commit: refuse$/m);
  assert.match(result.stdout, /^protected-path: allow$/m);
  assert.doesNotMatch(result.stdout, /\bversion:\s*\d+\.\d+\.\d+\b/);
  assert.doesNotMatch(result.stdout, /\b0\.12\.0\b/);
  assert.doesNotMatch(result.stdout, /\b1\.0\.0\b/);
  assertNoLocalSideEffects();
}

assertLine0SuccessPlan(
  runReleaseDryRun({ releaseLine: "0", commitMessage: "fix: example" }),
  "0.11.3",
);
assertLine0SuccessPlan(
  runReleaseDryRun({ releaseLine: "0", commitMessage: "feat: example" }),
  "0.12.0",
);
assertLine0BreakingRefusal(
  runReleaseDryRun({ releaseLine: "0", commitMessage: "feat!: example" }),
);
assertLine0BreakingRefusal(
  runReleaseDryRun({
    releaseLine: "0",
    commitMessage: "feat: example\n\nBREAKING CHANGE: shapes",
  }),
);

console.log("release-dry-run line 0 plan ok");

function assertLine1RcPlan(result, expectedVersion) {
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(
    result.stdout,
    new RegExp(`\\bversion:\\s*${expectedVersion.replaceAll(".", "\\.")}\\b`),
  );
  assert.match(result.stdout, /\bnext\b/);
  assert.doesNotMatch(result.stdout, /\blatest\b/);
  assert.match(result.stdout, /stellar\/v1\/circuits-manifest\.json/);
  assert.match(result.stdout, /^breaking-commit: allow$/m);
  assert.match(result.stdout, /^protected-path: allow$/m);
  assert.match(result.stdout, /\.tgz\b/);
  assert.match(result.stdout, /packed current package tree; skipped publish/);
  assertNoLocalSideEffects();
}

assertLine1RcPlan(
  runReleaseDryRun({ releaseLine: "1", commitMessage: "fix: example" }),
  "1.0.0-rc.0",
);
assertLine1RcPlan(
  runReleaseDryRun({ releaseLine: "1", commitMessage: "feat: example" }),
  "1.0.0-rc.0",
);
assertLine1RcPlan(
  runReleaseDryRun({ releaseLine: "1", commitMessage: "feat!: shapes" }),
  "1.0.0-rc.0",
);

function assertLine1PromotionPlan(result) {
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /\bversion:\s*1\.0\.0\b/);
  assert.match(result.stdout, /\blatest\b/);
  assert.doesNotMatch(result.stdout, /\bnext\b/);
  assert.doesNotMatch(result.stdout, /\bv0\b/);
  assert.match(result.stdout, /stellar\/v1\/circuits-manifest\.json/);
  assert.match(result.stdout, /^protected-path: allow$/m);
  assert.match(result.stdout, /\.tgz\b/);
  assert.match(result.stdout, /packed current package tree; skipped publish/);
  assertNoLocalSideEffects();
}

assertLine1PromotionPlan(
  runReleaseDryRun({
    releaseLine: "1",
    commitMessage: "chore: promote",
    promoteStable: "1",
  }),
);

function assertLine1StablePlan(result, expectedVersion) {
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(
    result.stdout,
    new RegExp(`\\bversion:\\s*${expectedVersion.replaceAll(".", "\\.")}\\b`),
  );
  assert.match(result.stdout, /\blatest\b/);
  assert.doesNotMatch(result.stdout, /\bnext\b/);
  assert.doesNotMatch(result.stdout, /\bv0\b/);
  assert.match(result.stdout, /stellar\/v1\/circuits-manifest\.json/);
  assert.match(result.stdout, /^protected-path: allow$/m);
  assert.match(result.stdout, /\.tgz\b/);
  assert.match(result.stdout, /packed current package tree; skipped publish/);
  assertNoLocalSideEffects();
}

assertLine1StablePlan(
  runReleaseDryRun({
    releaseLine: "1",
    commitMessage: "fix: example",
    stable1xPublished: "1",
  }),
  "1.0.1",
);
assertLine1StablePlan(
  runReleaseDryRun({
    releaseLine: "1",
    commitMessage: "feat: example",
    stable1xPublished: "1",
  }),
  "1.1.0",
);

function assertLine0AfterPromotion(result, expectedVersion) {
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(
    result.stdout,
    new RegExp(`\\bversion:\\s*${expectedVersion.replaceAll(".", "\\.")}\\b`),
  );
  assert.match(result.stdout, /dist-tags: sdk-v0$/m);
  assert.doesNotMatch(result.stdout, /\blatest\b/);
  assert.match(result.stdout, /stellar\/v0\/circuits-manifest\.json/);
  assert.match(result.stdout, /^protected-path: allow$/m);
  assert.match(result.stdout, /\.tgz\b/);
  assert.match(result.stdout, /packed current package tree; skipped publish/);
  assertNoLocalSideEffects();
}

assertLine0AfterPromotion(
  runReleaseDryRun({
    releaseLine: "0",
    commitMessage: "fix: example",
    stable1xPublished: "1",
  }),
  "0.11.3",
);

console.log("release-dry-run line 1 plan ok");

{
  const fixture = createMergeFixture({
    line0(files) {
      files["shapes.json"] = '{"shapes":[{"id":"0x-line"}]}\n';
    },
    line1(files) {
      files["shapes.json"] = '{"shapes":[{"id":"1x-line"}]}\n';
      files["sdk/unprotected.js"] = "line1 only change\n";
    },
  });
  try {
    const result = runReleaseDryRun({
      releaseLine: "1",
      commitMessage: "fix: example",
      mergeFrom: fixture.line0,
      mergeInto: fixture.line1,
      mergeRepo: fixture.dir,
    });
    assert.notEqual(result.status, 0, result.stdout);
    assert.notEqual(result.status, null);
    assert.match(result.stdout, /protected-path:.*shapes catalog/i);
    assert.doesNotMatch(result.stdout, /\.tgz\b/);
    assertNoLocalSideEffects();
  } finally {
    fixture.cleanup();
  }
}

console.log("release-dry-run protected merge refuse ok");

{
  const fixture = createMergeFixture({
    line0(files) {
      files["sdk/unprotected.js"] = "fix from line0\n";
    },
  });
  try {
    const result = runReleaseDryRun({
      releaseLine: "1",
      commitMessage: "fix: example",
      mergeFrom: fixture.line0,
      mergeInto: fixture.line1,
      mergeRepo: fixture.dir,
    });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /^protected-path: allow$/m);
    assert.match(result.stdout, /\.tgz\b/);
    assertNoLocalSideEffects();
  } finally {
    fixture.cleanup();
  }
}

console.log("release-dry-run protected merge clean ok");

{
  const fixture = createMergeFixture({
    line0(files) {
      files["sdk/unprotected.js"] = "line0 unprotected\n";
    },
    line1(files) {
      files["sdk/unprotected.js"] = "line1 unprotected\n";
    },
  });
  try {
    const beforeUnprotected = readFileSync(
      join(fixture.dir, "sdk/unprotected.js"),
      "utf8",
    );
    const result = runReleaseDryRun({
      releaseLine: "1",
      commitMessage: "fix: example",
      mergeFrom: fixture.line0,
      mergeInto: fixture.line1,
      mergeRepo: fixture.dir,
    });
    assert.notEqual(result.status, 0, result.stdout);
    assert.notEqual(result.status, null);
    assert.match(result.stdout, /^protected-path: allow$/m);
    assert.match(result.stdout, /^merge: conflict$/m);
    assert.doesNotMatch(result.stdout, /protected-path: refuse/);
    assert.doesNotMatch(result.stdout, /\.tgz\b/);
    assert.equal(
      readFileSync(join(fixture.dir, "sdk/unprotected.js"), "utf8"),
      beforeUnprotected,
    );
    assertNoLocalSideEffects();
  } finally {
    fixture.cleanup();
  }
}

console.log("release-dry-run protected merge conflict ok");

const protectedPathCases = [
  {
    label: "package version",
    path: "client-sdk/package.json",
    from: '{\n  "name": "@arcanetech/stellar-privacy-pool-zk-sdk",\n  "version": "0.11.2"\n}\n',
  },
  {
    label: "shapes catalog",
    path: "shapes.json",
    from: '{"shapes":[{"id":"0x-only"}]}\n',
  },
  {
    label: "circuit sources",
    path: "circuits/main.circom",
    from: "0x circuit\n",
  },
  {
    label: "contracts submodule commit",
    path: "soroban-privacy-pools",
    from: "submodule-commit-0x\n",
  },
  {
    label: "release manifest",
    path: "artifacts/circuits-manifest.json",
    from: '{"circuits":{"main":{"version":"0.x"}}}\n',
  },
];

for (const { label, path, from } of protectedPathCases) {
  const fixture = createMergeFixture({
    line0(files) {
      files[path] = from;
    },
  });
  try {
    const result = runReleaseDryRun({
      releaseLine: "1",
      commitMessage: "fix: example",
      mergeFrom: fixture.line0,
      mergeInto: fixture.line1,
      mergeRepo: fixture.dir,
    });
    assert.notEqual(result.status, 0, `${label}: ${result.stdout}`);
    assert.match(
      result.stdout,
      new RegExp(`protected-path: refuse ${label.replaceAll(" ", "\\s+")}`, "i"),
    );
    assert.doesNotMatch(result.stdout, /\.tgz\b/);
    assertNoLocalSideEffects();
  } finally {
    fixture.cleanup();
  }
}

{
  const fixture = createMergeFixture({
    line0(files) {
      files["client-sdk/package.json"] =
        '{\n  "name": "@arcanetech/stellar-privacy-pool-zk-sdk",\n  "version": "1.0.0-rc.0",\n  "dependencies": {\n    "@arcanetech/stellar-privacy-pool-zk-sdk": ">=0.11.0 <1.0.0"\n  }\n}\n';
    },
    line1(files) {
      files["client-sdk/package.json"] =
        '{\n  "name": "@arcanetech/stellar-privacy-pool-zk-sdk",\n  "version": "1.0.0-rc.0",\n  "dependencies": {\n    "@arcanetech/stellar-privacy-pool-zk-sdk": ">=1.0.0 <2.0.0"\n  }\n}\n';
    },
  });
  try {
    const result = runReleaseDryRun({
      releaseLine: "1",
      commitMessage: "fix: example",
      mergeFrom: fixture.line0,
      mergeInto: fixture.line1,
      mergeRepo: fixture.dir,
    });
    assert.notEqual(result.status, 0, result.stdout);
    assert.match(result.stdout, /protected-path: refuse zk SDK dependency range/i);
    assert.doesNotMatch(result.stdout, /\.tgz\b/);
    assertNoLocalSideEffects();
  } finally {
    fixture.cleanup();
  }
}

{
  const fixture = createMergeFixture({
    line0(files) {
      files["client-sdk/package.json"] =
        '{\n  "name": "@arcanetech/stellar-privacy-pool-zk-sdk",\n  "version": "1.0.0-rc.0",\n  "description": "from line0 only"\n}\n';
    },
  });
  try {
    const result = runReleaseDryRun({
      releaseLine: "1",
      commitMessage: "fix: example",
      mergeFrom: fixture.line0,
      mergeInto: fixture.line1,
      mergeRepo: fixture.dir,
    });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /^protected-path: allow$/m);
    assert.match(result.stdout, /\.tgz\b/);
    assertNoLocalSideEffects();
  } finally {
    fixture.cleanup();
  }
}

console.log("release-dry-run protected paths coverage ok");

{
  const packageBeforePublish = readFileSync(packageJsonPath, "utf8");
  try {
    const result = spawnSync(script, [], {
      cwd: root,
      encoding: "utf8",
      env: {
        ...process.env,
        DRY_RUN: "0",
        RELEASE_LINE: "0",
        COMMIT_MESSAGE: "fix: example",
      },
    });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /\bversion:\s*0\.11\.3\b/);
    assert.match(result.stdout, /dist-tags: sdk-v0 latest/);
    assert.match(result.stdout, /stellar\/v0\/circuits-manifest\.json/);
    assert.match(result.stdout, /^mode: publish$/m);
    assert.match(
      result.stdout,
      /skipped:.*npm publish/i,
    );
    assert.doesNotMatch(result.stdout, /packed current package tree/);
    const applied = JSON.parse(readFileSync(packageJsonPath, "utf8"));
    assert.equal(applied.version, "0.11.3");
    assert.equal(
      spawnSync("git", ["tag", "--list"], { cwd: root, encoding: "utf8" })
        .stdout,
      tagsBefore,
    );
    assert.equal(tarballCount(), tarballsBefore);
  } finally {
    writeFileSync(packageJsonPath, packageBeforePublish);
  }
}

console.log("release publish-mode plan apply ok");
