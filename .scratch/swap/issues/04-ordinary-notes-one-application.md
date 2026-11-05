# 04: Only ordinary notes of one application id can be swapped

**What to build:** A swap accepts only ordinary notes that share one application id. Escrow notes and notes with a non-zero accumulator cannot enter. Padding notes stay the empty dummies of a transfer, with application id zero.

**Blocked by:** 01 — Exact swap witness is accepted

**Status:** claimed

- [x] An escrow note as an input or as an output fails the witness check
- [x] A note with a non-zero accumulator fails the witness check
- [x] A non-zero note whose application id differs from the others fails the witness check
- [x] An accepted swap's padding notes have application id zero, value zero, asset zero, and a throwaway owner key
