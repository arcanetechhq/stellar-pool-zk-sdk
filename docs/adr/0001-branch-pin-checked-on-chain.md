# Branch pin is checked on chain and composed off chain

Superseded by ADR 0004. The circuit no longer proves leaf membership, and a Branch leaf binds an Outcome to an Opened pin.

A Branch pin commits a note to a set of Outcomes. The circuit only proves that the published Outcome is a leaf of that pin. The pool executes the Spend predicate and accepts the spend when the predicate leaves the committed values. The rest of the Plan stays with the Plan composer: a circuit cannot call contracts, and the note plaintext keeps the seven limbs already shipped.

## Considered options

- Ship all eight Outcomes in the encrypted note. Rejected: the Plan composer hands the spender only the Outcomes that may be published.
- Execute the Spend predicate inside the circuit. Rejected: `call` reads other contracts at spend time.
