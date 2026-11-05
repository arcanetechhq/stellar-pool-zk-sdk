# 07: The sender can prove an exact swap

**What to build:** The sender can turn the seller's swap authorization into a proof of an exact swap. The pool accepts it. The seller can then spend the proceeds note with an ordinary transfer.

**Blocked by:** 01 — Exact swap witness is accepted; 06 — The seller can create a swap authorization

**Status:** ready-for-agent

- [ ] `proveSwap` runs only on an SDK initialized for the swap circuit
- [ ] It takes the authorization result, both parties' input notes, the sender's spend scalar, the pool state, and audit parameters, and returns a proof the pool accepts
- [ ] The seller's output note in that proof is the note opening from the authorization, not a new note
- [ ] After inclusion, the seller spends that proceeds note with an ordinary transfer, and the pool accepts the transfer
