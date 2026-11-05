# Stellar Privacy Pool

Shielded pool on Soroban: users hold value as private notes in a Merkle tree, spend them with Groth16 proofs, and disclose note contents only to the KYT/audit side through encrypted audit slots.

## Language

### Notes

**Note**:
A unit of shielded value: amount, asset, owner, and secrets, committed as one Merkle leaf.
_Avoid_: UTXO, coin, commitment (the commitment is the leaf hash, not the note)

**Note opening**:
The secret contents of a note from which its commitment is derived.
_Avoid_: preimage, plaintext

**Padding note**:
A zero-value placeholder occupying an unused input or output position so every transaction has the same shape.
_Avoid_: dummy note, empty note, filler

**Escrow note**:
A note addressed to a Stellar G-address that has no registered private key; its owner key is derived from the recipient address and an escrow nonce, and it can only be swept by that G-address.
_Avoid_: onboarding note, unregistered-recipient note

**Dead-on-arrival note**:
A note that is recorded in the tree but can never be spent by its intended recipient (for example, its nullifier hash is already consumed or its owner key is unreachable).
_Avoid_: burned note, stuck note

### Swap

**Swap**:
A transaction in which the sender and the seller exchange two different assets at a quote. Every note that carries value shares one application id.
_Avoid_: trade, exchange

**Sender**:
The party who proves a swap and spends their notes with their spend scalar.
_Avoid_: buyer, taker

**Seller**:
The party who supplies the other asset in a swap and authorizes that spend with a swap authorization.
_Avoid_: maker, counterparty

**Quote**:
The price of the sender's asset in the seller's asset for one swap.
_Avoid_: rate, price

**Swap amount**:
The quantity of the sender's asset that a swap transfers to the seller.
_Avoid_: amount, fill

**Swap authorization**:
The seller's authorization that fixes the seller's input notes, the notes the seller receives, the sender's owner key, both assets, the quote, the swap amount, and the expiration ledger of one swap.
_Avoid_: delegated spend authorization, escrow sweep

**Expiration ledger**:
The ledger sequence the seller names when signing a swap authorization. The pool accepts the swap only while the current ledger is still below it.
_Avoid_: block, expiry block, ttl

### Disclosure

**Audit slot**:
One encrypted record per input/output position of a transaction, readable only by the audit key holder, describing that position's note.
_Avoid_: audit ciphertext, audit block, note audit

**Shared audit key**:
The single ECDH-derived encryption key and nonce that one transaction uses for all of its audit slots.
_Avoid_: audit ephemeral key (that is the public half only), session key

**Keystream schedule**:
The rule that maps (audit slot, field position) to the one-time pad applied to that field. Must be unique per slot within a transaction.
_Avoid_: stream seed, cipher seed

### Circuit versions

**Zk config nonce**:
The on-chain identifier of a registered circuit (verification key plus public-signal layout) that a transaction declares it was proven under.
_Avoid_: circuit version, VK id

**Binding layout**:
A circuit family where audit ciphertexts live off-chain and the circuit exposes only their authentication tags.
_Avoid_: userspace-MiMC, off-chain ciphertext mode
