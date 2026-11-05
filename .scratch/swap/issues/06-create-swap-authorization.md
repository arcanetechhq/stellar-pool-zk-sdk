# 06: The seller can create a swap authorization

**What to build:** The seller can create a swap authorization without loading a proving key. They receive the signature, the minimum sender balance, the floored payment, and the note openings of the notes they will receive. That signature is accepted by the exact-swap witness.

**Blocked by:** 01 — Exact swap witness is accepted

**Status:** claimed

- [x] The authorization is created from the seller's notes, the sender's owner key, the quote, the swap amount, and the expiration ledger, without a proving key
- [x] The result includes the signature, `minimumSenderBalance` equal to the swap amount, `bPaid`, and the note opening of the proceeds note
- [x] A change of zero returns one note opening, and a non-zero change returns the proceeds opening and the change opening
- [x] The signature verifies in an exact swap
- [x] A scalar that does not open the notes, mixed assets, mixed application ids, an empty note set, and a set of six notes are rejected
