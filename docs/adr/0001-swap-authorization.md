---
status: accepted
---

# Swap authorization binds the seller's received notes

A swap spends ordinary notes. The seller signs an EdDSA-Poseidon authorization over their input commitments, the sender's owner key, both assets, the quote, the swap amount, the expiration ledger, and the commitments of the notes the seller receives. The sender proves. The contract rejects the swap once the ledger reaches the expiration ledger. The pool address is not in the signature: a proof verifies only against a pool whose tree contains those commitments.

Seller-favorable rounding is `bPaid = floor(swapAmount × quoteNum / quoteDen)` on the seller's asset. The sender's asset moves by the exact swap amount. Input and output sums match per asset.

## Considered options

- A new note mode, like escrow. Ordinary notes are enough: delegated EdDSA already spends a note without putting the spend scalar in the witness. A new mode would make the notes unspendable by the existing transfer circuit.
- Leaving the seller's output notes unsigned. Value accounting would still pay the seller, but the sender would choose the openings. The seller now creates those one or two notes and signs their commitments, so the tokens remain spendable without a later message from the sender.
- Exact rational division. Floor toward the seller keeps every moved quantity an integer and conserves both assets.
- Binding the pool address in the signature. The verifying pool already requires the proof's root to be its own.
