---
id: SR-1505
title: "QSL-2093 lifecycle boundary code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2093; tests/it/lifecycle_boundaries.rs; tests/it/main.rs; .agent/receipts/qsl-2093/scope.md; .agent/receipts/qsl-2093/validation.md"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-276
---

## Summary

Ticket: QSL-2093. Frozen source: 071837be306cdb8586ff4fae3959b36055289332. PR: filament-spec-language#3180.

Scope is tests/it/lifecycle_boundaries.rs, its tests/it/main.rs registration, and .agent/receipts/qsl-2093/{scope,validation}.md. No production changes. Seven active PASS receipts and the recorded ignored deadline failure are accepted from the author/lead without execution or log rechecking.

Code-review with the Rust lane: real public parser/checker/emitter and Cancel APIs, independently chosen byte/node boundaries, typed refusals, source locus, whole artifact equality, and cancellation outcome/charge bounds were examined. The helper retains its OnceLock owner, holds only a Weak reference in the observer, and observes the listener join. The ignored deadline observer rendezvous blocks charge 1,000 until the second thread records Deadline and acknowledges; dropping the trigger owner lets the listener finish if checking stops early. No clock or sleep supplies the oracle.

## Verdict

PASS for the delivered boundary subset. No new defect found. This does not close QSL-2093 or establish passing FR-276-AC-2 coverage. Final affected-component gate remains with the lead after review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed. No plan supplied. No matrix, build, lint, test, historical inventory or whole-repository audit was run. This is a diff-scoped review of concrete behavioral subsets, not an engine-backed full-criterion coverage verdict. FR-255-AC-1/3 and FR-277-AC-1 are broader than this slice; remaining fields/operations and historical certificate/result/plugin families remain unclaimed. The three FR-276 statement tags record the behavior they actually assert, not full-AC completion. FR-276-AC-2 is correctly tagged but ignored and explicitly excluded from passing coverage; QSL-2720 owns the known implementation defect and blocks QSL-2093.

All examined and context units, including clean portions:

```yaml
{
  "scope": [
    {
      "id": "FR-255-AC-1",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "For each row of the setting table, driving its stage or operation past that limit at a configured bound `B` returns the code Behavior 1's table gives for the limit's carrying type, with the row's limit kind, bound `B`, the count the refused charge would have reached, and the row's setting name",
      "portion": "S1 input bytes and E4 output bytes only"
    },
    {
      "id": "FR-255-AC-2",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "A function `f` whose body is `1 + 1 + 1` (five expression nodes), checked with `s3.nodes` at 4",
      "portion": "five-node boundary, typed fields, rendered setting and authored region"
    },
    {
      "id": "FR-255-AC-3",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "Setting a field through its builder method changes that field's bound and leaves every other field at its prior value.",
      "portion": "SpineLimits setters and PackageLimits refusal of unowned setting only"
    },
    {
      "id": "FR-277-AC-1",
      "path": "spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md",
      "role": "examined",
      "excerpt": "Setting the field to the counter reached succeeds.",
      "portion": "S1 input bytes, S3 nodes and E4 output bytes only"
    },
    {
      "id": "FR-276",
      "path": "spec/functional/FR-276-cancel-a-lifecycle-operation.md",
      "role": "examined",
      "excerpt": "When the handle is cancelled, each lifecycle operation shall stop at its next charge and return its cancelled failure with the handle's `CancelCause`.",
      "portion": "parse/check/package observed-charge cancellation and no stage value only"
    },
    {
      "id": "FR-276-AC-2",
      "path": "spec/functional/FR-276-cancel-a-lifecycle-operation.md",
      "role": "examined",
      "excerpt": "A `check` over a generated source of 200,000 declarations, cancelled with `Deadline` from a second thread once the test's work-meter observer has seen 1,000 charges, returns `StageFailure::Cancelled(Deadline)`, no `CheckedPackage`, and the observer records at most one charge after the cancellation.",
      "portion": "static review of strict ignored oracle; no passing coverage"
    },
    {
      "id": "FR-506",
      "path": "spec/functional/FR-506-bound-model-admission-by-named-spine-limits.md",
      "role": "context_only",
      "excerpt": "default 10,000; a `u32` that the caller raises or lowers and that is used as given, so its bound is `u32::MAX`"
    }
  ],
  "bindings": [
    {
      "test_id": "changing_one_compile_setting_preserves_other_bounds_and_emit_rejects_source_settings",
      "ac_id": "FR-255-AC-3",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "source_byte_ceiling_counts_utf8_bytes_and_accepts_the_exact_bound",
      "ac_id": "FR-255-AC-1",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "source_byte_ceiling_counts_utf8_bytes_and_accepts_the_exact_bound",
      "ac_id": "FR-277-AC-1",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "the_fifth_expression_node_names_its_s3_setting_and_source_locus",
      "ac_id": "FR-255-AC-2",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "the_fifth_expression_node_names_its_s3_setting_and_source_locus",
      "ac_id": "FR-277-AC-1",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "package_output_ceiling_is_inclusive_and_never_returns_a_truncated_artifact",
      "ac_id": "FR-255-AC-1",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "package_output_ceiling_is_inclusive_and_never_returns_a_truncated_artifact",
      "ac_id": "FR-277-AC-1",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "parse_cancellation_at_a_real_charge_never_returns_recovered_or_partial_forms",
      "ac_id": "FR-276",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "checking_cancellation_at_a_real_charge_never_returns_a_checked_unit",
      "ac_id": "FR-276",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "emission_cancellation_at_its_first_charge_produces_no_package",
      "ac_id": "FR-276",
      "trace": "correct",
      "extent": "subset"
    },
    {
      "test_id": "a_deadline_from_another_thread_stops_two_hundred_thousand_declarations_at_the_next_charge",
      "ac_id": "FR-276-AC-2",
      "trace": "correct",
      "extent": "subset",
      "ignored": true,
      "passing_coverage": false
    }
  ]
}
```

