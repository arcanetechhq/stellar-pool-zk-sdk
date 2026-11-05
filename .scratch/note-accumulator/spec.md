Status: done

## Problem Statement

A depositor who hands an automation service every note plaintext and the temporary private keys of the intermediate notes cannot stop that service from spending those notes anywhere. The pool only checks that a note opens and that value balances. It does not check that the next notes, or the Stellar account that finally receives the funds, are the ones the depositor chose. A route that only constrains later private notes also cannot force the funds to leave the pool.

## Solution

Each note carries one accumulator field. The field is either zero or a mode wrapped around a payload. Zero means the note does not constrain how it is spent. Mode `001` means a private transfer: the circuit checks that the set of output note commitments matches the payload. Mode `010` means an exit: the pool contract checks that the transaction's public withdrawal matches the asset, the Stellar account, and the amount in the payload.

The depositor builds the route from the end. A deposit note in mode `001` pins the transfer note and the change note. The transfer note is in mode `010` and pins the withdrawal. The service can submit the deposit, the transfer, and the withdrawal. It cannot replace the transfer outputs, and it cannot send the withdrawal to a different Stellar account.

## User Stories

1. As a depositor, I want the note created by my deposit to carry a non-zero accumulator, so that the service which later spends it cannot choose the outputs itself.
2. As a depositor, I want that deposit note to pin both the onward note and the change note, so that the transfer step of the payment has a fixed payment and a fixed change.
3. As a depositor, I want the onward note to commit to a Stellar account, an asset, and an amount, so that the withdrawal step has to pay that account.
4. As a depositor, I want to build those notes from the end, before the deposit, so that every intermediate commitment already contains the next accumulator.
5. As a depositor, I want to give the service the note plaintexts and the temporary private keys of the intermediate notes, so that the service can submit the route without me signing each hop.
6. As an automation service, I want to complete deposit, then transfer with change, then withdraw, when every accumulator is satisfied, so that a precomputed payment goes through.
7. As an automation service, I want the transfer to be rejected when an output note is not one of the pinned notes, so that I cannot redirect the deposit.
8. As an automation service, I want the withdrawal to be rejected when the Stellar account is not the pinned account, so that I cannot keep the exit.
9. As an automation service, I want a masked output to be ignorable in a mode `001` product, so that a fee note does not have to be precomputed when the route leaves a remainder.
10. As an automation service, I want any value not covered by the pinned notes to be mine to assign, so that the fee is the leftover rather than a separate cap.
11. As an automation service, I want to place pinned notes in any output positions, so that slot order is not part of the route.
12. As an automation service, I want to spend a mode `001` note together with notes whose accumulator is zero, so that padding notes and unrelated notes can fill the shape.
13. As an automation service, I want each input note checked on its own, so that two mode `001` notes in one transaction must both match the same outputs.
14. As an automation service, I want at most one mode `010` note in a transaction, so that the single public withdrawal has one meaning.
15. As an automation service, I want mode `000` and mode `001` accumulators kept out of the public signals, so that a private route is not published beside the withdrawal.
16. As a change-note owner, I want the change note from the payment cycle to be spendable with no further route, so that the returned funds are ordinary notes.
17. As a Stellar account holder, I want the pinned withdrawal amount of the pinned asset to arrive on my account, so that the exit is the payment I was promised.
18. As a Stellar account holder, I want a smaller committed amount to be payable while the rest stays a private note, so that a route can exit only part of a note.
19. As a wallet, I want the accumulator inside the encrypted note plaintext, so that I can open the note's commitment after decryption.
20. As a wallet, I want a note whose accumulator is zero to open under every shape, so that an unconstrained note is not tied to one circuit.
21. As a wallet, I want a mode `010` note to be unspendable in the ten-input one-output shape, so that a shape with no public withdrawal cannot skip the exit check.
22. As a spender, I want a note whose accumulator is zero to impose no output check, so that ordinary payments stay unconstrained.
23. As a spender, I want an included commitment of zero or one to be rejected in a mode `001` check, so that a factor cannot disappear from the product.
24. As a spender, I want a second mode `010` note in the same transaction to be rejected, so that two exits cannot share one public withdrawal.
25. As a spender, I want a mode `010` withdrawal whose amount or asset differs from the payload to be rejected, so that the address is not the only bound field.
26. As a spender, I want the public accumulator slot to be zero when the transaction spends no mode `010` note, so that a voluntary withdrawal is not presented as a forced exit.
27. As an auditor, I want audit slots to stay the same length, so that a route does not change the audit record.
28. As a pool operator, I want the new circuit registered under the existing zk config nonces on a new pool, so that the route does not invent a parallel nonce.
29. As a pool operator, I want old notes and the old pool left behind, so that the new plaintext length does not have to be read by old clients.

## Implementation Decisions

- The accumulator is one field of the note. It occupies the commitment limb that is today a constant zero, and it is the seventh limb of the encrypted output-note plaintext. Those two copies are equal. Audit slots stay twelve limbs. Output-note plaintext length becomes 7 on every shape.
- The shapes are the existing two-by-two, six-by-six, and ten-by-one shapes, both the direct spend and the delegated spend. Zk config nonces stay `3`, `7`, and `10`. They are registered on a new pool. The previous pool and its notes are not migrated.
- Mode `000` stores a zero field. The circuit does not check outputs, and the field is not copied to a public signal.
- Mode `001` and mode `010` store `Poseidon255(2)(mode, payload)` with mode `1` or `2` as the first input. No other mode is accepted. The mode is not a bit shift of the payload: a full field element has no room for a three-bit prefix.
- Mode `001` payload is `Poseidon255(2)(route domain, product)`. The product runs over the output positions of that shape. A mask bit of `0` contributes the factor `1`. A mask bit of `1` contributes that position's commitment, which must not be `0` or `1`. The mask is a private bit per input note and per output position. It is not stored in the note. The product binds the set of commitments, not their positions. Each input is checked on its own. A zero accumulator skips the check. Value that is not inside the pinned commitments may leave through a masked note or, when the transaction has no mode `010` note, through the public withdrawal.
- The route domain and the exit domain are new four-byte constants, distinct from the commitment and nullifier domains. The circuit and the pool use the same constants and the same two-input and three-input Poseidon the pool already uses for host hashing.
- Mode `010` payload is `Poseidon255(3)(asset packed, recipient packed, amount)`, where the asset is packed as `Poseidon255(3)(exit domain, asset high, asset low)` and the recipient is packed as `Poseidon255(2)(recipient high, recipient low)`. The amount is the withdrawal amount in the same unit as the note value.
- The public signal list gains one accumulator slot per public withdrawal leg. Two-by-two and six-by-six have one. Ten-by-one has none. The only value that may occupy the slot is the accumulator of the single mode `010` input. If that input is absent, the slot is zero.
- The circuit proves that a non-zero slot equals the opened mode `010` note's accumulator. It does not interpret the asset, the account, or the amount. The pool does. When the slot is non-zero, the pool recomputes `Poseidon255(2)(2, payload)` from the withdrawal's asset, Stellar account, and amount, and rejects the transaction on mismatch. When the slot is zero, the pool does not apply this check.
- A transaction contains at most one mode `010` note. That note cannot be spent on the ten-by-one shape. Its amount may be smaller than the note value; the difference stays in a private output note. While the note is spent, the shape's single public withdrawal is that payment.
- A mode `001` commitment includes the child note's accumulator. Pinning the child therefore pins a later mode `010` exit.
- Spending a note still requires the owner's scalar. Possession of the plaintext secret is not enough. The acceptance cycle does not give the service the withdrawal recipient's key: the exit is a public withdrawal, not a spend of a note owned by that account.
- The acceptance cycle uses the two-by-two shape, which has two private outputs and one public withdrawal. The deposit's public value becomes one note whose accumulator is mode `001` and whose product is the onward note and the change note. The second deposit output is a padding note. The transfer spends the deposit note, with a padding note on the other input, and outputs those two pinned notes. The onward note's value equals its mode `010` amount. The change note's accumulator is zero. The withdrawal spends the onward note and pays the pinned Stellar account. No second fee withdrawal exists in that transaction.

## Testing Decisions

- A good test treats the pool transaction as a black box. It builds a real witness, checks that witness, submits the public signals to the pool with proof verification bypassed, and asserts whether the pool accepts the transaction and, on acceptance, which account received the tokens. It does not assert the inside of a template, a mask wire, or a hash helper.
- The only acceptance seam is the existing Cucumber harness that generates a real witness and checks it with `snarkjs wtns check`, then submits those public signals to `transact`. The positive cycle and both rejections run on that seam. There is no separate circuit suite and no separate hash-vector suite in the acceptance.
- The positive case is one payment cycle with a non-empty accumulator: deposit, transfer with change, withdraw. The witness for each hop checks, the pool accepts each hop, the change note is the pinned change, and the withdrawal pays the pinned Stellar account the pinned amount of the pinned asset.
- The mode `001` rejection is a transfer that spends the same deposit note but replaces a pinned output with a different note. The witness check fails, and the pool does not accept the transfer.
- The mode `010` rejection is a withdrawal of the same onward note that publishes that note's accumulator and names a different Stellar account. The witness check still passes, because the circuit does not interpret the account, and the pool rejects the transaction. The tokens do not move.
- The same pool rejection covers a withdrawal whose amount or asset differs from the payload. Those cases sit on the same seam and are not a second harness.
- A witness that puts any accumulator other than the opened mode `010` note into the public slot fails the witness check. That guards the circuit side of the same exit.
- Prior art is the Cucumber witness pipeline used by the audit and KYT features, and the pool tests that submit a transaction with proof verification bypassed. The revealed-note withdrawal entrypoint is not this seam.

## Out of Scope

- A new zk config nonce, or keeping the old pool able to spend notes written with a seven-limb plaintext.
- Encoding the mode as literal high or low bits of the field.
- Putting the accumulator into the audit plaintext.
- A fee ceiling inside the circuit. The unpinned remainder is the fee.
- More than one mode `010` note in a transaction, or a public withdrawal on the ten-by-one shape.
- Publishing mode `000` or mode `001` accumulators.
- The revealed-note withdrawal entrypoint. Mode `010` is checked on the pool's ordinary public withdrawal.
- Payment-client screens, route-building UI, and handing the service a package format beyond the note plaintexts and temporary keys.
- Regenerating the powers-of-tau file. Proving keys for the changed circuits are produced from the existing parameters when the acceptance witness is built.
- Migrating notes that already sit in a deployed pool.

## Further Notes

- Two-by-two is the acceptance shape because the cycle needs two private outputs and one public withdrawal. Six-by-six follows the same rules and is not a second acceptance cycle. Ten-by-one gets the same commitment and the mode `001` check, and it has no mode `010` slot.
- The commitment of a note whose accumulator is zero stays the commitment produced by a trailing zero limb. That is why an unconstrained note still opens on every shape. It is not a promise that an old pool can verify the new witness.
- Output-note ciphertexts grow by one limb, and the public signal list grows by one accumulator per public withdrawal. Clients and parsers that assume the old public-signal counts do not read this pool.
- The service can still take, as a private note, any value of a mode `010` note above the committed amount. In the acceptance cycle that remainder is zero because the onward note's value equals the withdrawal amount.
