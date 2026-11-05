# 04 — Payment cycle pins the transfer and the withdrawal from the deposit

**What to build:** A depositor can precompute a payment the service submits in three steps. The deposit pins the onward note and the change note. The onward note pins the withdrawal. Deposit, transfer with change, and withdraw all succeed, the change comes back unconstrained, and the Stellar account receives the pinned payment.

**Blocked by:** 02 — Mode 001 transfer accepts the pinned notes and rejects a substitute; 03 — Mode 010 withdrawal pays the pinned Stellar account and rejects a different one

**Status:** done

- [ ] Deposit, transfer with change, and withdraw all check and are accepted
- [ ] The change note is the note the deposit pinned, and its accumulator is zero
- [ ] The withdrawal pays the pinned Stellar account the pinned amount of the pinned asset
