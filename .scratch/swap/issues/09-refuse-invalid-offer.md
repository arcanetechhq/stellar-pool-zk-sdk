# 09: An invalid offer never starts a proof

**What to build:** The seller's authorization call and the sender's proof call refuse an offer that the swap circuit would reject, before a proof is built. A proof call on a transfer circuit is refused as well.

**Blocked by:** 03 — The swap authorization binds the parties, the notes, and the quote; 04 — Only ordinary notes of one application id can be swapped; 07 — The sender can prove an exact swap

**Status:** resolved

- [x] A substituted seller output, a subset of the seller's notes, a different sender key, the same asset on both sides, or a zero `bPaid` is refused before proving
- [x] An escrow note, a non-zero accumulator, or a mismatched application id is refused before proving
- [x] `proveSwap` on an SDK initialized for a transfer circuit is refused
- [x] When a current ledger is supplied and it has reached the expiration ledger, proving does not start

## Answer

Invalid offers, including a sixth sender note and a supplied current ledger that has reached expiration, are refused before a proof starts. Omitting the current ledger leaves expiry to the pool. Recorded on `feat/swap` at `83481de` (pool submodule `90bc8f1`).
