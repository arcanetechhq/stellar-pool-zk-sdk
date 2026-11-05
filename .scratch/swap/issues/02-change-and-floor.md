# 02: Change and seller-favorable floor

**What to build:** A swap with leftover value on both sides and a quote that does not divide evenly still checks. The seller pays the floored amount of their asset. The swap authorization binds two notes the seller receives: the proceeds and the change.

**Blocked by:** 01 — Exact swap witness is accepted

**Status:** claimed

- [x] A non-dividing quote yields `bPaid = floor(swapAmount × quoteNum / quoteDen)`, and `bPaid` is non-zero
- [x] The seller receives one proceeds note of the swap amount of the sender's asset and one change note of their own asset
- [x] The sender receives `bPaid` of the seller's asset and one change note of their own asset
- [x] The two seller output commitments in the witness are exactly the two commitments in the swap authorization
- [x] Input and output sums match for each asset
