---
id: TC-1706
title: "A clause-run function selection admits the run value form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: verifies
---
# TC-1706: A clause-run function selection admits the run value form

## Description

A clause-run function selection admits the public canonical argument forms
and resolves reference and population arguments against its selected
snapshot.

Scope: FR-650-AC-7 and FR-312-AC-11.

## Test Procedure

Keep FR-650-AC-7's `sameIdentity` public `call` requests with an identity-form
and a key-form reference to `child`, and its `short` call with a Text argument.
For FR-312-AC-11, run `sameIdentity` as a `clause.selection.function`
request using FSpec FR-181's tagged canonical Reference value
`{"type":"reference","universe":U,"object_type":T,"identity":"6368696c64"}`
for both arguments, then use the canonical `root` and `child` references to
distinguish the false result. In the same healthy current snapshot, run
`nonempty` with its Population argument's value `{"population":P}` and run it
again with a population identity the snapshot does not hold. In each request,
resolve the reference or population through the named snapshot; do not use a
default or synthesized context. Keep the `clause` request values distinct
from FR-650's internal run-value spellings used by the existing `call`
coverage.

Tag the tests `#[trace("TC-1706", "FR-650-AC-7", "FR-312-AC-11")]`; they
live in `tests/it/run_values.rs`.

## Expected Results

FR-650-AC-7's `sameIdentity` and `short` calls report `success`, `truth: true`,
exit 0. For FR-312-AC-11, two canonical `child` references report `success`,
`truth: true`, exit 0; canonical `root` and `child` references report
`violation`, `truth: false`, exit 10. `nonempty` with the selected snapshot's
`{"population":P}` reports `success`, `truth: true`, exit 0; a Population
selection not present in that snapshot refuses at stage `admit` with
`invalid_runtime_input`/`wrong-role-mapping`, exit 20. The report for each
request equals the result from the same admitted `ClauseRunRequest` built
directly.
