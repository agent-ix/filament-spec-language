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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The contract route has the same source-object exposure the fix round showed for the probabilistic route, but no test reaches it. A pre/post or invariant contract over a source object's operation (`Lamp::paint(c: Color)`) would carry source enum arguments into `ContractTarget::wire`. The only contract test substitutes values into a witness over a domain-package operation, so `scope.enums()` at contract_check.rs:745 is never exercised by a real path. Reviewer confidence medium: contract selection over a source-declared operation was traced by reading select_contract, not run. | fsl-replay/src/contract_check.rs:741-747, fsl-replay/src/contract_check.rs:335-356, fsl-analyze/tests/it/operation_contract.rs:946-981 |

## Dispositions

Round 1, checked against fix commit d15f16ec635119bfbd0364a2263e2ad50c99d87c (head d15f16ec635119bfbd0364a2263e2ad50c99d87c).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | Two of the four sites are now covered end to end. exact.rs:3982 (finish_replay, source route) is reached by the reachability test, and measure.rs:1046 (refute_mean) by the mean test; reviewer run 2/0. Two sites remain open. undefined.rs:89 is reachable in principle, since it runs only on the source route (`system.source_package()`) with the same scope, but it is untested and no input is yet shown whose undefined path holds a source-object step. plan_synthesis.rs:541: the unreachability argument (document.rs:347-357 universe keying, model.rs:1492-1508) covers universes only. FR-569 "Observations and evaluation" admits an initial snapshot naming a source population, and FR-120 check 3 keeps a population with no universe as `Population(None)`, so a source object with a `Color` field can enter a state without a universe. Either add tests or have the lead record accepted-no-change; both remaining arguments are one token, `scope.enums()`. |

Round 2, checked against fix commit 8c3085efea7d06867454f0a4da011f4811608243 (head 8c3085efea7d06867454f0a4da011f4811608243; tests and review files only, no production change).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | undefined.rs:89 is now covered by `a_source_enum_draw_is_spelled_in_an_undefined_counterexample` (probabilistic_replay.rs:1154-1191). It uses a source `Lamp::paint(c: Color, coin: Reference<M::Coin>)` that deletes the coin, and runs `analyze_items` to an `undefined-path` wire with the `Color::blue` draw. exact.rs:3982 and measure.rs:1046 were covered in round 1. Coder run 20 passed on this head (QSL-2302-tests-12-r2.log). plan_synthesis.rs:541 is deferred to QSL-3232 and QSL-3291. A plan with `model M` plus a source object is refused at emit by `root_regions.get(identity).ok_or(EmitRefusal::SourceMapInvariant)` (fsl-package/src/emit/model_correspondence.rs:75-77), because the source-model view has no model root. PR #3886 (QSL-3232) removes exactly that refusal by skipping the empty-alias source view. QSL-3291 (universe keying of scheme-less source populations) may still block the no-universe arm after that. The owed test is recorded on QSL-3291. |
| FND-002 | fixed | `a_source_enum_argument_and_result_are_spelled_in_a_source_operation_counterexample` (operation_contract.rs:988-1073) runs `analyze_items` on a contract item. The invariant `Dim` over a source `Lamp` is broken by `paint(c: Color): Color` with source universe `l`, so `ContractTarget::wire` writes `blue` as the argument and `red` as the result through contract_check.rs:745. |
