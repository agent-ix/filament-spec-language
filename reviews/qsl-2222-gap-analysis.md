---
id: SR-202611210022442313144108431
title: "QSL-2222 loop proof gap-analysis for PR 3270"
type: SpecReview
analysis: gap-analysis
scope: "FR-218 AC-43/48; FR-730; PR 3270 changed Rust files"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen head: `65dfdca3d24282967f6119274f64f6066ede53dd`, diff `origin/main...HEAD`. Read-only review; no Cargo, builds, tests or source mutations, per dispatch. `git diff --check origin/main...HEAD` passed. PR-reported 229 passing / 1 failing is unverified here; terminal_two_models::a_subject_reading_one_identity_derives_the_deadlock_item_of_that_identity has no main baseline evidence, so its failure is unresolved and is not classified as unrelated. QSL-2037, the structured blocker of QSL-2222/QSL-1971, is Done (verified via tracker UUID); the retained blocked_by edge does not create an active blocker.

Old #3141 findings: false-invariant fixture now uses decreasing `dec` with `got.left > 0` and initial 2, so it reaches 0 after body steps (source verified; execution not rerun). The no-proof trace finding remains open because the replacement AC-4 tag still names a different scenario. The old E0599 repair is source-inspected only; no independent compilation claim.

## Verdict

**CONDITIONAL** — targeted trace and runtime-branch coverage gaps remain; repository-wide assurance is not established.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The no-proof unbounded repeat test moved from AC-1 to AC-4, but AC-4 specifies two proof-carrying repeats plus a maximum-only repeat. This test exercises the prose no-loop-proof/no-obligation rule instead. Retag it to FR-730 (and TC-2022), retaining AC-4 on the dedicated four-item fixture. | FR-730-AC-4; fsl-analyze/tests/it/loop_proof.rs:57-64 |
| FND-002 | medium | The executable discharge fixtures cover only one nonnegative scalar variant. There are no runtime assertions for lexicographic tuples, negative first-decision floors or undefined invariant/variant refutations, although the change introduces each branch. Add focused checker/replay cases so wrong tuple ordering, omitted floors, and undefinedness cannot silently pass the loop-proof suite. | FR-730; fsl-analyze/tests/it/loop_proof.rs:252-330 |

## Coverage

Reconciliation: quoin matrix 0.36.8, quire 0.52.0 (engine 0.62.2), both exit 0. Whole-repository engine output retained in matrix.json and coverage.json. Static totals: tagged 4659; untagged 1161; tagged-by-ignored-test 156; method-without-symbol 104; retired 13. All 6093 rows have no run evidence. All six targeted criteria are tagged by active tests. Coverage extraction reports 22254 diagnostics and 5614 untracked symbols, with Rust census candidates 13121 / tagged 12563 / bound 12064; these are not a clean repository-wide assurance result. Whole-repository missing tags were not assigned to this PR without baseline attribution. The artifact verdict concerns the dispatched loop-proof subset; repository assurance does not pass.

Plan completion: not assessed

Reverse-gap/stub inspection covered changed public items and their callers across 36 files: slot type admission, derived items/selection/identity, proof-product evaluation and canonicalization, model checking, certificates, replay, result encoding and assertions. These behaviors have FR-218/FR-730 owners. No source stub found in the change. This is a bounded PR audit, not an exhaustive inventory of the repository's entire public API. Optional separate semantic-review fan-out was not run; ordinary code/test-intent review ran under the explicit dispatch scope.

## Examined files

- `fsl-analyze/src/analyze.rs`
- `fsl-analyze/src/analyze/document.rs`
- `fsl-analyze/src/analyze/items.rs`
- `fsl-analyze/src/analyze/protocol_item.rs`
- `fsl-analyze/src/claim_family.rs`
- `fsl-analyze/src/model_check/check.rs`
- `fsl-analyze/src/model_check/reduce.rs`
- `fsl-analyze/src/zone_check/certificate.rs`
- `fsl-analyze/src/zone_check/settle.rs`
- `fsl-analyze/tests/it/loop_proof.rs`
- `fsl-analyze/tests/it/protocol_composition.rs`
- `fsl-analyze/tests/it/protocol_system.rs`
- `fsl-eval/src/automaton/key.rs`
- `fsl-eval/src/simulation/model/encode.rs`
- `fsl-eval/src/simulation/protocol.rs`
- `fsl-eval/src/simulation/protocol/program.rs`
- `fsl-eval/src/simulation/protocol/settle.rs`
- `fsl-eval/src/simulation/subject.rs`
- `fsl-eval/tests/it/automaton_key.rs`
- `fsl-replay/src/certificate/closure.rs`
- `fsl-replay/src/lib.rs`
- `fsl-replay/src/model_check/counterexample.rs`
- `fsl-replay/src/model_check/item.rs`
- `fsl-replay/src/model_check/replay.rs`
- `fsl-replay/src/model_check/replay/timed.rs`
- `fsl-replay/src/model_check/smt_query.rs`
- `fsl-replay/src/outcome/counterexample.rs`
- `fsl-replay/src/spine.rs`
- `fsl-route/src/deadlock.rs`
- `fsl-route/src/model_request.rs`
- `fsl-route/src/request.rs`
- `fsl-semantics/src/check/protocol_clause/tests/refusal_corpus.rs`
- `fsl-semantics/src/check/protocol_clause/tests/rules.rs`
- `fsl-semantics/src/check/protocol_obligation/tests.rs`
- `fsl-semantics/src/check/protocol_slots.rs`
- `fsl-semantics/src/check/refusal.rs`

## Dispositions

### Round 1

Reviewed fix delta `65dfdca3d2..87c121d9` at frozen head `87c121d9f286074619792730fef7d0eb35801a6d`. Original findings above remain historical and unedited.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The no-proof fixture now traces the owning prose requirement. The separate four-item fixture keeps its AC-4 tag. |
| FND-002 | fixed | loop_proof.rs adds lexicographic success/refutation cases, scalar/tuple negative-floor refutations, undefined invariant/variant refutations, and simultaneous undefined guards at entry/post-body decisions. Helpers assert terminal counts, exact failure kind, Refuted and reproduced replay. Static evidence closes the missing assertions; execution is pending the assigned test lane. |

#### FND-001 after excerpt

```rust
#[trace("TC-2022", "FR-730")]
#[test]
fn a_repeat_without_a_variant_derives_no_loop_proof_items() {
```

#### FND-002 after excerpt

```rust
fn tuple_variants_use_lexicographic_order_in_checker_and_replay() {
    for (variant, proved) in [
        ("Pair(got.left, 3 - got.left)", true),
        ("Pair(0, got.left)", true),
        ("Pair(3 - got.left, got.left)", false),
        ("Pair(0, 3 - got.left)", false),
    ] {
```

### Current disposition verdict

**CONDITIONAL** — all findings have final dispositions, and static inspection found no new regression in the fix delta. No still-open finding remains. Focused runtime execution remains pending with the assigned lane; this reviewer ran no Cargo/build/tests or matrix. The reported compile-only checks are lead-provided evidence, not independently rerun. The known terminal_two_models failure still has no main baseline or resolution in this review. `git diff --check 65dfdca3d2..87c121d9` passed and the source worktree was clean.

Arch owns the FSpec amendment/reader pin. Architecture confirmed that source-backed typed Rust and JSON serve FR-730 in full, and only package-backed documents must return typed unsupported until the amended FR-368 pin. The extended existing source resolver and regression fixture match that ruling; no architecture hold remains.

### Round 2

Reviewed only the standard-alignment delta `87c121d9..7378ff93` at frozen head `7378ff93b931c17141cc57a06b1d5bfab6bf4e0c`. Architecture explicitly requires invariant-only obligations for proof-carrying bounded repeats, exhaustive derived items with no item max_depth, and freedom-item prefix followed by repeat control-tree order. The implementation and focused assertions match those constraints. No new findings. Runtime execution counts remain pending; this is not a runtime-pass claim.

- Bounded proof filtering: `fsl-route/src/deadlock.rs` separates all proof-carrying repeats from unbounded ones; `model_request.rs::derived_kinds` emits every invariant and only unbounded termination. `document.rs::resolve_protocol_freedom` rejects a bounded termination selection as inapplicable-kind.
- Ordering: `ProtocolTemporalItem::reading` reads the checked controls vector, whose documented contract is preorder with authored child order (`protocol_clause/types.rs:689-696`). Deadlock/time-lock/race applicable prefix remains before loop items. The Zulu/Alpha/Bounded fixture asserts concrete repeat ordinals and invariant/termination order for one and two caller requests.
- Horizons/limits: derived writer continues using `derived_item`, source resolver rejects temporal.max_depth, and the source JSON test asserts the emitted field is absent, max_states=1 produces incomplete/MaxStates=1, and both explicit horizons refuse. The bounded source JSON case accepts its invariant and refuses its termination selection.
- Bounded true/false invariants: the new fixture derives exactly one invariant, checks proved with no counterexample for true, and requires the typed invariant-false counterexample plus reproduced replay for false.
- Trace: changed AC-4 and new AC-5/6 have explicit executable trace attributes in loop_proof.rs. Static tag/assertion comparison only; no matrix rerun was authorized.
- `git diff --check 87c121d9..7378ff93` passed. No Cargo/build/tests/full gates/matrix ran. Parent reports narrow compile checks passed, but focused runtime results and terminal_two_models baseline/resolution remain pending.

All three code findings and both gap findings retain their round-1 final outcomes. There are no still-open or undispositioned findings, so round 2 adds no finding-outcome rows. No new finding was identified. Current merge assessment remains conditional on the assigned focused runtime results and treatment of the known terminal_two_models failure; no architecture hold remains.

### Round 3 — explicitly authorized runtime-failure correction

Reviewed the explicitly authorized extra round limited to the four runtime failures, delta `4473cdbca0..2aa81cd9`, at frozen head `2aa81cd9a8a3b35c33737e5122f479609f2bf91d`. Read `/tmp/protocol-3270-diagnostic-only.txt` in full. No new finding or scope expansion was identified. No Cargo/build/tests/matrix/full gates ran in this reviewer.

- Diagnostic evidence at `82c2f073`: four selected, one passed, three failed, zero ignored. The bounded direct test passed. Both tuple/floor failures were strict replay-package `IllTyped`/`TypeMismatch` at semantic node 22 (`tuple_value`); source JSON reached the correct bounded termination disposition but its assertion demanded a terminal `result` string.
- `family.rs::Application::call_parameter` now returns the existing `parameter(index)` for Tuple alongside Function/Imported. Tuple parameters are read by `positions.get(index)` after constructor arity validation. Application frame acceptance uses `frame.order[frame.typed.len()]`, so the declared position corresponds to the actual source operand; tuples preserve identity argument order, and only IEEE calls reorder contextual operands.
- The existing `integer_widens` guard and exact `convert` path perform the change: bounded integer to Integer or containing bounded range; identical/narrower/non-integer cases are unchanged. Existing already-lowered Integer avoidance remains intact. No new conversion semantics, contract, dependency or pin was added. Generic tuple use is owned by existing FR-065 tuple-constructor wording/AC-10 and Contract IR FR-347; FR-218/FR-730 traced tuple/floor tests exercise strict tuple replay. Existing FR-065-AC-9 tests cover the same converter's widening/identity/narrowing/literal behavior for functions; those tests were inspected, not executed here.
- Bounded fixtures now author the required exhausted control via loop_unit_with_maximum; true/false invariant and no-termination assertions remain unchanged. The one bounded direct pass is reused as explicitly permitted by arch.
- Source JSON keeps the successful invariant assertion and checks the rejected termination's request_index=1, category=refusal, settled_by=disposition, invalid_capability/inapplicable-kind. It no longer incorrectly calls the terminal-result helper for a disposition. Unbounded, limited and horizon phases still require their terminal record and their original exact results/causes.
- Diagnostic-only inspect_err output runs only on replay errors and preserves the full refusal/variant; it changes no settlement path or assertion.
- `git diff --check 4473cdbca0..2aa81cd9` passed; worktree source remained clean. Spec is unchanged from the prior focused PASS, so no new spec analysis or matrix was run.

All prior findings retain their final dispositions; there are no new findings or reopened findings. Accordingly this round has no finding-outcome rows.

**CONDITIONAL on the three queued final-head runtime results only.** Static code/gap checks found no remaining defect in this authorized delta. The supplied architecture ruling permits reuse of unaffected 7378ff93 results and the bounded 82c2f073 pass. The lead reports the terminal_two_models failure exactly matches main; arch explicitly says that pre-existing failure does not hold this merge. No architecture or baseline hold remains. The final three runtime results are not yet available to this reviewer and are not claimed passed.
