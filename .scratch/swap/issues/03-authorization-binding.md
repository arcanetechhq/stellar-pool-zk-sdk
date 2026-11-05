# 03: The swap authorization binds the parties, the notes, and the quote

**What to build:** A witness that does not match the swap authorization fails. The sender cannot replace the seller's payment, drop one of the seller's notes, use another owner key, swap an asset for itself, or pay a quote that rounds to nothing.

**Blocked by:** 01 — Exact swap witness is accepted

**Status:** resolved

- [x] A different opening in place of the seller's proceeds note fails the witness check
- [x] Spending a subset of the seller's authorized notes fails the witness check
- [x] A sender owner key other than the key in the swap authorization fails the witness check
- [x] The same asset on both sides fails the witness check
- [x] A quote and swap amount whose floor is zero fails the witness check

## Answer

A replaced proceeds note, a subset of the seller's notes, another sender key, the same asset on both sides, a zero floor, and a product that wraps in the field all fail the witness check. Recorded on `feat/swap` at `83481de` (pool submodule `90bc8f1`).
