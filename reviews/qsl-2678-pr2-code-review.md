---
id: SR-2026112017124277723988805
title: "Code review — QSL-2678 PR #3212 stale oracle refresh"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3212 diff: fsl-analyze, fsl-command, fsl-package, fsl-replay, fsl-semantics, tests/corpus/values, tests/it/source_corpus.rs"
review_set: subset
evaluated_revision: "c04100a5636404f565e94f5e97ac0ee581363534"
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Ticket: QSL-2678. Reviewed PR #3212 at head `c04100a56` (base `3da51c971`) by reading the diff, with context from FSL `origin/main` `ad5547bde` and FSpec `origin/main` `cd89fcb31`. Each changed expected value was checked against the spec text or a recorded ruling. Most refreshed oracles match the spec. One production change bends the checker to fit a stale test and contradicts FR-880-AC-8 and FR-383.

## Verdict

**FAIL — fix round needed.** One high, one medium in this file; the AC-coverage lane adds one medium and one low (below).

These refreshed values are spec-backed:

- The `type`-only units (`no-profile-unit`, `obj-src-decimal`, `obj-src-float`, the prepared-command `type Digit`) now give `missing_declaration`/`empty-package` at S3, and the dimension-plus-unit unit `obj-src-qty` succeeds. Both follow FR-910 behaviour 3 and FR-910-AC-1/2.
- `conv-text-num` is admitted by FSpec FR-478-AC-11 / TC-964 TN-08.
- `opt-some`, `opt-Some-result` and `qty-scalar-mult` now succeed.
- `fn-param-named-set-2006` gives `invalid_syntax`, which is QSL-2006's expected outcome. QSL-2006 is Done.
- IEEE-bearing `Set`/`Bag`/`OrderedSet` are admitted, as FR-323-AC-3 and FR-534-AC-1 require.
- A function reference to an attempt-bearing protocol checks. FR-383 says it is written, and FR-218-AC-8 gives the reference rule.
- The whole-population, `seq`, post-test and decision-reads-body refinement cases are dropped. Those constructs are now written (FR-375-AC-14's unbuilt list, FR-880).
- The driver-owned unsupported document follows FR-286-AC-6.
- The infinite-trace `eventually` over `[0, 2]` now settles success.
- The replay change in `replay.rs` makes an enabled transition with a wrong post-state classify as `PostState`. The default closure had been mapping it to `not_enabled`, which matches the function's own doc. FR-128-AC-3's test is un-ignored.
- The 100k protocol-location fixture raises the emitter limit and does not lower the test, per the recorded ruling. The I2 read stays ignored under QSL-2691.

The rulings on QSL-2577 `work_units`, `Engine::external` and QSL-2709 integer containment do not apply to anything in this diff.

## Scope and skill provenance

The code-review and rust-review lanes ran by reading the diff (`gh pr diff 3212`). Context came from the read-only checkout `/Users/peter/dev/qsl-2631-code` through `git show`. Under the brief there was no worktree, no build and no test run, so `quoin gap-analysis` / `quoin matrix` could not run at the PR head. A manual AC-to-test check over the changed tests was done instead and is recorded under Coverage. No Python and no logic-bearing shell is in the diff.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | `protocol.rs` `K::Repeat => false` makes every repeat that progresses through a nested repeat withhold `RepeatStepsThroughRepeat` (FND-001). |
| Oracle integrity | The corpus outcomes are refreshed, but their `flips_with` gap keys still report the gaps as open (FND-002). |
| Duplication | The source-view block in `observation.rs` copies the normalize and error-mapping block above it (FND-003). |
| Test fidelity | The synthetic unbuilt-engine diagnostic omits FR-286's cause (FND-004). |
| Safety/concurrency/wire | No unsafe code, concurrency or wire-format change. The `ItemSelectionReadError` split is behaviour-preserving. |

## Coverage (manual AC-to-test check)

- FR-534-AC-1 (TC-1339): the test dropped two of the AC's clauses instead of flipping them (FND-005).
- FR-620-AC-3: the new `an_explicit_item_on_a_package_selection_matches_its_source` repeats a subset of `a_package_selection_analyzes_as_its_source` (FR-850-AC-1). Its trace names FR-620-AC-3, which is about invalid-request members (FND-006).
- FR-880-AC-8 (TC-2502): `a_nested_repeat_with_a_stepping_body_is_written` is unchanged and green on main. FND-001 turns it red.
- FR-383-AC-17: `a_protocol_requiring_a_refused_temporal_adds_no_second_refusal` is still red. The PR body says it ends in `InternalFault(UnresolvedDraft)`. Its new fixture also depends on FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `K::Repeat => false` withholds every repeat that steps through a nested repeat, contradicting FR-880-AC-8 and FR-383 and turning the FR-880-AC-8 test red | fsl-semantics/src/check/lowering/protocol.rs:302 |
| FND-002 | medium | Ten refreshed corpus cases keep their `flips_with` gap keys, so the pinned flip report still lists closed gaps as open | tests/it/source_corpus.rs:780-826 |
| FND-003 | low | Unselected-source normalization duplicates the selection loop's normalize and error-mapping block | fsl-semantics/src/model/observation.rs:1742-1767 |
| FND-004 | low | The synthetic unbuilt-engine diagnostic has no `backend-not-supplied` cause or `backend` member, which FR-286-AC-6 requires | fsl-command/tests/it/prepared_command.rs:852-857 |
| FND-005 | medium | FR-534-AC-1's float-leaf admission and multi-failure priority clauses were deleted from TC-1339's test instead of flipped | fsl-semantics/src/check/check/declared_type_tests.rs:142-200 |
| FND-006 | low | The new package-item test duplicates the FR-850-AC-1 test and is traced to the unrelated FR-620-AC-3 | fsl-analyze/tests/it/analyze_request_wire.rs:667-681 |

## Findings detail

- **FND-001.** FR-880 says "A `repeat` whose body steps through a nested `repeat` is written by the FSpec FR-464 'Nested `repeat` rule'". FR-880-AC-8 says "A `repeat` that steps only through a nested `repeat` compiles". FR-383 says "S4 therefore has no `not-yet-implemented` refusal for a checked protocol". FSpec FR-464 counts a nested repeat as a step when its exit is infeasible, its maximum is absent or at least 1, and its body steps. The PR's rule never counts a nested repeat as a step. The FR-880-AC-8 test at `fsl-package/src/emit/tests/protocol_controls.rs:1871` then gets `RepeatStepsThroughRepeat`. The PR changed the checker so that two stale refusal tests pass: `refinement_adverse.rs:89-103` and `protocol_requirements.rs:906`. Fix: restore `K::Repeat => child(0)`, or the full FR-464 three-condition rule. Delete the `RepeatStepsThroughRepeat` case from `a_refinement_over_an_unwritten_clause_refuses_naming_the_construct`, since no checked-protocol construct is left unwritten. Give FR-383-AC-17's test a temporal refusal that does not depend on a withheld protocol construct.
- **FND-002.** The QSL-2039 precedent is that a landed gap's expectation is updated and its `flips_with` removed. QSL-2006 is Done. G-024, G-026, G-022 and G-001 are settled by FR-478-AC-11 and FR-910. G-028 is settled for decimal, float and qty. The expectations still carry those keys, and `REQUIRED_FLIPS_BY_CASE`/`EXPECTED_FLIP_COUNTS` still pin them as open. Fix: set `flips_with` to `"none"` on each settled case and update both tables (drop G-001, G-022, G-024, G-026, QSL-2006, and G-028 5 to 2). `null-primary-bad` (G-009) stays pinned only if its gap is still open.
- **FND-003.** The new loop repeats about 20 lines of `NormalizeOutcome` handling. Fix: extract one `normalize_view(&DomainPackage, limits)` helper and call it from both loops.
- **FND-004.** FR-286-AC-6 describes the driver document as holding a diagnostic with code `unimplemented_capability`, "cause `backend-not-supplied` and `backend` naming the engine". Fix: pass `Some("backend-not-supplied")` as the cause and set the `backend` member to `aot`.
- **FND-005.** FR-534-AC-1 says "set, bag and ordered-set element types holding a float leaf are admitted … missing declarations retain … first-refusal priority, including a fixture with more than one possible semantic failure". The PR deleted the IEEE set/bag assertions and the competing two-failure fixture. It did not turn them into `Ok(())` assertions or add a multi-failure missing-declaration fixture. Fix: add the IEEE `Set`/`Bag` types to the `Ok(())` loop. Add a fixture with two missing declarations, for example `Bag<Set<Missing>>`, that asserts the outer layer's refusal and location.
- **FND-006.** The test is a strict subset of `a_package_selection_analyzes_as_its_source` at line 627. Fix: delete it.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-880-AC-8, path: "spec/functional/FR-880-write-every-checked-protocol-construct-at-s4.md", role: examined, excerpt: "`repeat Loop by Seller visible (got.ok) while { not got.ok }` over a body that binds `got` compiles with `test` `post`. A `repeat` that steps only through a nested `repeat` compiles. Both packages are admitted. A post-test `repeat` whose body has a cycle through no step is still refused at S3 by FR-218's progress rule."}
  - {id: FR-383-AC-17, path: "spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md", role: examined, excerpt: "A graph in which W's required temporal clause key, its model relationship key or an attempt's operation anchor key is bound to a key no emitted node carries refuses missing_declaration/missing-name at the construct that bound it ..."}
  - {id: FR-092-AC-18, path: "spec/functional/FR-092-key-type-parameter-and-declared-nodes.md", role: examined, excerpt: "An FR-383 construct not yet emitted refuses unsupported_construct/not-yet-implemented at that construct, names it and the ticket that writes it, and writes no clause node."}
  - {id: FR-910-AC-2, path: "spec/functional/FR-910-check-and-compile-a-unit-that-declares-only-aliases-or-dimensions.md", role: examined, excerpt: "A unit whose only declaration is `type I = Integer;` ... refuse at S3 check with code `missing_declaration` and cause `empty-package`, located at the unit's `language` header declaration"}
  - {id: FR-910-AC-1, path: "spec/functional/FR-910-check-and-compile-a-unit-that-declares-only-aliases-or-dimensions.md", role: examined, excerpt: "a unit of `dimension Length;` and `unit m : Length = rational(1, 1);`, each pass S3 check and compile through the spine"}
  - {id: FR-286-AC-6, path: "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md", role: examined, excerpt: "a driver's `OutcomeDocument::new(Execute, None, Unsupported)` for an unbuilt engine holds one diagnostic whose code serializes as `unimplemented_capability` with cause `backend-not-supplied` and `backend` naming the engine, with `category` unsupported, `last_stage` `null`, and exits 21."}
  - {id: FR-534-AC-1, path: "spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md", role: examined, excerpt: "set, bag and ordered-set element types holding a float leaf are admitted with members compared by bit identity (FSpec FR-149, FSpec FR-144), missing declarations retain their existing semantic cause, original source location and first-refusal priority, including a fixture with more than one possible semantic failure."}
  - {id: FR-323-AC-3, path: "spec/functional/FR-323-key-union-values-in-collections.md", role: examined, excerpt: "With `union F { Measured(Float64) }` (a float payload), `Set<F>`, `Bag<F>`, `OrderedSet<F>` and `Sequence<F>` are admitted"}
  - {id: FR-218-AC-8, path: "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md", role: examined, excerpt: "In a unit that declares `protocol Fill`, `protocol Fill` as an expression checks to a reference bound to `Fill`'s node key, typed with `Fill`'s semantic type."}
  - {id: FR-620-AC-3, path: "spec/functional/FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md", role: examined, excerpt: "An item carrying `request_index`, `extent`, `candidates`, `temporal.fairness` or an unknown member ... each refuse the whole request at the request stage with `invalid-request`, exit 20, naming the member, and run no item."}
  - {id: FR-128-AC-3, path: "spec/functional/FR-128-replay-a-model-counterexample.md", role: examined, excerpt: "Refusals settle no result: AC-1's lasso with its last step removed ...; a step `upd(a)` replaced by `attemptUpdate` with receiver `z`, outside the universe (not enabled) ..."}
  - {id: FR-062-AC-5, path: "spec/functional/FR-062-implement-checked-family-contract.md", role: examined, excerpt: "a test asserts that neither `check` nor the S4 v2 emitter (`fsl_package::emit_checked`) returns `Incomplete` across each stage's fixture set."}
  - {id: FR-375-AC-14, path: "filament-specification spec/functional/analysis/FR-375-declare-model-refinement.md", role: examined, excerpt: "two `history` rows give two refusals at their keywords naming `history-row`; `abstract Spec::Twice` one at `Twice` naming `protocol-abstract-side`; a non-empty resolved list one at `Spec` naming `refinement-internal-list`"}
  - {id: FSpec-FR-478-AC-11, path: "filament-specification spec/functional/type-model/FR-478-convert-text-to-a-number.md", role: examined, excerpt: "Over `s: String`, `convert<Integer>(s)` of `42` gives `42`"}
  - {id: FR-286-AC-1, path: "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md", role: context_only, excerpt: "The `check` outcome over `tests/fixtures/spine-compile.native` serializes to a document whose `format` is `filament-outcome/1`, `operation` `check`, `last_stage` S4, `category` success"}
  - {id: FSpec-FR-464-nested-repeat-rule, path: "filament-specification spec/objects/interfaces/FR-464-checked-package-protocol-node-bodies.md", role: context_only, excerpt: "Repeat-through-repeat is a supported capability: a `repeat` whose body progresses only through a nested `repeat` is admitted when the nested `repeat` necessarily steps, and refused otherwise."}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-880-AC-8
    related_ids: [FR-383-AC-17, FR-092-AC-18, FSpec-FR-464-nested-repeat-rule]
    path: fsl-semantics/src/check/lowering/protocol.rs
    lines: "302"
    excerpt: |-
      `repeat Loop by Seller visible (got.ok) while { not got.ok }` over a body that binds `got` compiles with `test` `post`. A `repeat` that steps only through a nested `repeat` compiles. Both packages are admitted.
    finding: "K::Repeat => false makes repeat_steps_through_repeat withhold every repeat that progresses through a nested repeat as RepeatStepsThroughRepeat. FR-880-AC-8 and FR-383 require such repeats to be written, with no not-yet-implemented refusal for a checked protocol, and the change turns protocol_controls.rs a_nested_repeat_with_a_stepping_body_is_written red. Restore the nested-repeat progress rule and delete the stale RepeatStepsThroughRepeat refusal cases."
  - fnd: FND-002
    severity: medium
    confidence: medium
    method: code-review
    check: test-intent
    artifact_id: FR-910-AC-2
    related_ids: [FSpec-FR-478-AC-11, FR-910-AC-1]
    path: tests/it/source_corpus.rs
    lines: "780-826"
    excerpt: |-
      A unit whose only declaration is `type I = Integer;` ... refuse at S3 check with code `missing_declaration` and cause `empty-package`
    finding: "The refreshed corpus expectations (conv-text-num, fn-param-named-set-2006, no-profile-unit, obj-src-decimal/float/qty, opt-some, opt-Some-result, qty-scalar-mult) now hold the settled outcomes, yet they keep their flips_with gap keys. REQUIRED_FLIPS_BY_CASE and EXPECTED_FLIP_COUNTS still pin those gaps as open, so the flip report is wrong. Set flips_with to none and update both tables."
  - fnd: FND-003
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: fsl-semantics/src/model/observation.rs
    related_ids: []
    path: fsl-semantics/src/model/observation.rs
    lines: "1742-1767"
    excerpt: |-
      for source in model_selections.sources {
          if !included_sources.insert(source.model_selection.identity.as_str()) {
              continue;
          }
          let view = match normalize(source, limits) {
    finding: "The unselected-source loop copies the selection loop's NormalizeOutcome handling and budget-refusal mapping. Extract one helper so the two mappings cannot drift."
  - fnd: FND-004
    severity: low
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: FR-286-AC-6
    related_ids: [FR-286-AC-1]
    path: fsl-command/tests/it/prepared_command.rs
    lines: "852-857"
    excerpt: |-
      a driver's `OutcomeDocument::new(Execute, None, Unsupported)` for an unbuilt engine holds one diagnostic whose code serializes as `unimplemented_capability` with cause `backend-not-supplied` and `backend` naming the engine, with `category` unsupported, `last_stage` `null`, and exits 21.
    finding: "The synthetic unbuilt-engine diagnostic passes cause None and no backend member, unlike FR-286-AC-6's specified document. Pass Some(\"backend-not-supplied\") and the aot backend."
  - fnd: FND-005
    severity: medium
    confidence: high
    method: code-review
    check: coverage
    artifact_id: FR-534-AC-1
    related_ids: [FR-323-AC-3]
    path: fsl-semantics/src/check/check/declared_type_tests.rs
    lines: "142-200"
    excerpt: |-
      set, bag and ordered-set element types holding a float leaf are admitted with members compared by bit identity (FSpec FR-149, FSpec FR-144), missing declarations retain their existing semantic cause, original source location and first-refusal priority, including a fixture with more than one possible semantic failure.
    finding: "The PR deleted the IEEE Set/Bag assertions and the competing two-failure fixture instead of turning them into admitted assertions and a multi-failure missing-declaration fixture. TC-1339's test no longer covers two clauses of its traced AC."
  - fnd: FND-006
    severity: low
    confidence: high
    method: code-review
    check: trace
    artifact_id: FR-620-AC-3
    related_ids: []
    path: fsl-analyze/tests/it/analyze_request_wire.rs
    lines: "667-681"
    excerpt: |-
      An item carrying `request_index`, `extent`, `candidates`, `temporal.fairness` or an unknown member, and a request carrying an unknown member, and a request whose `deadline_ms` is 9007199254740992 (2^53), each refuse the whole request at the request stage with `invalid-request`, exit 20, naming the member, and run no item.
    test_id: an_explicit_item_on_a_package_selection_matches_its_source
    claimed_ac: FR-620-AC-3
    correct_ac: FR-850-AC-1
    finding: "The new test is a strict subset of a_package_selection_analyzes_as_its_source (FR-850-AC-1) and is traced to FR-620-AC-3, which covers invalid-request members. Delete it."
bindings:
  - {test_id: an_explicit_item_on_a_package_selection_matches_its_source, ac_id: FR-620-AC-3, trace: wrong}
  - {test_id: a_nested_repeat_with_a_stepping_body_is_written, ac_id: FR-880-AC-8, trace: correct}
  - {test_id: a_function_reference_to_an_attempt_protocol_checks, ac_id: FR-218-AC-8, trace: correct}
  - {test_id: keyed_collections_of_an_ieee_bearing_union_are_admitted, ac_id: FR-323-AC-3, trace: correct}
  - {test_id: a_returned_counterexample_rejects_a_different_real_post_state_as_content_mismatch, ac_id: FR-128-AC-3, trace: correct}
```
