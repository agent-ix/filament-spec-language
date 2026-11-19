---
id: TC-2020
title: "A role on a whole model checks, emits and refuses its replicated form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218 FR-383
    type: verifies
---
# TC-2020: A role on a whole model checks, emits and refuses its replicated form

## Description

Verify that `role w on M` checks as a role on the whole model `M` selects, that S4 writes it in FSpec FR-465's whole-model form with no refusal, and that the replicated form over a model alias refuses at the alias in the checker. Scope: FR-218-AC-41 and FR-383-AC-68.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | a protocol over `(input: M::Actor)` with `role W on M` and `role R on M::Actor` | checks; `W` records the identity of the package `M` selects |
| 2 | row 1 with `role r each M from pop max 2 lifetime workflow` added | `invalid_model_binding`/`malformed-declaration`, refusal region text exactly `M`; no refusal for `pop` |
| 3 | ADR-027 §7's `Fill` with `role w on M`, compiled through S4 | a package; the role node's `model` is the text identity of `M`'s package, `replication` empty, no `model` dependency; FSL's I2 reader admits it |
| 4 | row 3 with `role w each M from cells max 2 lifetime workflow` | no package; S3 refuses `invalid_model_binding`/`malformed-declaration` at `M` |

## Expected Results

- Codes, causes and region text are asserted as fields, never message text.
- No S4 refusal names a role on a whole model.
