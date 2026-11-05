# 07: The sender can prove an exact swap

**What to build:** The sender can turn the seller's swap authorization into a proof of an exact swap. The pool accepts it. The seller can then spend the proceeds note with an ordinary transfer.

**Blocked by:** 01 — Exact swap witness is accepted; 06 — The seller can create a swap authorization

**Status:** resolved

- [x] `proveSwap` runs only on an SDK initialized for the swap circuit
- [x] It takes the authorization result, both parties' input notes, the sender's spend scalar, the pool state, and audit parameters, and returns a proof the pool accepts
- [x] The seller's output note in that proof is the note opening from the authorization, not a new note
- [x] After inclusion, the seller spends that proceeds note with an ordinary transfer, and the pool accepts the transfer

## Answer

`proveSwap` on the swap circuit produces a proof the pool accepts, and the seller later spends the authorized proceeds note with an ordinary transfer. Recorded on `feat/swap` at `83481de` (pool submodule `90bc8f1`).
