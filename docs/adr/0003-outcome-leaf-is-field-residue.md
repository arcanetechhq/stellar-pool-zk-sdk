# An Outcome leaf is the field residue of the predicate object

Superseded by ADR 0004. The Outcome stays that residue, and the Merkle leaf is the binding of the Outcome to the Opened pin.

A predicate slot carries the XDR of `operations` only. The pool runs those operations, takes the values left on the stack as `result`, and hashes `to_xdr` of the object `{ result, operations }` with SHA256. The digest reduced into the BN254 scalar field is the Outcome the circuit treats as a Merkle leaf.

## Considered options

- Send `result` in the transaction and hash those bytes unchanged. Rejected: execution already produces `result`, so the caller cannot choose a second copy.
- A hand-rolled byte codec with a domain prefix. Rejected: `to_xdr` already canonicalizes the object.
- Two public limbs for the raw 32-byte digest. Rejected: one Branch pin leaf is one field element.
