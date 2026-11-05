# 01: Exact swap witness is accepted

**What to build:** A sender and a seller can complete an exact swap. Each side spends one note, neither side has change, and the quote divides evenly. The witness checks. The seller receives one note of the swap amount of the sender's asset, and that note is the single output commitment in the swap authorization.

**Blocked by:** None (can start immediately)

**Status:** claimed

- [x] One non-zero note from the sender and one from the seller produce a witness that checks
- [x] The seller's only received note is worth the swap amount of the sender's asset, and its commitment is the one signed in the swap authorization
- [x] The sender receives `bPaid` of the seller's asset, and `bPaid` equals the exact rational quote
- [x] The remaining slots are padding notes, and the public signals carry the tree root and a future expiration ledger
- [x] The seller's spend scalar is not a witness input; the swap authorization signature verifies instead
