# 02 — zk SDK 1.x plan and stable promotion

**What to build:** A publisher can dry-run the zk SDK on the 1.x line. Before promotion the plan is a `1.0.0` release candidate on the `next` dist-tag. A manual promotion prints stable `1.0.0` and moves `latest` there. Later fixes and features on that line are stable 1.x versions on `latest`. A later 0.x publish still moves only `v0`.

**Blocked by:** 01 — zk SDK 0.x release plan

**Status:** done

- [x] Dry-run on line 1 before promotion prints the next `1.0.0-rc.N` and dist-tag `next`, and does not move `latest`
- [x] A manual promotion dry-run prints stable `1.0.0` and dist-tag `latest`
- [x] After that promotion, a `fix` on line 1 prints the next stable 1.x patch on `latest`, and a `feat` prints the next stable 1.x minor on `latest`
- [x] After that promotion, a line 0 publish prints dist-tag `v0` and does not take `latest`
- [x] Dry-run still packs without tagging, publishing, uploading, or keygen
