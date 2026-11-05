# 06 — Privacy SDK 1.x release plan

**What to build:** A publisher can dry-run the privacy SDK on the 1.x line. The Stellar package becomes `1.0.0` and depends on the zk SDK at `>=1.0.0 <2.0.0`. Core, state, and relay keep their own versions and are not selected for publish from this line. Release-candidate and promotion dist-tags match the zk SDK line: `next` until a manual promotion, then `latest`.

**Blocked by:** 05 — Privacy SDK 0.x release plan and zk SDK range

**Status:** done

- [x] On line 1 the Stellar package version is `1.0.0` and its zk SDK range is `>=1.0.0 <2.0.0`
- [x] Dry-run on line 1 before promotion prints a `1.0.0` release candidate and dist-tag `next`
- [x] A manual promotion dry-run prints stable `1.0.0` and dist-tag `latest`
- [x] Core, state, and relay are not selected for publish from line 1
- [x] After promotion, a line 0 publish prints dist-tag `v0` and does not take `latest`
- [x] Dry-run packs the Stellar package without publishing it
