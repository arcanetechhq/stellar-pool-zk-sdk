# 08: The sender can prove a swap with change

**What to build:** The sender can prove a swap where both sides have change and the quote does not divide evenly. The pool accepts it. The seller receives the proceeds note and the change note that were opened when the swap authorization was created.

**Blocked by:** 02 — Change and seller-favorable floor; 07 — The sender can prove an exact swap

**Status:** claimed

- [x] `proveSwap` submits a swap with leftover value on both sides and a non-dividing quote, and the pool accepts it
- [x] The seller's two output notes are the openings returned with the swap authorization
- [x] The sender receives `bPaid` of the seller's asset and one change note of their own asset
- [x] Input and output sums match for each asset
