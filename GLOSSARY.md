# Note accumulator

How a note constrains the transaction that spends it.

## Language

**Note accumulator**:
The field of a note that states the constraint on spending that note.
_Avoid_: commitment, nullifier

**Unconstrained note (mode 000)**:
A note whose accumulator is zero. Spending it adds no route, exit, or outcome constraint.
_Avoid_: empty note, padding note

**Route pin (mode 001)**:
A note accumulator that fixes which output notes the spend must include.
_Avoid_: branch pin, transfer pin

**Exit pin (mode 010)**:
A note accumulator that fixes the asset, the Stellar account, and the amount of the public withdrawal.
_Avoid_: branch pin, withdrawal accumulator

**Branch pin (mode 011)**:
A note accumulator that fixes the set of Outcomes allowed when the note is spent.
_Avoid_: branch operator, branch accumulator

**Outcome**:
One allowed member of a Branch pin: a Spend predicate together with the values that predicate must leave behind.
_Avoid_: branch, outcome accumulator, exit, route pin

**Spend predicate**:
The program whose result is bound to one Outcome.
_Avoid_: Predicate, KYT predicate, operator, script

**Plan**:
The Outcomes of one Branch pin, held off the note by its Plan composer.
_Avoid_: route, route pin

**Plan composer**:
The author of a note's Branch pin. They hold its Plan and give the spender the Outcomes that may be published.
_Avoid_: automation service, spender
