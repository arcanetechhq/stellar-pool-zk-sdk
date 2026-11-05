Status: ready-for-agent

## Problem Statement

A sender and a seller want to exchange two shielded assets at an agreed quote. The seller must be able to authorize the exact notes, the quote, the swap amount, and the expiration ledger without placing their spend scalar in the proof. The sender, who builds the proof, must not be able to withhold the notes the seller is paid: the seller has to hold those note openings before the proof exists.

## Solution

The seller signs a swap authorization over their input notes, the sender's owner key, both assets, the quote, the swap amount, the expiration ledger, and the commitments of the one or two notes the seller will receive. A new six-by-six swap transaction verifies that signature, moves the two assets at a seller-favorable integer quote, and pays each party in ordinary notes. The pool rejects the transaction once the ledger reaches the expiration ledger. The sender proves with a new SDK method. The seller creates the authorization, and the openings of the notes they receive, with a separate method that does not load a proving key.

## User Stories

1. As a seller, I want to authorize spending my notes with a signature instead of putting my spend scalar in the proof, so that the sender can prove the swap without learning that scalar.
2. As a seller, I want the authorization to name the sender's owner key, so that a different key cannot fill the swap.
3. As a seller, I want my own owner key and the sender's owner key to be different, so that one party cannot be both sides.
4. As a seller, I want the authorization to fix both assets, so that the sender cannot substitute another token.
5. As a seller, I want the two assets to be different and both non-zero, so that a swap of an asset for itself is rejected.
6. As a seller, I want to choose the expiration ledger when I sign, so that the offer dies at a ledger I name.
7. As a seller, I want the pool to reject the swap when the current ledger has reached that expiration ledger, so that a late proof cannot be included.
8. As a seller, I want the last valid ledger to be the one before the expiration ledger, so that a swap submitted on the named ledger is already too late.
9. As a seller, I want the authorization to survive later deposits in the pool, so that I do not have to re-sign every time the tree root changes.
10. As a seller, I want the authorization to bind the commitments of the notes I receive, so that the sender cannot replace them.
11. As a seller, I want those notes created when I authorize, and their note openings returned to me, so that I can spend the payment after the swap is included without another message from the sender.
12. As a seller, I want exactly one proceeds note, worth the swap amount of the sender's asset, so that the payment is a single ordinary note.
13. As a seller, I want one change note of my own asset when the change is non-zero, so that the unspent remainder comes back to me.
14. As a seller, I want no change note when the change is zero, so that the authorization contains only the proceeds commitment.
15. As a seller, I want the result and the change kept as one note each, so that this authorization does not split my payment.
16. As a seller, I want `minimumSenderBalance` on the authorization result, equal to the swap amount, so that the sender can see how much of their asset they must hold.
17. As a seller, I want `bPaid` on the authorization result, so that the rounded amount of my asset is visible without repeating the division.
18. As a seller, I want to create the authorization without loading a proving key, so that signing does not download the swap circuit.
19. As a seller, I want the signer to reject a spend scalar that does not open my notes, so that I cannot sign someone else's notes.
20. As a seller, I want the signer to reject my notes when they do not share one asset and one application id, so that a mixed set cannot be authorized.
21. As a seller, I want to authorize from one to five notes, so that a six-by-six swap still leaves a slot for the sender.
22. As a seller, I want an empty set and a set of six notes rejected, so that the shape cannot be filled with only my notes.
23. As a seller, I want each authorization to sample fresh nullifiers and ephemeral scalars for the notes I receive, so that two offers do not reuse one output note.
24. As a seller, I want to keep the note openings and also hand them to the sender, so that the sender can place those exact notes in the proof.
25. As a sender, I want to prove the swap after receiving the seller's input note openings, the swap authorization, and the openings of the notes the seller receives, so that I can fill an offer I did not sign.
26. As a sender, I want my own notes spent with my spend scalar, so that my side of the swap uses the ordinary spend.
27. As a sender, I want every non-zero input whose commitment is in the authorization to be treated as the seller's, so that I cannot spend the seller's note with my scalar.
28. As a sender, I want every other non-zero input to be mine and to match the authorized sender key and sender asset, so that I cannot fund the swap with another asset or another key.
29. As a sender, I want the proof refused before it starts when my notes sum to less than `minimumSenderBalance`, so that I do not build a proof that cannot pay.
30. As a sender, I want the proof refused when the seller's notes sum to less than `bPaid`, so that an offer the seller cannot fund fails fast.
31. As a sender, I want the signature checked before the proof, so that a tampered authorization fails before proving.
32. As a sender, I want the seller's output notes copied from the returned note openings, so that I cannot invent a different payment note.
33. As a sender, I want the proof refused when those openings do not match the signed commitments, so that a substituted payment is caught.
34. As a sender, I want my proceeds and my change created for me, so that I do not hand-build my output notes.
35. As a sender, I want one note of `bPaid` of the seller's asset, so that I receive the rounded payment as one note.
36. As a sender, I want one change note of my asset when my change is non-zero, and no change note when it is zero, so that my remainder comes back in one note.
37. As a sender, I want the unused output slots filled with padding notes, so that the six-by-six shape is complete.
38. As a sender, I want a `WithdrawResult` for the existing submit path, so that the swap is submitted like any other pool transaction.
39. As a sender, I want the proof skipped when I pass a current ledger that has already reached the expiration ledger, so that I do not prove an offer the pool will reject.
40. As a sender, I want the proof to proceed when I do not pass a current ledger, so that the pool remains the authority on expiry.
41. As a sender, I want audit parameters to be required, so that the swap is not encrypted to a demo audit key by default.
42. As a sender, I want the SDK to build the Merkle witnesses from the pool state, so that I pass notes and a state file rather than paths.
43. As a sender, I want `proveSwap` to refuse an SDK instance that was not initialized for the swap circuit, so that a transfer proving key cannot be used by mistake.
44. As a sender, I want to spend several of my notes in one swap, so that the swap amount need not fit in a single note.
45. As a sender, I want one to five of my notes accepted and a sixth rejected, so that the seller keeps at least one input slot.
46. As a pool, I want the swap submitted through the existing transaction entrypoint, so that nullifiers, the tree, pairwise outputs, and the KYT passage stay on the path they already use.
47. As a pool, I want a new zk config nonce for this circuit, so that the transfer circuits stay registered as they are.
48. As a pool, I want the expiration ledger published and compared with the current ledger, so that expiry does not depend on the circuit knowing the time.
49. As a pool, I want an expiration ledger that does not fit in a 32-bit ledger number rejected, so that the compared value is the value that was signed.
50. As a pool, I want the proof's tree root to be one of my roots, so that notes absent from my tree cannot be spent here.
51. As a pool, I want no pool address inside the authorization, so that a second pool that actually contains the same commitments may accept the same offer.
52. As a pool, I want every output commitment inserted, including padding notes, so that a zero-value padding note is still a real leaf.
53. As a pool, I want adjacent output commitments to be non-zero pairs, so that the pairwise tree insertion accepts the swap.
54. As a sender, I want the swap amount of my asset transferred in full, so that my side of the quote is not rounded.
55. As a seller, I want to pay `floor(swapAmount × quoteNum / quoteDen)` of my asset, so that rounding favors me and every moved quantity is an integer.
56. As a sender, I want a zero `bPaid` rejected, so that I cannot pay a swap amount and receive nothing.
57. As a sender, I want a quote or a swap amount of zero rejected, so that an empty quote cannot authorize a transfer.
58. As a sender, I want the product of the swap amount and the quote numerator constrained below `2^253`, and each of those inputs below `2^127`, so that the floor is integer division and not a wrap in the field.
59. As a sender, I want each asset conserved, so that tokens cannot appear or disappear.
60. As a sender, I want the fractional remainder of the division to stay with the seller as a smaller payment, so that the remainder is not minted and not burned.
61. As a sender, I want no separate fee, so that the floor is the whole seller advantage.
62. As a sender, I want a partial fill rejected, so that I cannot spend a smaller swap amount than the one signed.
63. As a sender, I want a subset of the seller's signed notes rejected, so that I must spend every note the seller authorized.
64. As a sender, I want notes beyond the signed seller set rejected when I label them as the seller's, so that I cannot pull in an extra seller note.
65. As a sender, I want a third asset on a non-zero note rejected, so that only the two authorized assets move.
66. As a sender, I want a padding note's zero asset ignored by the swap, so that an empty slot is not a third asset.
67. As a sender, I want every non-zero note in the swap to share one application id, so that the payment cannot be relabeled into another application.
68. As a sender, I want padding notes to keep application id zero, so that empty slots stay the same dummies as in a transfer.
69. As a sender, I want every non-zero note to be an ordinary note with owner mode zero and a zero accumulator, so that a pinned note cannot leave its pin through a swap.
70. As a sender, I want an escrow note rejected on input and never created on output, so that a derived escrow key cannot be spent without the G-address authorization.
71. As a sender, I want padding notes to use a throwaway key, a zero value, a zero asset, and a zero accumulator, so that an empty slot has no party as owner.
72. As a sender, I want an input padding note to carry a throwaway scalar in the witness, so that the spend check can run on every input slot as it does in a transfer.
73. As a sender, I want an output padding note to publish only a throwaway key, so that nobody keeps the scalar of an empty output.
74. As a seller, I want the notes I receive to use my owner key, my application id, and a zero accumulator, so that I can spend them later with an ordinary transfer.
75. As a seller, I want the secret of a note I receive derived the same way as a deposit, so that the note opens under the existing transfer circuit.
76. As a seller, I want the output note encrypted to me as well as opened from the note opening I stored, so that the public note tag still matches.
77. As a sender, I want to learn the seller's output note openings and still be unable to spend them, so that seeing the opening is not enough without the seller's scalar.
78. As a seller, I want a second swap that reuses my input notes to fail once the first is included, so that the offer cannot pay twice.
79. As a sender, I want two different authorizations over the same seller notes to be spendable only once, so that the first included proof wins.
80. As an auditor, I want every input and output slot audited as a note plaintext, so that the swap discloses the same note records as a transfer.
81. As an auditor, I want the quote and the swap amount absent from the audit record, so that I read the moved amounts from the notes.
82. As a sender, I want the public signals to be the six nullifiers, six commitments, ephemeral keys, audit key, audit tags, output note tags, the tree root, and the expiration ledger, so that withdraw addresses, escrow fields, and public amounts are not part of this proof.
83. As a sender, I want commitment and nullifier formulas left unchanged, so that notes created by a swap spend in the existing transfer circuit.
84. As a pool operator, I want the transfer circuits untouched, so that this swap does not rewrite an audited transfer statement.

## Implementation Decisions

- The swap is a new circuit and a new zk config nonce, chosen at registration from nonces that are not already in use. It is submitted through the existing pool transaction entrypoint. The direct and delegated transfer circuits are unchanged. The circuit is six inputs, six outputs, tree depth 20, and twelve audit slots. It has no public deposit and no public withdrawal.
- Public outputs, in order, are the six nullifier hashes, the six commitment hashes, the six ephemeral keys, the audit ephemeral public key, the twelve audit tags, and the six output note tags. Public inputs, after those outputs, are the tree root and the expiration ledger. The expiration ledger is range-checked to a 32-bit unsigned integer.
- The pool reads that public expiration ledger and rejects the transaction when the current ledger sequence is greater than or equal to it. The circuit does not know the current ledger.
- A non-zero input whose commitment is in the signed seller set is a seller note. Every other non-zero input is a sender note. Zero-value inputs are padding notes. Seller notes are authorized by one EdDSA-Poseidon signature under the seller's owner key. Sender notes are authorized by the sender's spend scalar. Padding inputs still carry a throwaway scalar in the witness so the per-slot spend check can run. There is at least one non-zero note on each side, and at most five on each side.
- The signed message is a new domain-separated chain, with no unbound extra field. In order: the domain separator, the sender's owner key, the sender asset, the seller asset, the quote numerator, the quote denominator, the swap amount, the expiration ledger, the count of seller input commitments, those commitments in strictly ascending order, the count of seller output commitments, and those commitments in strictly ascending order. The two assets are different and both non-zero. The sender and seller keys are different. The tree root and the pool address are not in the message.
- `bPaid` is the floor of `swapAmount × quoteNum / quoteDen`. The swap amount, the numerator, and the denominator are non-zero and below `2^127`. The product is below `2^253`. `bPaid` is non-zero. The sender's asset moves by exactly the swap amount. The seller's asset moves by exactly `bPaid`. Change in each asset returns to the party who contributed it. Sums match per asset. There is no separate fee.
- Every non-zero note has owner mode zero, a zero accumulator, and the single application id taken from the seller's input notes. Escrow notes are rejected. Padding notes are the existing dummies: value zero, asset zero, accumulator zero, application id zero, owner mode zero, and a throwaway key. Output padding notes do not retain the scalar.
- The seller's received notes are created when the authorization is signed. There is one proceeds note of the swap amount of the sender's asset, owned by the seller. When the seller's change is non-zero there is one change note of that change of the seller's asset, also owned by the seller. Their secrets are derived as on a deposit. The signature binds their commitments, not the openings. The openings are returned beside the signature. The circuit accepts the proof only when the non-zero outputs under the seller's key are exactly that signed set.
- The sender's output notes are created during proving and are not signed. They are one note of `bPaid` of the seller's asset to the sender, plus one change note of the sender's asset when that change is non-zero. Remaining output slots are padding notes. Every output commitment is a real non-zero commitment and is inserted. Output note tags cover the same seven-limb deposit plaintext. Audit tags cover the same twelve-limb note plaintext. The quote is not an audit field.
- `createSwapAuthorization` is a WASM function. It does not initialize the SDK and does not load a proving key. It takes the seller's spend scalar as a decimal string, one to five note records of the kind a withdrawal already accepts, the sender's owner key, the quote numerator, the quote denominator, the swap amount, and the expiration ledger. Amounts and the ledger are arbitrary-precision integers. It checks the scalar, the shared asset, and the shared application id, computes commitments, sorts them, computes `bPaid` and the change, creates the seller's output notes, and signs. It returns the signature, the canonical signed fields, `minimumSenderBalance`, `bPaid`, and one or two note openings in role order: proceeds, then change when present.
- `proveSwap` is a method on the SDK instance that was initialized for the swap circuit, and it rejects any other circuit. It takes the authorization result, the seller's input notes, the sender's input notes, the sender's spend scalar, the pool state file, and required audit parameters. An optional current ledger rejects the call when that ledger has reached the expiration ledger. The method builds Merkle witnesses, checks the signature and the openings, checks both sums, copies the seller's output notes from the openings, creates the sender's output notes, pads with dummies, and returns the same result type as an ordinary proof: proof bytes, public-signal bytes, ciphertext, and the output ephemeral scalars.
- A note the seller receives is spent later by the existing transfer proof, using the stored note opening and the seller's spend scalar. The sender, who saw the opening, cannot spend it.
- The accepted swap-authorization decision is the constraint on this shape: ordinary notes, seller-received commitments inside the signature, floor toward the seller, expiry enforced by the pool, and no pool address in the signature.

## Testing Decisions

- A good test treats the swap as a pool transaction. It creates a swap authorization, builds a real proof with `proveSwap`, checks that witness, submits the transaction, and asserts whether the pool accepts it. It does not assert the inside of a template, a hash helper, or the order of private wires.
- The only acceptance seam is the existing Cucumber harness that generates a real witness and checks it, then submits those public signals to the pool transaction entrypoint. Positive cases and rejections run on that seam. There is no separate circuit suite and no separate arithmetic suite in the acceptance.
- The positive case has change on both sides and a quote that does not divide evenly. The pool accepts the swap. The seller receives one note of the swap amount of the sender's asset. The seller's change note is their input sum minus `bPaid`. The sender receives `bPaid` of the seller's asset and their own change. After inclusion, the seller spends the proceeds note with an ordinary transfer, and the pool accepts that transfer.
- A second positive case has exact note sums, so neither side has a change note. The authorization contains one seller output commitment. The pool accepts.
- Replacing the seller's proceeds note with a different opening fails the witness check, and the pool does not accept the swap.
- A proof whose expiration ledger is in the past passes the witness check only when the caller did not supply the current ledger. The pool rejects it, and no note is spent. The same rejection covers an expiration ledger that does not fit in 32 bits.
- A different sender key, a subset of the seller's notes, a repeated seller commitment, the same asset on both sides, a zero `bPaid`, a non-zero accumulator, an escrow note, and a mismatched application id fail the witness check.
- Padding notes in an accepted swap have application id zero and a throwaway owner. They do not have to carry the swap's application id.
- Prior art is the Cucumber witness pipeline used by the audit, KYT, and note-accumulator features, and the pool tests that submit a transaction after the witness check. The revealed-note withdrawal entrypoint is not this seam.

## Out of Scope

- A new note mode, or spending escrow notes inside a swap.
- Changing the direct or delegated transfer circuits.
- Putting the pool address or the tree root in the swap authorization.
- Splitting the seller's proceeds or the seller's change into more than one note.
- A caller-chosen layout of output notes. `proveSwap` builds the sender's notes and copies the seller's notes.
- Public deposits and public withdrawals in the swap.
- Spending a note whose accumulator is non-zero.
- A fee other than the floored remainder.
- More than one sender or more than one seller.
- A shape other than six-by-six.
- Publishing the quote or the swap amount as a public signal or as an audit field.
- A new transaction entrypoint, or a change to how a KYT passage is registered.
- Wallet screens and a network format for handing the authorization to the sender, beyond the object the authorization function returns.
- Regenerating the powers-of-tau file. The swap proving key is produced from the existing parameters when the acceptance witness is built.
- Choosing the numeric zk config nonce in this spec. Registration picks an unused nonce.

## Further Notes

- The seller's change is known when they sign, because it depends only on their own notes and `bPaid`. It does not depend on which of the sender's notes fund the swap.
- The sender must receive the seller's output note openings in order to build the witness. Knowledge of an opening does not spend the note. The spend still requires the seller's scalar.
- An offer that is never included leaves the seller's input notes unspent. The output openings are useless until their commitments are in the tree.
- Duplicate copies of the same commitments in another pool are valid for the same authorization. That is the accepted reading of leaving the pool address unsigned.
- Clients that parse a transfer's public signals cannot parse a swap proof. The public list is the swap list above, not the transfer list with empty withdraw and escrow fields forced to zero.
