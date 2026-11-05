# 02 — Mode 001 transfer accepts the pinned notes and rejects a substitute

**What to build:** A depositor can pin the exact output notes of a later transfer. The transfer succeeds when those notes are produced, in either output position. A transfer that substitutes a different note does not succeed.

**Blocked by:** 01 — Unconstrained notes spend on the new accumulator field

**Status:** done

- [ ] A transfer that outputs the pinned notes checks and is accepted
- [ ] A transfer that replaces a pinned note fails the witness check and is not accepted
- [ ] The same pinned notes are accepted when their output positions are swapped
- [ ] A masked output may be any note without failing the check
- [ ] An included commitment of zero or one is rejected
- [ ] A second input whose accumulator is zero can be spent in the same transfer
