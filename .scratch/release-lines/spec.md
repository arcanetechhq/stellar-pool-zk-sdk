Status: done

## Problem Statement

Deployed testnet services install the privacy SDK on the 0.x line and must keep receiving compatible updates. A breaking change of shapes and Soroban contracts is about to open a 1.x line for mainnet. The SDK does not know which network it will run on. Today both packages have a single publish path: the zk SDK turns every push into a prerelease, and the privacy SDK releases from one branch. A second line on the same package name would steal the `latest` tag, overwrite the single circuits manifest, and force testnet onto new verification keys.

## Solution

Two release lines of the same packages live on two git branches. `development` publishes stable 0.x. `v1` publishes 1.x, as prereleases until a person manually promotes the first stable 1.0.0. A major version means a breaking API, including the commit that introduces the new shapes and contracts. Patch and minor versions inside a major stay compatible and flow to existing installs through a wide semver range. Each line keeps its own npm dist-tag channel and its own circuits manifest. Publishers and CI test a release by running the same release script in dry-run mode and reading the plan it prints.

## User Stories

1. As a testnet operator, I want compatible zk SDK and privacy SDK updates to install without editing my dependency, so that deployed services stay on the old shapes and contracts.
2. As a testnet operator, I want a 1.x release to stay outside my dependency range, so that mainnet shapes never replace the verification keys my pool is registered with.
3. As a testnet operator, I want the privacy SDK range `>=0.7.0 <1.0.0`, so that both patches and minors of the 0.x line arrive on install.
4. As a testnet operator, I want prereleases excluded from that range, so that 1.x release candidates never install into testnet.
5. As an SDK publisher, I want a `fix` commit on `development` to publish the next stable patch, so that bugfixes ship as 0.x without a prerelease suffix.
6. As an SDK publisher, I want a `feat` commit on `development` to publish the next stable minor, so that additive API ships inside the same major.
7. As an SDK publisher, I want a `feat!:` or `BREAKING CHANGE` commit on `development` to fail the release, so that the 0.x line cannot jump to 1.x by accident.
8. As an SDK publisher, I want the breaking shapes-and-contracts commit to land only on `v1`, so that the new zk config nonce and verification keys belong to the 1.x line.
9. As an SDK publisher, I want `v1` to publish `1.0.0-rc.N` under the `next` dist-tag until I promote it, so that mainnet can be integrated before it becomes the default install.
10. As an SDK publisher, I want a manual promotion on `v1` to publish stable `1.0.0` and move `latest` to 1.x, so that new installs follow the new line only when I say so.
11. As an SDK publisher, I want every later `fix` on `v1` to publish a stable 1.x patch and every `feat` to publish a stable 1.x minor, so that the promoted line stops using release candidates.
12. As an SDK publisher, I want every 0.x publish to move the `v0` dist-tag, so that the maintenance line is still installable by tag after `latest` has moved to 1.x.
13. As an SDK publisher, I want a 0.x publish that happens after 1.0.0 to leave `latest` on 1.x, so that a late testnet patch does not become the default install.
14. As an SDK publisher, I want `latest` to stay on the highest stable 0.x until that manual promotion, so that today's default install remains the testnet line.
15. As a privacy SDK publisher, I want `development` to depend on the zk SDK with `>=0.11.0 <1.0.0`, so that a new 0.x zk SDK is picked up when the privacy SDK is installed, without a new privacy SDK release.
16. As a privacy SDK publisher, I want `v1` to depend on the zk SDK with `>=1.0.0 <2.0.0`, so that the 1.x privacy SDK cannot resolve a 0.x zk SDK.
17. As a privacy SDK publisher, I want only the Stellar package's major to become 1.0.0 when it moves onto zk SDK 1.x, so that core, state, and relay versions stay independent of the shapes change.
18. As a privacy SDK publisher, I want core, state, and relay to publish only from `development`, so that the same non-breaking versions are not published twice from two branches.
19. As a privacy SDK publisher, I want `v1` to consume those already published core, state, and relay versions, so that the 1.x Stellar package still builds against them.
20. As an SDK publisher, I want to bring a compatible fix from `development` into `v1` by merging the branch, so that the fix is not retyped by hand.
21. As an SDK publisher, I want the merge to keep `v1`'s package version, zk SDK range, shapes catalog, circuit sources, submodule commit, and release manifest, so that the 0.x line cannot roll back 1.x schemas or the 1.x dependency window.
22. As an SDK publisher, I want shared SDK code that is not in that protected set to merge normally, so that ordinary bugfixes do land on `v1`.
23. As CI, I want to reject a merge that would replace any protected path with the `development` copy, so that a clean-looking merge cannot silently retarget verification keys.
24. As CI, I want a conflict in unprotected SDK code to remain for a person to resolve, so that overlapping edits are not auto-picked.
25. As an SDK publisher, I want each line to decide circuit rebuilds from its own manifest, so that publishing 0.x does not force a full keygen of 1.x or the reverse.
26. As an SDK publisher, I want rebuilt artifacts stored under that package version's folder, so that an already shipped SDK keeps fetching the verification keys it was built with.
27. As an SDK publisher, I want the manifest baked into the package at build time, so that installers do not follow a live manifest that another line just replaced.
28. As an SDK publisher, I want to run the release script locally in dry-run mode, so that I can see the version, dist-tag, manifest path, and both verdicts before anything is published.
29. As CI on a pull request, I want the same script in dry-run mode, so that the plan is checked without tags, registry writes, object storage, or keygen.
30. As CI on a push to `development` or `v1`, I want the same script without dry-run, so that the publish path is the one I already rehearsed.
31. As an SDK publisher, I want dry-run to pack the package without publishing it, so that the tarball is proven buildable.
32. As an SDK publisher, I want the SDK package to carry no network identity, so that testnet and mainnet choose a line by version range rather than by a name inside the SDK.
33. As a mainnet integrator, I want to opt into 1.x by raising my range to that major, so that the switch to new shapes is a deliberate dependency change.
34. As a mainnet integrator, I want `npm install` of the package name with no range to follow `latest`, so that after promotion the default line is 1.x.
35. As a testnet operator, I want `npm install` of the package at the `v0` tag to follow the 0.x line after promotion, so that maintenance installs do not depend on `latest`.

## Implementation Decisions

- One release script per repository. The zk SDK and the privacy SDK each own a script. CI and a local dry-run call that script. The observable contract is the same in both repositories.
- The script reads the release line (`0` for `development`, `1` for `v1`), the commit message, and whether this run is a dry-run. For a merge check it also reads the two branch tips.
- Version selection follows conventional commits. `fix` increments patch. `feat` increments minor. `feat!:` or a `BREAKING CHANGE` footer increments major.
- Line 0 refuses a major bump and exits non-zero. Line 0 publishes a stable version with no prerelease suffix.
- Line 1, before manual promotion, publishes `1.0.0-rc.N`. After manual promotion it publishes stable 1.x, with `fix` as patch and `feat` as minor.
- Dist-tags. A stable 0.x publish moves `v0`. It also moves `latest` only while no stable 1.x exists. A 1.x prerelease moves `next` and does not move `latest`. The manual promotion publishes `1.0.0`, moves `latest` to that version, and leaves `v0` on the 0.x line.
- Manual promotion is a workflow dispatch on `v1`, not a commit message and not the prerelease checkbox on a GitHub release.
- The privacy SDK mirrors the branch names. On `development` the Stellar package depends on the zk SDK at `>=0.11.0 <1.0.0`. On `v1` it depends on `>=1.0.0 <2.0.0`. The Stellar package becomes `1.0.0` on the breaking transition. Core, state, and relay keep their own versions and are published only from `development`.
- Transfer from `development` to `v1` is a git merge. On conflict, and whenever a protected path differs, `v1` keeps its own copy of: the package version, the zk SDK dependency range, the shapes catalog, the circuit sources, the contracts submodule commit, and the release manifest. Any other path merges as usual.
- CI fails the merge when a protected path would change to the `development` blob. The release script reports that verdict.
- Circuit rebuild selection reads a per-major manifest: one object for the 0.x line and one object for the 1.x line, both under the existing stellar prefix. Artifact objects stay under a folder named by the package version being published. The package embeds the manifest it was built with. Dry-run prints the manifest path and does not upload.
- Dry-run prints the next version, the dist-tag, the manifest path, the breaking-commit verdict, and the protected-path verdict, then packs the package without publishing. It does not create a git tag, a GitHub release, an object-storage upload, or a keygen.
- Pull requests run the script as a dry-run. Pushes to `development` and `v1` run it for real.
- The payment client dependency on the Stellar privacy package is widened once from the current caret on `0.7.0` to `>=0.7.0 <1.0.0`. The client does not depend on the zk SDK directly.
- The current zk SDK bumper, which always cuts a prerelease and tags from the hyphen in the version, is replaced by this script. The privacy SDK publisher, which already can skip the registry write, is extended so the same script also owns version, dist-tag, and verdicts. The privacy SDK's release automation follows `development` and `v1` rather than a single main branch.

## Testing Decisions

- A good test treats the release script as a black box. It sets the line, the commit message, dry-run, and when needed two git tips, runs the script, and asserts the exit code and the plan on stdout. It does not assert private helpers, workflow YAML, or keygen.
- The modules under test are the two release scripts, one in the zk SDK repository and one in the privacy SDK repository. The same cases apply to both, with the privacy SDK cases also asserting the Stellar package's zk SDK range and that core, state, and relay are not selected for publish from line 1.
- Cases that must pass: `fix` on line 0 yields the next stable patch, dist-tag `v0`, and the 0.x manifest path; `feat` on line 0 yields the next stable minor; `feat!:` on line 1 before promotion yields the next `1.0.0-rc.N` and dist-tag `next`; a merge that only touches unprotected code yields a pass verdict; dry-run still packs.
- Cases that must fail: `feat!:` on line 0; a merge that replaces any protected path with the `development` copy.
- After a simulated promotion, a `fix` on line 1 yields a stable 1.x patch and dist-tag `latest`, and a following 0.x publish reports dist-tag `v0` without taking `latest`.
- Dry-run cases assert the absence of side effects that the script can perform locally: no new git tag on the fixture repository, and no pack publish. They do not call the registry, object storage, or keygen.
- Prior art is the privacy SDK publish script's existing dry-run, which prints what it would publish and returns without a registry write. The zk SDK has no release-script tests today. Package-level tests in the privacy SDK stay as they are; they are not the seam for this feature.

## Out of Scope

- Generating or changing shapes, circuits, verification keys, the powers-of-tau file, or Soroban contract logic. This spec only decides which line publishes them.
- Registering a new zk config nonce on a pool.
- Teaching the SDK which network it is on, splitting package names by network, or pinning testnet to one exact version.
- Publishing core, state, or relay from `v1`.
- Running keygen, uploading artifacts, or creating a GitHub release inside dry-run.
- Migrating deployed testnet pools onto the new contracts.
- Cutting the `v1` branch or landing the breaking shapes commit. The script must accept that line once the branch exists.

## Further Notes

- The zk SDK today publishes only from `development`, at `0.11.1-rc.4`, and every automated bump is a prerelease. There is no `v1` branch yet. The live circuits manifest is a single object shared by every version. Artifact objects are already stored per package version.
- The Stellar privacy package in this checkout is `0.6.1` and depends on the zk SDK with a caret that stops before `0.12.0`. The payment client depends on the Stellar package with a caret on `0.7.0`, so the published Stellar line is ahead of this checkout. The agreed client window starts at `0.7.0`.
- A release line is not a zk config nonce. The nonce stays the on-chain identifier of a registered verification key and public-signal layout. The npm major only says which API and which shapes that package was built against.
- No ADR in this repository contradicts the two-line publish. The contracts repository's rule that production proving artifacts and the zk SDK are published from this repository still holds. Circuit source changes remain in the contracts submodule; `v1` keeps that submodule commit across merges from `development`.
