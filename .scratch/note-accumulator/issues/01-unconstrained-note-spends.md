# 01 — Unconstrained notes spend on the new accumulator field

**What to build:** A depositor can deposit, transfer, and withdraw a note whose accumulator is zero. The pool accepts the payment. The field is stored on the note and does not constrain the spend.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] Deposit, transfer, and withdraw of a note with accumulator zero are accepted by the pool
- [ ] The witness for that payment checks
- [ ] The zero accumulator is not copied into the public accumulator slot
- [ ] Audit slots for the payment stay the same length as before
