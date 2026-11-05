# 07 — Privacy SDK protected merge verdict

**What to build:** A publisher can dry-run a merge from the privacy SDK 0.x branch into its 1.x branch. The 1.x line keeps the Stellar package version, the zk SDK dependency range, and the release manifest. Ordinary SDK code that merges cleanly is allowed through. A conflict in that ordinary code is left for a person.

**Blocked by:** 05 — Privacy SDK 0.x release plan and zk SDK range

**Status:** done

- [x] Dry-run exits non-zero when the merge would replace the Stellar package version, the zk SDK dependency range, or the release manifest with the 0.x copy
- [x] Dry-run exits zero when the merge only brings clean changes to unprotected SDK code
- [x] A conflict in unprotected SDK code exits non-zero and does not choose either side
- [x] The verdict names which protected path failed, when one did
