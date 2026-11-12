# The branch tree is a circom Merkle tree

Superseded by ADR 0004. The pool rebuilds the depth-3 tree. The circuit only anchors the path.

A Branch pin holds eight Outcomes in a depth-3 tree. The circuit checks that the published Outcome is a leaf of that tree. The pool does not rebuild the tree, so its nodes are not the host Poseidon.

## Considered options

- Hash the nodes with the host Poseidon so the pool could rebuild the root. Rejected: the pool only receives the published leaf.
