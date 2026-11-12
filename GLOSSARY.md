# Note accumulator

How a note constrains the transaction that spends it.

## Language

**Note accumulator**:
The single field of a note. It holds one constraint: zero, a Route pin, an Exit pin, or a Branch pin.
_Avoid_: composite accumulator, commitment, nullifier

**Unconstrained note (mode 000)**:
A note whose accumulator is zero. Spending it adds no route, exit, or branch constraint.
_Avoid_: empty note, padding note

**Route pin (mode 001)**:
A note accumulator that fixes which output notes the spend must include.
_Avoid_: branch pin, transfer pin

**Exit pin (mode 010)**:
A constraint that fixes the asset, the Stellar account, and the amount of the public withdrawal. A spend has at most one.
_Avoid_: branch pin, withdrawal accumulator, opened pin

**Branch pin (mode 011)**:
A note accumulator that stores the Merkle root of that note's Branch leaves.
_Avoid_: branch operator, branch accumulator, branch root

**Branch root**:
The Merkle root a spend publishes for a Branch pin note.
_Avoid_: branch pin, opened pin

**Branch leaf**:
The binding of one Outcome to one Opened pin. The pool checks that the opened leaf sits in the Branch pin.
_Avoid_: outcome, sibling route

**Outcome**:
The on-chain result of one Spend predicate. It is the mandatory preimage of a Branch leaf.
_Avoid_: route pin, exit pin, branch root

**Opened pin**:
The optional preimage of a Branch leaf. It is a Route pin, an Exit pin, or zero.
_Avoid_: exit pin as a name for both modes, second accumulator

**Spend predicate**:
The program the pool runs to produce the Outcome of one opened Branch leaf. A spend has one for each Branch pin it opens and none for any other note.
_Avoid_: Predicate, KYT predicate, operator, script

**Plan**:
The Branch leaves of one Branch pin, held off the note by its Plan composer.
_Avoid_: route, route pin

**Plan composer**:
The author of a note's Branch pin. They hold its Plan and give the spender the leaf that may be opened.
_Avoid_: automation service, spender
