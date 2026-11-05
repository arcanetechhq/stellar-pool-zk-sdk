# 08: The sender can prove a swap with change

**What to build:** The sender can prove a swap where both sides have change and the quote does not divide evenly. The pool accepts it. The seller receives the proceeds note and the change note that were opened when the swap authorization was created.

**Blocked by:** 02 — Change and seller-favorable floor; 07 — The sender can prove an exact swap

**Status:** resolved

- [x] `proveSwap` submits a swap with leftover value on both sides and a non-dividing quote, and the pool accepts it
- [x] The seller's two output notes are the openings returned with the swap authorization
- [x] The sender receives `bPaid` of the seller's asset and one change note of their own asset
- [x] Input and output sums match for each asset

## Answer

`proveSwap` accepts a non-dividing quote with change on both sides, including a swap that spends two seller notes. The seller's outputs are the authorization openings. Recorded on `feat/swap` at `83481de` (pool submodule `90bc8f1`).
