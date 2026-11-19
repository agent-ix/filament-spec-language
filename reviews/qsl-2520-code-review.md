---
id: SR-2026111911313600000005
title: "QSL-2520 replay fixture repair code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2520; fsl-replay/src/result/wire/value.rs; fsl-replay/src/spine/call/tests.rs; fsl-replay/src/spine/clause/tests/temporal.rs"
review_set: subset
---

## Summary

Reviewed frozen eeb0b6f948fa1145734ea164a076f84bed4c1573 using origin/main...HEAD. Three test-only edits repair setup and an obsolete wire member assertion; no production code, pins, CI lanes, thresholds or warning suppression changed. Code review includes its Rust lane, repository conventions and the installed Rust implementation, testing and wire-boundary guidance. No applicable AssuranceProfile was found.

## Verdict

PASS for the scoped source review: no introduced defect or weakened oracle found. Execution remains a separate pending gate; this review claims no green result for the frozen head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence

The S3 EmptyPackage refusal checks semantic graph, concrete type-node and model-root inventories; aliases alone do not populate them (fsl-semantics/src/check/mod.rs:8241). StringHolder and PosHolder each add one owned concrete record through the real TypeEnvironment/check/link path, with a required field of precisely String or Int[1,9]. No checking bypass, mock, vendored schema or copied helper was introduced. The fixture record names have no existing duplicate in fsl-replay.

The String tests still compare exact canonical bytes for ASCII, Unicode, empty and UUID-looking strings, validate shape, decode through package keys and require the native String variant and exact payload. The Option leaf test requires the string tag and decoded String payload type. Adjacent unknown-member, lone-surrogate and Boolean-kind controls remain intact.

TC-786 retains real check_expression in Kernel mode, real package evaluation, SumOutOfDomain for empty and [9,9], exact sum-node location, mapped reason, exit 10, and completion 4 with exit 0. Pos remains the same alias and collection domain, bound and source expression.

The temporal fixture evaluates real modulo by zero at position 2; ItemCause::undefined_evaluation sets cause and leaves code absent, and serde omits code=None (fsl-replay/src/outcome.rs:399,722). The new exact cause.cause assertion is FR-286-AC-11's schema; get("code").is_none() rejects even an explicit null code. Result/category/position/counterexample/witness and insufficient-data assertions are retained. The fixture position 2 is derived from its supplied [0,1,2] trace rather than FR-283's separate position-1 example.

## Validation and limits

git diff --check passes. Dispatch-provided scoped rustfmt checks pass; broad workspace formatting and the 567 unchanged fsl-semantics Clippy diagnostics are baseline limitations, not introduced findings. No cargo command was started by this reviewer: the authorized seven-name command already occupies the worktree queue at /private/tmp/qsl2520-green-rebased.log, with no finished test result visible during review. The lead owns the final focused gate. Aggregate, workspace test and CI commands were intentionally excluded by Peter's direct override.

Code-review and Rust lane ran together. Scoped gap analysis is a separate artifact. Spec review/sub-analyses are inapplicable because no spec or plan changed. Full-repository audit and plan completion are outside the assigned fixture repair.

## Scope and bindings

The following records include clean units and context, with verbatim excerpts (long criteria excerpted to 600 characters). Bindings indicate correct scoped behavioral evidence, not complete verification of every clause of a broad AC.
{
  "clean": true,
  "scope": [
    {
      "id": "FR-056-AC-23",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "K2's native String value retains any Unicode scalar sequence exactly, including empty, ASCII, multibyte and UUID-looking text; equality is exact-sequence equality, its key is UTF-8 byte order, and Text normalization does not apply. The closed String object accepts exactly `kind: string` and `text`; an extra member and a lone surrogate refuse, and the typed result wire round trip preserves each value and its UTF-8 byte contribution to result sizing. Existing Text, UUID and Timestamp kind encodings remain unchanged. String gains no arithmetic, concatenation or general relational operator; its un"
    },
    {
      "id": "FR-100-AC-10",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "With `type Pos = Int[1, 9]` checked under `CheckMode::Kernel`, S6a evaluation of `sum<Pos>(x in q: x)` for an empty `q` of `Sequence<Int[1, 9]>[0, 2]` returns `Outcome::Undefined(Undefined::SumOutOfDomain)` located at the `sum` node, and the outcome mapping renders it `{\"kind\": \"undefined\", \"reason\": \"sum-out-of-domain\"}`, exit 10. The same `sum` for `q` holding `4` completes with `4`."
    },
    {
      "id": "FR-286-AC-10",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "Over FR-283-AC-1's clause run `always p` violated at position 0, `from_clause_run` gives an item with `result` `refuted`, category violation, `clause`, `position` 0, a `counterexample` with a `prefix` of three identities, an empty `loop` (`[]`, the finite spelling of FR-330 and FSpec FR-364), `trace_position` 0, a `decisive` beside the counterexample with `position` 2, no `quantifier` and no `index`, `element` `false` and the empty `value_path`, `cause` null and a `witness` member exactly when the run's `ClauseRunReport::witness` holds a record (FR-266 gives this temporal run the basis `closed"
    },
    {
      "id": "FR-286-AC-11",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "Over FR-283-AC-5's clause run, `from_clause_run` gives an item with `result` `refuted`, category violation, `position` 1, a `counterexample` with `trace_position` 1, cause `undefined-evaluation{where, cause}` with `cause` `division-by-zero`, no `witness` and no `undefined` label."
    },
    {
      "id": "FR-286-AC-12",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "Over FR-283-AC-11's clause run, `from_clause_run` gives an item with `result` `undefined`, category undefined, cause `insufficient-data`, `position` 0, `count` 1 and `min_count` 2, and neither `counterexample` nor `witness`; its document exits 10."
    },
    {
      "id": "TC-786",
      "path": "spec/test-cases/TC-786-run-s-exit-statuses-come-from-the-exit-function-with-undefined-at-10.md",
      "role": "examined",
      "excerpt": "Verify that every exit status FSL `command`'s run returns is FR-285's exit code of the outcome's category."
    },
    {
      "id": "FR-090",
      "path": "spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md",
      "role": "context_only",
      "excerpt": "ADR-011 S6a reference evaluation SHALL return"
    },
    {
      "id": "FR-267",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "context_only",
      "excerpt": "The library\nserializes `fsl_replay::spine::run_clause`'s `ClauseRunReport` (FR-109) as\nthe `/2` clause-run document; this requirement owns that producer."
    },
    {
      "id": "FR-330",
      "path": "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md",
      "role": "context_only",
      "excerpt": "`fsl_replay::spine::run_clause` (FR-109) SHALL accept a `Temporal` selection\nthat names one temporal clause, a supplied trace and the clause's `over`\nbinding."
    },
    {
      "id": "FR-331",
      "path": "spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md",
      "role": "context_only",
      "excerpt": "A counterexample over a\nmodel subject replays through `replay_model_trace` (FR-128); both share the\npayload type and its evaluation."
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "native_string_values_round_trip_in_the_closed_canonical_form",
      "ac_id": "FR-056-AC-23",
      "trace": "correct"
    },
    {
      "test_id": "native_string_type_uses_a_string_leaf_tag",
      "ac_id": "FR-056-AC-23",
      "trace": "correct"
    },
    {
      "test_id": "native_string_wire_refuses_extra_members_and_lone_surrogates",
      "ac_id": "FR-056-AC-23",
      "trace": "correct"
    },
    {
      "test_id": "existing_wire_kind_strings_remain_unchanged",
      "ac_id": "FR-056-AC-23",
      "trace": "correct"
    },
    {
      "test_id": "tc_786_sum_over_pos_is_sum_out_of_domain_exit_10_or_completes_exit_0",
      "ac_id": "FR-100-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "violated_undefined_and_insufficient_data_items_carry_their_record_members",
      "ac_id": "FR-286-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "violated_undefined_and_insufficient_data_items_carry_their_record_members",
      "ac_id": "FR-286-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "violated_undefined_and_insufficient_data_items_carry_their_record_members",
      "ac_id": "FR-286-AC-12",
      "trace": "correct"
    }
  ]
}
