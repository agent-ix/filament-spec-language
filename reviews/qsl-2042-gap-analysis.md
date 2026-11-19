---
id: SR-2026111911313600000010
title: "gap-analysis of QSL-2042 temporal source journeys"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3061; tests/it/temporal_package_route.rs; tests/corpus/temporal/**"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: references
---

## Summary

Ticket QSL-2042, PR 3061, reviewed head f74323ac74911db25fee5bfc84670dbe32149078. Diff-scoped gap analysis checks delivered behavior ownership and hollow-evidence risks; no fresh repository matrix was requested or computed. No defects found in the delivered slice.

## Verdict

**PASS for the reviewed diff.** This is a partial temporal corpus delivery, not a whole-ticket or repository assurance completion verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed the complete 30-file PR diff: the added source_journeys module in tests/it/temporal_package_route.rs and every new native source, expected record and compile request under tests/corpus/temporal/. Sources: event-window, fixed-window, operators, p-badbound, p-badclock, p-badfield, p-next, p-nonbool, p-noprofile, p-previous, p-unbounded-finite, p-weakuntil and timestamped-window. Each expected record was examined; the event/fixed/timestamped request documents were examined. The existing source_corpus runner and command_harness compile_file helper were context, not changed or requalified.

The five journey tests call real public command::run/compile entry points. They assert tested/refuted/pending pairs, lasso prefix/loop identities and decisive witness fields, missing-fairness and work-budget causes, exact empty-trace reason/pointer/code/cause and stage/category, unknown-observation refusal, and event-position closed-window outcomes. Rover cases read the authoritative examples root selected by FSL_EXAMPLES_DIR; a missing or wrong root fails. No sibling source/model/request files are copied. This root-reference design is explicitly authorized by the dispatch and user's no-copy instructions.

Expected values are independent test oracles. The loop-specific witness/cause assertions execute for explicit named table entries whose result and exit are asserted first. Fixture/request writes are confined to tempfile directories. Test-only panic/unwrap handling matches the existing harness style. No production, unsafe, concurrency, package boundary, manifest, CI or gate policy changed.

Trace bindings FR-330/328/329/327 name the exercised temporal families; FR-645-AC-1/2 name the empty-trace refusal obligations. These tests exercise portions of compound criteria and do not prove every locus/field/algorithm condition of those requirements. No whole-criterion, whole-ticket or repository-wide coverage claim is made. Fixed/timestamped programs currently establish compilation, not replay; remaining model-subject window aggregates, timed-model trace refusal, timestamped/fixed replay and monitor/analyze routes remain open on QSL-2042.

Reported coder evidence accepted without repetition: 13-program source_corpus::corpus_temporal passed; five source_journeys tests passed across the corrected checked-handoff runs, including 22 authoritative rover requests, with zero ignored. The earlier zero-selected default run is not passing evidence. The corrected feature-enabled run passed four tests, then the changed unknown-observation fixture passed its single focused test. No tests, builds, computed matrix, gate or peer measurements were repeated in this review. The lead owns the one final gate.

No AssuranceProfile is selected for this diff. Repository assurance context from the lead's lane is accepted; no new profile inventory or repository-wide audit was performed.

Plan completion: not assessed

## Examined Scope

```json
{
  "scope": [
    {
      "id": "source_journeys",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "mod source_journeys {"
    },
    {
      "id": "rover_examples_match_baseline",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "fn rover_examples_match_baseline() {"
    },
    {
      "id": "bounded_nested_and_derived_next_have_real_trace_verdicts",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "fn bounded_nested_and_derived_next_have_real_trace_verdicts() {"
    },
    {
      "id": "empty_corpus_traces_refuse_with_the_precise_reason",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "fn empty_corpus_traces_refuse_with_the_precise_reason() {"
    },
    {
      "id": "an_unknown_observation_key_is_not_an_empty_or_false_trace",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "fn an_unknown_observation_key_is_not_an_empty_or_false_trace() {"
    },
    {
      "id": "authored_event_clock_compiles_and_settles_the_closed_window",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "fn authored_event_clock_compiles_and_settles_the_closed_window() {"
    },
    {
      "id": "FR-645-AC-2",
      "path": "spec/functional/FR-645-refuse-a-malformed-or-mismatched-trace-with-a-reason-message-and-locus.md",
      "role": "context_only",
      "excerpt": "A `finite` array of zero keys reports `reason` `empty-finite-trace`, `request_pointer` `selection.temporal.trace.finite`, a message and the clause locus."
    },
    {
      "id": "operators",
      "path": "tests/corpus/temporal/operators.native",
      "role": "examined",
      "excerpt": "temporal BoundedUntil using t over (r: Reading) clock \"steps\" on origin {"
    },
    {
      "id": "event-window",
      "path": "tests/corpus/temporal/event-window.native",
      "role": "examined",
      "excerpt": "temporal EventWindow using t over (r: Reading) clock \"events\" on origin {"
    }
  ]
}
```

