---
id: SR-202611210022442313144108428
title: "Code review — QSL-2678 PR #3229 red main batch 3"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3229 diff: Cargo.toml, src/population_fixtures.rs, fsl-replay/src/outcome.rs, fsl-semantics/src/check/refusal.rs, fsl-semantics/src/model/observation/document.rs, fsl-semantics/tests/it/child_composite.rs, fsl-eval/tests/it/machine_contexts.rs, fsl-eval/tests/it/model_simulation.rs, spec/functional/FR-531"
review_set: subset
evaluated_revision: "129e52875128677c7f8e615fa57aceaf430518f6"
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Ticket: QSL-2678. This review covers PR #3229 at head `129e5287` (merge base `287d78e7`), read from the diff. Context came from `git show` against `origin/pr-3229` and `origin/main` `f7b1d41b`. Nothing was built or run, as the brief required. There is no Python and no logic-bearing shell in the diff.

## Verdict

**FAIL — one fix round needed.** One high and three lows.

These items are spec-backed:

- **population-fixtures.** `fsl-eval` is `optional = true`, `default = []` does not include the feature, and `population-fixtures = ["dep:fsl-eval"]` is the only thing that enables it. Cargo.lock does not change, which is correct: the root package already lists `fsl-eval` because it is a dev-dependency, and `fsl-eval` does not depend on the root crate, so no cycle forms. `claim_names: base.claim_names()` fills the sidecar field that was missing.
- **Outcome bytes.** FR-286 "Canonical encoding and key order" says `OutcomeDocument::to_bytes` writes RFC 8785 through `fsl_foundation::canonical_json::encode`, for every `filament-outcome/1` document. The new literal is sorted at every depth (top level, item and `basis`), and the `items` array still lists request indices 0, 1, 2 in request order. The test still checks the request order that FR-286-AC-3 names, and its second assertion still checks that the bytes are stable.
- **Undeclared field cause.** FR-300 "Causes are attributed to StateModel" says a field the static type does not declare uses `ill_typed`/`type-mismatch` (FR-056-AC-9). FR-363 says no candidate gives "the refusal an undeclared record field and an undeclared object field both get (FR-300)". FR-359-AC-1 says the same. FR-138-AC-2 says `self.zzz` gives `ill_typed`/`type-mismatch`. The history-field exception (FR-138, FR-363-AC-2) is still `MissingName` at `state_model.rs:298`. `refinement_history.rs:937-939` already expects `IllTyped` and is red on main, so this remap fixes it. The remap was made without updating two corpus cases (FND-001).
- **FR-531 row.** The bare package's `evaluate_current_value_invariants` and `evaluate_value_invariant_selection` (`fsl-eval/src/value/expression/bare.rs:917-931`) return `unmet(self, SOURCE_DECLARATION)`. That is `unknown_required_feature`/`unsupported-feature` with reason `needs-source-declaration` at `CallAdmissionLocus::Package`, with no charge. The row describes this correctly, but its classification differs from the sibling row's (FND-004).
- **Float decoder test.** `fields["path"]` is the RFC 6901 pointer to the failing native member. For `missing-member` it is the containing float object, which follows FR-663 "the pointer to the member (to the object for a missing member)". `record.path` holds the snapshot path root and the typed steps, which follows FR-106 "path root" and AC-58.
- **Context and parameter pointers.** The roots `/contexts/N/fields/<name>` (where `N` is the `enumerate` index of the document's `contexts` array) and `/parameters/<name>` are the documents' own spelling. Both tests resolve the pointer in the JSON they supplied. The nested steps come from the same `refusal_steps`/`pointer_of` as a snapshot field. `record.path` is unchanged, so FR-106-AC-15/AC-21 ("the invocation binding as the path root") still hold.

## Scope and skill provenance

The code-review and rust-review lanes ran by reading the diff. `quoin gap-analysis` was not run because there was no worktree or build under the brief. A manual AC-to-test check over the changed tests was done instead and is recorded below. Item 3 called for a search: the base and main were searched for `UnknownField` and `missing-name` in field-read contexts across fsl-semantics, fsl-replay, fsl-command (including `tests/it/cli_contract/`), the root `tests/it` and `tests/corpus`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Two corpus cases added on 2026-11-19 still expect `missing_declaration`/`missing-name` for an undeclared field read. The remap makes them `ill_typed`/`type-mismatch`, which turns `corpus_temporal` and `corpus_operations` red | fsl-semantics/src/check/refusal.rs:3794,3888; tests/corpus/temporal/p-badfield.expected.json:1; tests/corpus/operations/inv-unknown-field-bad.expected.json:1 |
| FND-002 | low | After the remap, the closing `any(IllTyped, type-mismatch)` assertion is also satisfied by the two `UnknownField` refusals, so it no longer proves that `1 + "a"` kept its cause | fsl-semantics/tests/it/variant_fields.rs:993-1000 |
| FND-003 | low | The context-field pointer test is traced to FR-106-AC-15/AC-21. Those ACs cover record payloads and UUID/Timestamp kinds under a parameter or result root. Its missing, unknown and wrong-kind context-field cases are FR-106-AC-44's | fsl-eval/tests/it/machine_contexts.rs:1679 |
| FND-004 | low | The new FR-531 row calls the FR-652 forms "unmet parity" and says they refuse "as it does for `evaluate_value_invariants`". The row above lists that method as "required in a package form" (FR-940), so the inventory gives two different classifications for one behaviour | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:330 |

## Findings detail

- **FND-001.** `p-badfield.native` reads `r.nosuch` over `record Reading`. That read goes through `typing.rs:3018` `self.field`, which refuses with `CheckCause::UnknownField` (`check.rs:5299`). `inv-unknown-field-bad.native` reads `self.unknownField` in an invariant. That read goes through `state_model.rs:309`, which refuses with `StateModel(UnknownField)`, and the wrapper delegates `code()`/`cause()` to the inner cause. Both `.expected.json` files (#3061 and #3065, written after #2819) pin `"code":"missing_declaration","cause":"missing-name"`, and `source_corpus.rs` `mismatch` compares both members. QSL-2678 does not list either area as red on main. Not run, so confidence is medium. **Fix:** in this PR, set both expectations to `"code":"ill_typed","cause":"type-mismatch"`. FR-300, FR-363 and FR-138-AC-2 (`self.zzz`) justify the change.
- **FND-002.** **Fix:** assert that the arithmetic refusal itself carries `IllTyped`/`type-mismatch`. Find it by its cause (not `UnknownField`) or by its location, rather than with `any`.
- **FND-003.** **Fix:** retag to `#[trace("TC-1045", "FR-106-AC-44")]`, plus the FR-663 criterion the pointer belongs to.
- **FND-004.** **Fix:** either move the two FR-652 methods into the FR-940 "required in a package form" row next to `evaluate_value_invariants`, or drop the "as it does for `evaluate_value_invariants`" clause and state why the FR-652 forms owe no package form.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-300, path: "spec/functional/FR-300-check-model-forms-through-the-state-model-family.md", role: examined, excerpt: "An absent declaration or state-clause anchor uses `missing_declaration`/`missing-name`; a field absent from the effective static type of a dereferenced object uses `ill_typed`/`type-mismatch` (FR-056-AC-9), as does a `lookup` key whose static type does not conform."}
  - {id: FR-363-AC-2, path: "spec/functional/FR-363-type-navigation-over-a-relationship-end.md", role: examined, excerpt: "a name matching no candidate refuses `ill_typed`/`type-mismatch`, except that outside the declaring refinement's rows a name that is a history field name of any refinement of the unit refuses `missing_declaration`/`missing-name` (FR-138)."}
  - {id: FR-056-AC-9, path: "spec/functional/FR-056-admit-domain-package-model-declarations.md", role: examined, excerpt: "inherited field access checks (`deref(g).code` over a `Gadget` whose supertype `Widget` declares `code`), and `deref(g).nope` and `g = r` over an unrelated `Rock` refuse `ill_typed` at check."}
  - {id: FR-138-AC-2, path: "spec/functional/FR-138-check-and-compute-history-fields.md", role: examined, excerpt: "an invariant of another type `Other` reading its undeclared field `last` (`missing_declaration`/`missing-name`); an invariant reading an undeclared non-history field `zzz` (`ill_typed`/`type-mismatch`)."}
  - {id: FR-359-AC-1, path: "spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md", role: examined, excerpt: "A parent-typed read of `promised_by` refuses `ill_typed`/`type-mismatch` at check, naming the field, the cause the checker gives an undeclared record field today."}
  - {id: FR-286-AC-3, path: "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md", role: examined, excerpt: "An `analyze` outcome of three items serializes with three `items` entries in request order, each holding its terminal record and category. Serializing any outcome twice gives equal bytes."}
  - {id: FR-286-canonical-encoding, path: "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md", role: examined, excerpt: "The machine-output documents are encoded by one function, `fsl_foundation::canonical_json::encode`, which writes the RFC 8785 (JCS) canonical encoding through `quire-canonical`. The rule covers the `filament-outcome/1` document of every operation ... `OutcomeDocument::to_bytes` calls it"}
  - {id: FR-106-AC-15, path: "spec/functional/FR-106-admit-snapshots-and-invocations.md", role: examined, excerpt: "Each refusal names the first nested path reached in field order, including an index for a sequence element. The same cases in a parameter or result use the invocation binding as the path root."}
  - {id: FR-106-AC-21, path: "spec/functional/FR-106-admit-snapshots-and-invocations.md", role: examined, excerpt: "each refuse `invalid_runtime_input`/`wrong-value-kind` at the nested path (the sequence element with its index) ... The same cases in a parameter or result use the invocation binding as the path root."}
  - {id: FR-106-AC-44, path: "spec/functional/FR-106-admit-snapshots-and-invocations.md", role: examined, excerpt: "a field missing from `fields` (`missing-member` naming it); an undeclared field ... (`unknown-member` naming the first in document order); `placed` as `{\"integer\": \"1\"}` (`wrong-value-kind` at the context path `OrderLifecycle`, `placed`)."}
  - {id: FR-106-AC-58, path: "spec/functional/FR-106-admit-snapshots-and-invocations.md", role: examined, excerpt: "`gain` `bits` `\"3FF0000000000000\"` (uppercase) ... `invalid-value` naming `bits`; ... `bits` removed, `missing-member` naming `bits`; an added member `value`, `unknown-member` naming `value`; `kind` `\"Float64\"`, `invalid-value` naming `kind`"}
  - {id: FR-663, path: "spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md", role: examined, excerpt: "A snapshot or invocation admission refusal (FR-106 checks 1 to 11) SHALL carry the document locus of the first failing member: the snapshot's labels, the pointer to the member (to the object for a missing member), the object's population and key, and the member name."}
  - {id: FR-531-capability-inventory, path: "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "| `evaluate_current_value_invariants`, `evaluate_value_invariant_selection` (FR-652 ...) | unmet parity: the type invariants are read from the source declarations, which the bytes do not carry; the bare package refuses at the `Package` locus with `unknown_required_feature`/`unsupported-feature`, reason `needs-source-declaration`, and charges nothing, as it does for `evaluate_value_invariants` |"}
  - {id: FR-940-decision-selected-package, path: "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md", role: context_only, excerpt: "FR-531 states that the model-check evaluators (`evaluate_holds`, `evaluate_clause`, `evaluate_value_invariants`, `check_separation`, `evaluate_frame` and the others of its inventory's source-checked row) take a source-checked value that the bytes do not carry."}
  - {id: Cargo.toml, path: "Cargo.toml", role: examined, excerpt: "population-fixtures = [\"dep:fsl-eval\"] ... fsl-eval = { path = \"fsl-eval\", optional = true }"}
findings:
  - fnd: FND-001
    severity: high
    confidence: medium
    method: code-review
    check: code-bug
    artifact_id: FR-300
    related_ids: [FR-363-AC-2, FR-138-AC-2, FR-056-AC-9]
    path: fsl-semantics/src/check/refusal.rs
    lines: "3794,3888"
    excerpt: |-
      An absent declaration or state-clause anchor uses `missing_declaration`/`missing-name`; a field absent from the effective static type of a dereferenced object uses `ill_typed`/`type-mismatch` (FR-056-AC-9), as does a `lookup` key whose static type does not conform.
    finding: "The UnknownField remap is spec-correct, but tests/corpus/temporal/p-badfield.expected.json (r.nosuch over a record) and tests/corpus/operations/inv-unknown-field-bad.expected.json (self.unknownField in an invariant) still pin missing_declaration/missing-name. Both reads refuse UnknownField, so corpus_temporal and corpus_operations turn red. Update both expectations to ill_typed/type-mismatch in this PR."
  - fnd: FND-002
    severity: low
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: fsl-semantics/tests/it/variant_fields.rs
    related_ids: [FR-300]
    path: fsl-semantics/tests/it/variant_fields.rs
    lines: "993-1000"
    excerpt: |-
      refusals.iter().any(|refusal| {
          refusal.cause.code() == fsl_foundation::diagnostic::Code::IllTyped
              && refusal.cause.cause() == Some("type-mismatch")
      }),
      "1 + \"a\" remains ill_typed/type-mismatch: {refusals:?}"
    finding: "With UnknownField now ill_typed/type-mismatch, the any() is satisfied by the two field refusals, so it no longer proves 1 + \"a\" kept its cause. Assert on the arithmetic refusal itself."
  - fnd: FND-003
    severity: low
    confidence: high
    method: code-review
    check: trace
    artifact_id: FR-106-AC-15
    related_ids: [FR-106-AC-21, FR-106-AC-44, FR-663]
    path: fsl-eval/tests/it/machine_contexts.rs
    lines: "1679"
    excerpt: |-
      Each refusal names the first nested path reached in field order, including an index for a sequence element. The same cases in a parameter or result use the invocation binding as the path root.
    test_id: a_context_field_refusal_carries_a_pointer_that_resolves_in_the_snapshot
    claimed_ac: FR-106-AC-15
    correct_ac: FR-106-AC-44
    finding: "The context-field pointer test exercises a missing, unknown and wrong-kind context field (FR-106-AC-44 conditions) plus the FR-663 pointer, not AC-15/AC-21's record-payload and UUID/Timestamp cases under a parameter or result root. Retag to FR-106-AC-44 and the FR-663 criterion."
  - fnd: FND-004
    severity: low
    confidence: low
    method: code-review
    check: ambiguous
    artifact_id: FR-531-capability-inventory
    related_ids: [FR-940-decision-selected-package]
    path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
    lines: "330"
    excerpt: |-
      | `evaluate_current_value_invariants`, `evaluate_value_invariant_selection` (FR-652 ...) | unmet parity: the type invariants are read from the source declarations, which the bytes do not carry; ... as it does for `evaluate_value_invariants` |
    finding: "The row classes the FR-652 forms as unmet parity yet equates them with evaluate_value_invariants, which the row above classes as required in a package form (FR-940). Put them in one class, or drop the comparison and say why no package form is owed."
bindings:
  - {test_id: a_context_field_refusal_carries_a_pointer_that_resolves_in_the_snapshot, ac_id: FR-106-AC-15, trace: wrong}
  - {test_id: a_parameter_refusal_carries_a_pointer_that_resolves_in_the_invocation, ac_id: FR-106-AC-21, trace: correct}
  - {test_id: analyze_items_keep_request_order_and_bytes_are_stable, ac_id: FR-286-AC-3, trace: correct}
  - {test_id: a_parent_typed_read_of_a_child_field_is_refused, ac_id: FR-359-AC-1, trace: correct}
  - {test_id: float_snapshot_decoder_refuses_malformed_bits_and_wrong_slots, ac_id: FR-106-AC-58, trace: correct}
```
