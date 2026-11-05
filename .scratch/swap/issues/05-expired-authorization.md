# 05: The pool rejects an expired swap authorization

**What to build:** The pool refuses a swap whose expiration ledger has been reached, and it refuses an expiration ledger that does not fit in a 32-bit ledger number. The seller's notes stay unspent.

**Blocked by:** 01 — Exact swap witness is accepted

**Status:** ready-for-agent

- [ ] A swap whose public expiration ledger is less than or equal to the current ledger is rejected, and no note is spent
- [ ] A swap submitted on the named expiration ledger is rejected
- [ ] An expiration ledger that does not fit in 32 bits is rejected
- [ ] A swap whose expiration ledger is still ahead is accepted
