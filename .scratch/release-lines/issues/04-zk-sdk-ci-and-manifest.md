# 04 — zk SDK CI publish and per-line manifest

**What to build:** Pull requests rehearse the zk SDK release with the same script publishers run locally. A push to the 0.x or 1.x branch publishes for real through that script. Circuit rebuilds consult the manifest for that line, and artifact objects stay in the folder of the package version being published. The old bumper that turned every push into a prerelease no longer chooses the version.

**Blocked by:** 01 — zk SDK 0.x release plan; 02 — zk SDK 1.x plan and stable promotion; 03 — zk SDK protected merge verdict

**Status:** done

- [x] A pull request runs the release script as a dry-run and does not tag, publish, upload, or run keygen
- [x] A push to the 0.x branch publishes a stable 0.x version and moves `v0`
- [x] A push to the 1.x branch publishes the version the script selects for that line, including a release candidate on `next` before promotion
- [x] A manual promotion on the 1.x branch publishes stable `1.0.0` and moves `latest`
- [x] Rebuild selection reads the 0.x manifest for the 0.x line and the 1.x manifest for the 1.x line
- [x] Rebuilt artifacts are stored under the package version being published
- [x] Version selection no longer always cuts a prerelease
