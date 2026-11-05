# 03 — Mode 010 withdrawal pays the pinned Stellar account and rejects a different one

**What to build:** Spending a note in mode 010 pays the pinned asset and amount to the pinned Stellar account. Naming a different account does not move the tokens. A different amount or asset is rejected the same way.

**Blocked by:** 01 — Unconstrained notes spend on the new accumulator field

**Status:** done

- [ ] Withdrawal to the pinned account, asset, and amount is accepted and the tokens arrive there
- [ ] Withdrawal to a different Stellar account passes the witness check and is rejected by the pool, and the tokens do not move
- [ ] A different amount or asset is rejected by the pool the same way
- [ ] A public slot that is not the opened note's accumulator fails the witness check
- [ ] A transaction with no mode 010 note publishes a zero accumulator slot
- [ ] A second mode 010 note in the same transaction is rejected
- [ ] The ten-input one-output shape cannot spend a mode 010 note
