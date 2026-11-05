# 05 — Privacy SDK 0.x release plan and zk SDK range

**What to build:** A publisher can dry-run the privacy SDK on the 0.x line and read the same kind of plan as for the zk SDK: next stable patch or minor, dist-tags, and a refusal of a breaking commit. The Stellar package on this line depends on the zk SDK at `>=0.11.0 <1.0.0`, so a newer 0.x zk SDK is selected at install time. The package is packed and nothing is published.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] Dry-run on line 0 with a `fix` prints the next stable patch and dist-tags `v0` and `latest`
- [x] Dry-run on line 0 with a `feat` prints the next stable minor and the same dist-tags
- [x] Dry-run on line 0 with `feat!:` or a `BREAKING CHANGE` footer exits non-zero
- [x] The Stellar package depends on the zk SDK at `>=0.11.0 <1.0.0`
- [x] Dry-run packs the Stellar package and does not publish, tag, or upload
