---
id: SR-2026111911313600000006
title: "QSL-2520 scoped fixture acceptance and trace review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2520; three changed fsl-replay test regions and FR-056-AC-23, FR-100-AC-10, FR-286-AC-10/11/12"
review_set: subset
---

## Summary

Scoped acceptance-criteria/test binding review of frozen eeb0b6f948fa1145734ea164a076f84bed4c1573. This applies the dispatch's explicit fixture-repair scope; it is not the gap-analysis skill's full-repository assurance audit or a claim that every repository criterion is covered.

## Verdict

PASS within the assigned diff: no new trace gap, unspecified production behavior, stub or weakened oracle found. Focused execution is pending and remains the lead's gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

quire matrix --scope . --format json exited 0 with quire 0.52.0 / engine 0.62.2; output is /private/tmp/qsl2520-matrix.json. FR-056-AC-23, FR-100-AC-10 and FR-286-AC-10/11/12 all compute tagged, and the exact modified tests appear as binders. Manual source review confirms the tags still describe the test assertions and their real execution paths. This is tag evidence, not a qualified green execution record. The run emitted extensive symbol-id-collision and unrecognized-trace warnings in unrelated repository files; clean scope does not imply a clean global trace inventory. No strict/full-repository verdict is claimed.

String's required closed canonical schema remains kind/string plus text, exact Unicode payload, native String decoding, Option type-leaf String and unchanged negative controls. The required concrete declaration repairs package admission without weakening those oracles. The Pos record repairs the same admission precondition without altering sum domain, empty-seed failure, overflow-of-domain failure, location, mapped reason or completion oracle.

FR-286-AC-11 explicitly defines undefined-evaluation{where,cause}; the producer stores division-by-zero in cause, while code is an omitted None. The changed test requires that exact value and forbids the obsolete member. Adjacent FR-286-AC-10/12 result, category, counterexample, witness and insufficient-data assertions remain unchanged. FR-090 family-outcome, FR-267 producer ownership and FR-330/331 temporal/replay contracts were read as context; no production contract changed. Broad criteria have other controls beyond this test, and correct scoped bindings do not claim complete AC coverage.

Full-repository reverse-gap/stub audit and spec-review methods were not run: the brief explicitly requests a scoped fixture check, and no production, specification or plan changed. No separate optional semantic audit was invoked; the authorized manual binding/oracle check is recorded here.

## Scope and bindings
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
