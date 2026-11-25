---
id: SR-3880002
title: "Gap analysis of QSL-2302 source-enum wire helpers (FSL PR #3880)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@52645e4479a2890db431b75592254521d0746992; fsl-eval/src/simulation/model/encode.rs, fsl-replay/src/outcome/counterexample.rs, fsl-replay/src/contract_check.rs, fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/measure.rs, fsl-analyze/src/analyze/exact/undefined.rs, fsl-command/src/command/plan_synthesis.rs, fsl-analyze/tests/it/{value_validity,operation_contract,probabilistic_replay,package_probabilistic_refutation}.rs"
review_set: subset
---
# SR-3880002: Gap analysis of QSL-2302 source-enum wire helpers (FSL PR #3880)

## Summary

Ticket: QSL-2302. PR agent-ix/filament-spec-language#3880, head
52645e4479a2890db431b75592254521d0746992. Plan completion: not assessed.

The PR changes no spec document and adds no trace tag. `quire matrix --scope . --format json`
exit 0 (638 rows), so the matrix is the same as on main. The four new tests carry no
`#[trace]` attribute. No acceptance criterion names a source-enum value in an assignments,
contract or probabilistic counterexample, so this does not leave a criterion untested.

Owning requirements: FR-286 (the `Assignments` and `Contract` rows: "one FR-181 typed value per
parameter node") and FR-120 "Identities" / typed forms (`{"type":"enum","name":…,"member":…}`,
where `name` is the declaration identity text). Every changed line traces to these, so no code
lacks an owner (no reverse gap). There are no stubs, and no tautological or helper-generated
oracles: the tests assert literal JSON.

Spec conflict check (FR-120 vs FSpec FR-181-AC-19): the conflict is real but older than this
PR. FR-181 writes `<qualified-name>`, and AC-19 pins the bytes `"name":"Shop::Color"`. FR-120
says FR-181 "leaves the text form to the implementation" and writes the declaration identity
text `{"node":…,"package":…}` instead. This PR only extends the FR-120 form to source enums.
The conflict was raised as FND-004 of reviews/qsl-580-pr1646-gap-analysis.md and deferred to
STD-493, which is still in Backlog. No new finding.

## Verdict

**CONDITIONAL** — traceability is unchanged and complete for the scope. One low coverage gap:
four of the changed production call sites are never run with a source enum.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test runs the changed call sites in exact.rs, measure.rs, undefined.rs or plan_synthesis.rs with a source-enum value. The new tests call the public helpers (probabilistic_wire, probabilistic_undefined_wire, ContractTarget::wire, assignments_wire) directly, with substituted values. If one of those four sites passed `&[]` instead of `scope.enums()`/`input.enums`, every test would still pass. | fsl-analyze/src/analyze/exact.rs:3972-3983, fsl-analyze/src/analyze/exact/measure.rs:1046, fsl-analyze/src/analyze/exact/undefined.rs:89, fsl-command/src/command/plan_synthesis.rs:537-542 |

## Coverage

- Matrix: `quire matrix --scope . --format json` exit 0, 638 rows; no criterion added, removed or retagged.
- Reverse gaps: none. The changed behavior is owned by FR-286 (Assignments, Contract) and the FR-120 typed forms.
- Stubs / inflation: none.
- Semantic review: skipped (not opted in); the code-review SR-3880001 covers test intent.
- Plan completion: not assessed
