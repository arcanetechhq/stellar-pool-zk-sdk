# Branch membership is anchored in the proof and checked by the pool

A Branch leaf is `Poseidon255(3)(DOM_PAIR, outcome, opened)`. `DOM_PAIR` is ASCII `PACC`. The Outcome is the field residue already produced from the predicate. The Opened pin is a Route pin, an Exit pin, or zero. The note keeps one accumulator: zero, a Route pin, an Exit pin, or a Branch pin. The Branch pin stores `Poseidon255(2)(3, branchRoot)`.

Each input publishes `branchRoot`, `openedPin`, `leafIndex`, and three sibling hashes. The circuit fixes those values and does not hash the path, which keeps the 6×6 circuit in the 2^17 domain. A nonzero root must match the note's Branch pin, and the Opened pin must match the output notes or the public withdrawal. A zero root forces those signals to zero, and the note is checked as a Route pin, an Exit pin, or unconstrained. The pool rebuilds the leaf and the depth-3 path with host Poseidon255 and requires the folded root to equal `branchRoot`. Index bit 0 hashes `(node, sibling)`; bit 1 hashes `(sibling, node)`. Unused leaves are 0, and every level hashes both children. An index above 7 is rejected.

A predicate is required for a nonzero root and forbidden for a zero root. Its bytes stay beside the proof. A spend has at most one Exit pin, whether it is a note accumulator or an Opened pin.

## Considered options

- Hash the path in the circuit. Rejected: the 6×6 circuit leaves the 2^17 domain.
- Send the path only beside the predicate. Rejected: the proof for one leaf could be paired with another leaf's predicate.
- Store the Outcome and the Opened pin together in the note accumulator. Rejected: the note keeps one field. The pair lives in the leaf.
