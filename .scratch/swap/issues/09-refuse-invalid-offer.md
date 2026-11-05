# 09: An invalid offer never starts a proof

**What to build:** The seller's authorization call and the sender's proof call refuse an offer that the swap circuit would reject, before a proof is built. A proof call on a transfer circuit is refused as well.

**Blocked by:** 03 — The swap authorization binds the parties, the notes, and the price; 04 — Only ordinary notes of one application id can be swapped; 07 — The sender can prove an exact swap

**Status:** ready-for-agent

- [ ] A substituted seller output, a subset of the seller's notes, a different sender key, the same asset on both sides, or a zero `bPaid` is refused before proving
- [ ] An escrow note, a non-zero accumulator, or a mismatched application id is refused before proving
- [ ] `proveSwap` on an SDK initialized for a transfer circuit is refused
- [ ] When a current ledger is supplied and it has reached the expiration ledger, proving does not start
