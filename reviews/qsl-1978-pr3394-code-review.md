---
id: SR-202611212117102555233480001
title: QSL-1978 PR3394 inline Integer code-review
type: SpecReview
analysis: code-review
review_set: subset
scope: agent-ix/filament-spec-language; QSL-1978; fsl-semantics/src/model/intake/model_unit.rs,
  fsl-semantics/tests/it/model_unit_input.rs, fsl-analyze/tests/it/plan_search.rs
relationships:
- target: ix://agent-ix/filament-spec-language/FR-710
  type: reviews
---

## Summary

Ticket: QSL-1978. Initial native Codex review of PR3394. Exact model identifier is unavailable; no alternative backend is claimed.

## Verdict

FAIL — two actionable defects.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Machine and process branches discard inline Integer bookkeeping, omitting the used value_type construct and bypassing interval and identity-collision validation. | fsl-semantics/src/model/intake/model_unit.rs:585-586,613-614 | implementation-bug-despite-evidence |
| FND-002 | high | The generated/generated collision fixture uses distinct OrderTotal and OrdeRTotal identities, so its expect_err rejects correct compilation and fails the focused suite. | fsl-semantics/tests/it/model_unit_input.rs:1283,1299 | implementation-bug-despite-evidence |

## Evidence

FND-001: read_state_machine and read_process both delegate their fields to read_object. The new path builds additional_types and inline_integers together. Their top-level branches move only additional_types, unlike object and push_model_type. Consequently, a model containing only `process Flow { n: Int[0,3]; }` generates FlowN with custom value_type kind but never sets has_value_type; the analogous `machine Gate { n: Int[0,3]; states (Open); }` has the same defect. Admission rejects the unregistered custom kind. Even when another named scalar supplies the kind entry, these two paths bypass the new read_value_type and declaration collision loop, so reversed intervals and authored/generated collisions do not receive the required inline-span refusal. Propagate their inline metadata through the same shared assembly path and cover each declaration family, including invalid bounds/collisions, without depending on an unrelated scalar to populate constructs. This conclusion is from static tracing, not a claimed reproduction.

FND-002: FCD PackageIdentity::alias_identity concatenates owner verbatim with the capitalized field slug. `Order` + `Total` is `OrderTotal`, while `Orde` + `RTotal` is `OrdeRTotal`; the source's fifth row therefore has no collision. Its failure does not establish that the two actual authored/type collision orders failed. Replace that row with an actual generated/generated collision, for example `object A { bC: Int[0,3]; } object AB { c: Int[0,3]; }`, whose aliases both mint `ABC`, and retain exact code/cause/span assertions and both authored source orders. Do not change the producer to reject these distinct existing names. The supplied runtime failure and this deterministic minter trace agree; the runtime output alone does not identify the loop row.

## Coverage

The source review covers exactly three changed Rust files at head `1f7de6b7fab1a8569f07e379dff72c7a26ebdf3d`, relative to `ba280846beb6e8bf79c89b179c8841839c203090`. Repository instructions, Rust implementation/testing/wire/safety rules and applicable callers were read. No applicable AssuranceProfile was found. No workflow, dependency, lint, test-selection or skip change is present. No code, spec, index, branch or commit was changed by this reviewer.

The Shop regression preserves identity lookup, exact OrderId/OrderTotal bounds and both constraint identities/origins, all three used kinds, invariant admission and repeated byte equality. Full end coordinates are emitted on the new inline definitions and bounds through existing source_span; existing declaration origins remain unchanged. The native visitor changes the current base's named fixture back to inline Int[0,2], keeps a named control, checks the actual admitted range and retains all four result/state ordinals. Optional/nominal tests remain present. Named bounds and inline bounds share the parser and scalar writer; identity minting calls the pinned FCD implementation rather than duplicating it.

Lead-supplied focused runtime evidence for this exact head: 31 passed, 1 failed, 0 ignored: semantics model_unit_input 29 passed and 1 failed; analyze two selected tests passed. The only failing function is inline_integer_fields_refuse_constrained_alias_identity_collisions at its expect_err. The reviewer did not execute cargo or runtime tests. Lead-supplied production cargo check passed; this is not an all-target or whole-crate runtime result. git diff --check passed in this review. Final build/landing gates remain the lead's responsibility.

Scope units (verbatim excerpts; long criteria are excerpted to 600 characters):

```yaml
scope:
- id: FR-710
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: 'The compiler SHALL map each declaration to the document node FSpec FR-154

    admits for it, under the FR-208 meaning in this table, and SHALL write one

    `constructs` entry for each kind it uses and none for a kind it does not use:'
- id: FR-710
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: 'A collision with an authored or

    another generated type identity SHALL refuse under the existing

    `ambiguous_declaration`/`ambiguous-name` rule, at that inline type span,

    with no document written.'
- id: FR-710-AC-1
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: Test (TC-1942) |
- id: FR-710-AC-2
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: Test (TC-1942) |
- id: FR-710-AC-3
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: Test (TC-1942) |
- id: FR-710-AC-5
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: Test (TC-1943) |
- id: TC-1942
  path: spec/test-cases/TC-1942-compile-a-model-unit-to-an-admitted-domain-package.md
  role: examined
  excerpt: 'Step 4: each unit refuses `ambiguous_declaration`/`ambiguous-name`, located

    at the inline field type `Int[0,9]`, and writes no domain package document.'
- id: FSpec-FR-154
  path: filament-specification/spec/functional/type-model/FR-154-admit-domain-package-model.md
  role: context_only
  excerpt: 'A value type (FR-208

    `filament.meaning.model.value-type/v1`) binds its native in one of two ways:

    by `scalar` and constraints'
- id: FCD-FR-095
  path: filament-core-data/spec/functional/FR-095-mint-package-identity-and-provenance.md
  role: context_only
  excerpt: The frontend SHALL pass the `<Name>` part — the declaring artifact's id
    — through verbatim, and SHALL NOT slug it
- id: fsl-semantics/src/model/intake/model_unit.rs
  path: fsl-semantics/src/model/intake/model_unit.rs
  role: examined
  excerpt: '//! FR-710: compile an FSL model unit to its FR-154 domain-package document.'
- id: fsl-semantics/tests/it/model_unit_input.rs
  path: fsl-semantics/tests/it/model_unit_input.rs
  role: examined
  excerpt: '//! FR-710-AC-5 (TC-1943): a package input supplies a model unit in place
    of'
- id: fsl-analyze/tests/it/plan_search.rs
  path: fsl-analyze/tests/it/plan_search.rs
  role: examined
  excerpt: '//! QSL-1978: the plan search engine (FR-755, FR-1011, FR-1012) over FSpec'
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: Both machine and process assembly now call push_model_type, which propagates inline_integers and preserves shared interval/collision validation; object assembly uses the same helper. |
| FND-002 | fixed | Round 1: Both aliases mint ABC under unchanged FCD case-preserving owner/capitalized field-slug rules. All four other collision rows, both authored orders, and exact code/cause/span assertions remain. |
| FND-003 | fixed | Round 2: reserved count replaced by number/LevelNumber, with complete machine/process fixture bodies and all original interval, identity, origin and refusal oracles retained. |


## Disposition round 1 evidence

Reviewed frozen head `d01d8c1c41547bb937bc21c403a2ee4e38145463` against initial head `1f7de6b7fab1a8569f07e379dff72c7a26ebdf3d`. Initial IDs, findings rows and all original review text remain verbatim above. No new finding was established in this round.

FND-001: object, machine and process now use existing push_model_type, which moves inline_integers before additional types and preserves each owner's wire index and inheritance links. Every read_object-consuming assembly branch now propagates the metadata to the common used-kind, interval and identity-collision checks. The new positive machine/process test requires exactly their own meaning plus value_type (no incidental named scalar), resolves LevelCount from the field, checks exact source endpoints and constraint origins, then reads the admitted ScalarValueDomain::Integer [-2,3] and admitted field-to-scalar key. Shared object controls continue to assert constraint identities. New negative tests cover reversed and overflow bounds and authored collisions in both orders plus actual generated/generated collisions for each family, requiring exact diagnostic code/cause and the inline source slice. No optional/nominal/native ordinal control is removed.

FND-002: the false fifth row is replaced by A.bC and AB.c, both of which mint ABC. The authoritative minter and all four other negative rows are unchanged. Required authored source orders and exact diagnostics are retained; no producer behavior was changed to manufacture a collision between distinct names.

Static disposition: both original defects are fixed; no new defect found. Focused production cargo check success was supplied by the lead; this reviewer ran no cargo, runtime tests, build, Kani or matrix. Own git diff --check passed. The six affected inline_integer_ runtime functions on this exact head remain pending at the time of this review; there is no claim that they passed. The previous exact-head initial result remains 31 passed and 1 failed at the initial head, not a current-head runtime result. If all six affected functions pass, the retained combined focused inventory is 35 distinct test functions (33 semantics and 2 analyze), not 37. Landing remains conditional on actual current-head runtime evidence, predecessor integration and the lead's final gate.

Gap assurance remains bounded and CONDITIONAL: the initial matrix accepted-size refusal is unresolved, no repeated oversized matrix request was run, and no engine-backed repository PASS is asserted. The new source behavior is still owned by FR-710 Compilation, FSpec FR-154 scalar/constraints and FCD FR-095 identity rules. No new schema, kind, identity namespace or numeric family was added. Plan completion: not assessed.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | high | The round-1 machine/process fixtures use reserved count as a field identifier, causing all three new tests to fail during parsing before their intended admission and refusal assertions execute. | fsl-semantics/tests/it/model_unit_input.rs:1321; fsl-cst/src/token.rs:165 | implementation-bug-despite-evidence |

This row records the actual round-1 fixture defect subsequently fixed in the reviewed round-2 delta; it is not an unresolved defect at the round-2 head. The lead supplied d01d8c1c41547bb937bc21c403a2ee4e38145463 runtime results after the first disposition was posted: the three old inline functions passed and all three new machine/process functions failed in parsing. Thus round-1 source inspection established the production propagation fix, but the new runtime oracles were not exercised. The machine parser's backtracking reported expected states at the reserved field token; the source issue is the Count keyword (fsl-cst/src/token.rs:165), not an absent requirement for an extra custom kind.

## Disposition round 2 evidence

Reviewed round-2 source head `de6a521870893d6d9a9bc8da131251fc77598586` relative to `d01d8c1c41547bb937bc21c403a2ee4e38145463`. Production code is unchanged. FND-001 and FND-002 retain their verified fixed outcomes; all initial findings and round-1 disposition rows remain verbatim.

The new fixtures replace reserved count with legal number and consistently update LevelCount to LevelNumber, including both authored collision source orders. The machine has operation toggle, Open/Shut states and its transition; the process has operation go and a begin step. These are established FR-710 / current CST fixture forms. All exact interval [-2,3], admitted field-to-scalar binding, typeRef, origin endpoints, bounds origins, reversed/overflow refusal, exact code/cause/span and genuine generated/generated collision assertions remain. No grammar or production behavior is widened to accommodate a test.

The two-used-kind expectation is preserved and grounded in FCD semantic-ir.schema.json TypeDefinition.states/transitions/steps, which reference the nested state/transition/step definitions. Those definitions have identity/origin and their own member data, without custom kind fields. Current read_state_machine and read_process put those records into the owning wire node; they do not emit separate custom type definitions for them. Each fixture therefore uses its owner meaning and generated scalar value_type; adding enum or state/step construct meanings would be incorrect.

Runtime history is kept separate by head: the initial head had 31 passes and 1 incorrect collision-fixture failure; d01 had three affected old inline passes and three new fixture parse failures. None of those counts is a claim that the initial 32 all passed or that the new oracles ran at d01. The retained final focused inventory is 35 distinct functions (33 semantics, 2 analyze), subject to actual final affected-test evidence. Reviewer ran no cargo, runtime, build, baseline, Kani or matrix. Own diff --check passed.

Gap assurance remains CONDITIONAL: the initial computed-matrix accepted-size refusal remains unresolved and no oversized retry was run. No new unowned behavior, stub, invented kind, schema or numeric family was added by the fixture repair. Plan completion: not assessed. No whole-repository assurance PASS or QSL-1978 completion is claimed.

## Current runtime evidence

The lead supplied the affected machine/process test result at exact source head `de6a521870893d6d9a9bc8da131251fc77598586`: `model_unit_input::inline_integer_machine_and_process_fields_` ran three tests, all passed, zero failed or ignored, command exit 0. The new tests now reach their admission, exact interval, identity/origin and refusal oracles. FND-003 is fixed with actual runtime evidence, and the propagation regression checks supporting FND-001 now pass.

Retained focused evidence totals 35 distinct test functions: 27 unaffected semantics functions from the initial head, the three old inline functions passing at d01d8c1c41547bb937bc21c403a2ee4e38145463, the three new machine/process functions passing at de6a521870893d6d9a9bc8da131251fc77598586, and two analyze functions from the initial head. These are retained results across the reviewed source deltas, not a single-head execution of 35 tests and not a whole-crate runtime suite.

Owned code disposition: all three recorded defects fixed, no unresolved source finding at de6a521870893d6d9a9bc8da131251fc77598586. Approval remains conditional on the requested dependency-pin/current-main integration review and the lead's exact-final-head all-target gate after latest review artifact custody. Finite predecessor must land first. The final-round review remains open for that integration delta; no final comment has been posted yet.

## Dependency integration inspection

Inspected rebased source head `71739753afe2d0a9de02ce02f59d08764f268e90` on base `cf8d85826db560569cd8806cce07007d4b493e0c`, with merged pin commit `21edb2ead1c8b69768beedaab003e077ce4ed042` verified as an ancestor. The three owned Rust files and FR-710/TC-1942 are byte-identical to the previously reviewed de6 source; the branch has no local pin edits or API workarounds. Its net own diff contains the three Rust files and two review artifacts.

The current main dependency delta selects filament-contract-model revision `96554ea506a19b16ff9863ad3d30066578bc609c` and filament-specification revision `f6b0480cd90f5d1b5c5ea64f694895bb7b5ce2e0`. FSpec FR-154 and FR-208 source files are unchanged between the prior and current pins. FCD identity/schema dependency remains at the reviewed revision. Relevant model-member reader changes admit hyphens in artifact IDs while retaining the member-name rule; the scalar-operands production code is unchanged. Current-main model intake, model-unit producer, domain-package records and this branch's native plan test module have no integration delta. Adjacent changes concern composition/checked-package functionality; this review does not independently qualify that upstream functionality. No relevant inline Integer API adaptation or new source defect was found.

The full focused 35-function requalification on this new-pin source head is pending at the time of this integration inspection. This is a source review, not a claim that old-pin runtime results establish new-pin execution. Final artifacts/comments remain open for actual new-pin results and any material finite-predecessor integration delta.

## Final round 2 verdict

All three recorded defects are fixed. The lead supplied new-pin requalification at exact source head `71739753afe2d0a9de02ce02f59d08764f268e90`: all 33 model_unit_input tests and both selected analyze tests passed, 35 passed total, zero failed or ignored, command exit 0. This is one pinned-head focused run. The lead also supplied the new-pin production cargo check success for fsl-semantics and fsl-analyze (exit 0, with the existing unrelated sample_fingerprint warning). The reviewer executed no cargo/build/runtime gate.

Owned code review: APPROVED, conditional only on the lead validating any subsequent finite-predecessor/current-main source or dependency delta and running the mandatory all-target gate at the exact final custody head. There is no unresolved owned source finding or pending owned runtime check at this reviewed head. The latest two review artifacts must be committed before that final gate; no artifact commit may follow the gate it is supposed to cover. Finite predecessor lands first; this review does not authorize landing ahead of it or closing QSL-1978. A subsequent material integration delta must receive a bounded continuation of this same final review, not a new initial review or a third fix cycle.

Gap verdict remains CONDITIONAL for the documented unavailable matrix; no full-repository assurance PASS is asserted. Original finding rows and historical round-1 evidence/outcomes remain unchanged. The later actual runtime and integration evidence above supersedes only historical pending statements, without rewriting that history.
