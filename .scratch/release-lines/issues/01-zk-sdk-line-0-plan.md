# 01 — zk SDK 0.x release plan

**What to build:** A publisher can dry-run the zk SDK release on the 0.x line and read the plan: the next stable version, the dist-tags it would move, and the 0.x circuits manifest path. A fix ships as a patch, a feature as a minor, and a breaking commit is refused. The package is packed and nothing is tagged or published. The existing push-to-publish path keeps choosing versions until the CI ticket switches it over.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] Dry-run on line 0 with a `fix` commit prints the next stable patch and dist-tags `v0` and `latest`
- [x] Dry-run on line 0 with a `feat` commit prints the next stable minor and the same dist-tags
- [x] Dry-run on line 0 with `feat!:` or a `BREAKING CHANGE` footer exits non-zero and prints no publishable version
- [x] The plan names the 0.x circuits manifest path
- [x] Dry-run packs the package and does not publish it, create a git tag, upload artifacts, or run keygen
