# 03 — zk SDK protected merge verdict

**What to build:** A publisher can dry-run a merge from the 0.x branch into the 1.x branch and get a pass or fail before merging. The 1.x line keeps its package version, zk SDK dependency range, shapes catalog, circuit sources, contracts submodule commit, and release manifest. Ordinary SDK code that merges cleanly is allowed through. A conflict in that ordinary code is left for a person, not auto-picked.

**Blocked by:** 01 — zk SDK 0.x release plan

**Status:** done

- [x] Dry-run exits non-zero when the merge would replace any protected path with the 0.x copy: package version, zk SDK dependency range, shapes catalog, circuit sources, contracts submodule commit, or release manifest
- [x] Dry-run exits zero when the merge only brings clean changes to unprotected SDK code
- [x] A conflict in unprotected SDK code exits non-zero and does not choose either side
- [x] The verdict names which protected path failed, when one did
