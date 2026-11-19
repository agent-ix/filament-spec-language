---
id: SR-2026111911313600000006
title: "QSL-958 bounded lasso gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-958; fsl-analyze/tests/it/protocol_bounded_lasso.rs; fsl-eval/tests/it/temporal_trace.rs; FR-329-AC-8..10; FR-217-AC-7; FR-338-AC-6 context"
review_set: subset
---

## Summary

Manual changed-scope AC/code/test gap analysis, explicitly authorized by the lead under Peter's no-full-matrix instruction. Quoin is available (0.36.7); automated full matrix was deliberately not invoked. This is a bounded manual audit, not repository-wide assurance.

FR-329-AC-8 is retained and traced: real fork/join counterexample replay checks false at origin and true with a distance-0 witness; the documented position-keyed TemporalLetters seam checks a witness true only at a later loop position, because model-state atoms cannot differ on a fork/join loop. FR-329-AC-9 checks observed all-counted lasso refusal without atom reads and counted flag combinations including a closing counted step. FR-217-AC-7 is bound to both fair NoValue and unfair refusal tests over the same adversarial-generated counted payload. TC-839 steps 6-8 and TC-662 step 6 are accounted for. FR-329-AC-10 uses the FR-338-AC-6 Duplicate/Lose cycle, carrying genuine recompiled default fairness and using the same [1,1] bounded window for both scheduler packages.

The tests call production evaluator, model checker, request decoding, source recompilation, replay, fairness and settlement paths. No source or assertion stubs, unowned new behavior, unmatched scoped tags, or mocked unit under test were found. No plan was supplied.

## Verdict

PASS for the manual changed-scope gap check. This records only the examined criteria and does not assert whole-repository coverage or passing pending execution.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Frozen candidate: fc15bbce824f6589287d9595c0ba1674ce5b6a59. Ticket: QSL-958. PR: agent-ix/filament-spec-language#3033.

- FR-329-AC-8 (examined), spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md: Over a fair protocol-subject lasso (a `repeat` around a `parallel` whose loop is only `fork` and `join` steps), entered at interval distance 0, built through FR-217's replay, under a bounded profile: with `p` false at every position, `eventually[1,1] holds(p)` `on origin` is `Completed(false)` at position 0; with `p` true at position 0, `eventually[0,1] holds(p)` is `Completed(true)`; with `p` true only at a loop position, at distance 0, `eventually[0,1] holds(p)` is `Completed(true)`.
- FR-329-AC-9 (examined), spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md: Under a bounded profile, a `Lasso::Observed`, a `Lasso::Model` over a model subject, and a protocol-subject `Lasso::Model` whose loop has one counted step each refuse `unsupported_construct`/`expression-form` and return no truth value; FR-217-AC-7 states the replay of the last.
- FR-329-AC-10 (examined), spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md: Under a bounded profile, a zero-counted protocol-subject lasso that is unfair for a scheduler constraint refuses `invalid_runtime_input`/`invalid-value` naming the constraint and returns no truth value; the same lasso replayed against the package that declares `scheduling adversarial` is fair, and with `p` false at every position `eventually[1,1] holds(p)` is `Completed(false)` at position 0.
- FR-217-AC-7 (examined), spec/functional/FR-217-replay-a-protocol-counterexample.md: Under a bounded profile, a fair protocol lasso whose loop has one counted step replays to `inconclusive` with cause `NoValue`, and the same lasso made unfair for a scheduler constraint refuses `invalid_runtime_input`/`invalid-value` as unfair.
- FR-338-AC-6 (context_only), spec/functional/FR-338-check-an-en-1-closure-certificate.md: `eventually[0,1] p` on origin with p false then true advances to accept, false then false to reject; the final counted-distance region and terminal false-extension give the evaluator's same result; zero-distance protocol steps read letters without reducing debts, open-upper until/release retain IV-2 prefix semantics, and an ordinary open-upper `F/U[a,open]` gains its least-fixed-point acceptance set only after the lower shift completes; the non-progress lasso vectors distinguish starvation from actual counted progress without invented fairness, and on each retains an older unfulfilled activation after creating a new one; auxiliary exhaustion/cancellation returns typed Stopped with exact limit/value/setting/source and settles Incomplete rather than a certificate defect.

Plan completion: not assessed

## Validation

Touched-file rustfmt and diff check passed in the lead's evidence; reviewer independently ran git diff --check (exit 0). The lead-owned name-filtered fsl-analyze protocol_bounded_lasso:: integration tests are compiling; the exact evaluator refusal test is pending. No new build was started by the reviewer. All aggregate suites, full matrix, workspace Clippy, cargo-deny, and full cargo tests were omitted under Peter's explicit scope instruction. Execution is not yet claimed passing.

## Review data

Both SpecReview files passed explicit-path `quire validate --scope /Users/peter/dev/qsl-958-code` (exit 0, no stderr). Linear comment read-back passed: the JSON-form YAML was parsed both as JSON and through quire's YAML frontmatter parser; clean/findings, all five scope records, eight bindings, and placeholder exclusion agree. Existing QSL has-review label was added to the ticket.

Tooling note: `quire schema --module /Users/peter/.ix/filament/modules/spec-artifacts-process SpecReview` returned its schema with exit 0 but printed UnknownEdgeType/UnknownRole diagnostics for missing cross-module ADR/TestMatrix vocabulary. The executable resolves to /Users/peter/.nvm/versions/node/v24.15.0/bin/quire, version 0.52.0 (engine 0.62.2). Full scoped discovery validation above succeeds. No older binary or cached schema fallback was used.

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-329-AC-8",
      "path": "spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md",
      "role": "examined",
      "excerpt": "Over a fair protocol-subject lasso (a `repeat` around a `parallel` whose loop is only `fork` and `join` steps), entered at interval distance 0, built through FR-217's replay, under a bounded profile: with `p` false at every position, `eventually[1,1] holds(p)` `on origin` is `Completed(false)` at position 0; with `p` true at position 0, `eventually[0,1] holds(p)` is `Completed(true)`; with `p` true only at a loop position, at distance 0, `eventually[0,1] holds(p)` is `Completed(true)`."
    },
    {
      "id": "FR-329-AC-9",
      "path": "spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md",
      "role": "examined",
      "excerpt": "Under a bounded profile, a `Lasso::Observed`, a `Lasso::Model` over a model subject, and a protocol-subject `Lasso::Model` whose loop has one counted step each refuse `unsupported_construct`/`expression-form` and return no truth value; FR-217-AC-7 states the replay of the last."
    },
    {
      "id": "FR-329-AC-10",
      "path": "spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md",
      "role": "examined",
      "excerpt": "Under a bounded profile, a zero-counted protocol-subject lasso that is unfair for a scheduler constraint refuses `invalid_runtime_input`/`invalid-value` naming the constraint and returns no truth value; the same lasso replayed against the package that declares `scheduling adversarial` is fair, and with `p` false at every position `eventually[1,1] holds(p)` is `Completed(false)` at position 0."
    },
    {
      "id": "FR-217-AC-7",
      "path": "spec/functional/FR-217-replay-a-protocol-counterexample.md",
      "role": "examined",
      "excerpt": "Under a bounded profile, a fair protocol lasso whose loop has one counted step replays to `inconclusive` with cause `NoValue`, and the same lasso made unfair for a scheduler constraint refuses `invalid_runtime_input`/`invalid-value` as unfair."
    },
    {
      "id": "FR-338-AC-6",
      "path": "spec/functional/FR-338-check-an-en-1-closure-certificate.md",
      "role": "context_only",
      "excerpt": "`eventually[0,1] p` on origin with p false then true advances to accept, false then false to reject; the final counted-distance region and terminal false-extension give the evaluator's same result"
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "a_zero_counted_protocol_lasso_decides_intervals_in_counted_steps",
      "ac_id": "FR-329-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "a_bounded_clause_refuses_a_lasso_that_is_not_zero_counted",
      "ac_id": "FR-329-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "replaying_a_counted_protocol_lasso_under_a_bounded_clause_gives_no_value",
      "ac_id": "FR-329-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "replaying_a_counted_protocol_lasso_under_a_bounded_clause_gives_no_value",
      "ac_id": "FR-217-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "replaying_a_counted_protocol_lasso_refuses_an_unfair_scheduler_loop",
      "ac_id": "FR-329-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "replaying_a_counted_protocol_lasso_refuses_an_unfair_scheduler_loop",
      "ac_id": "FR-217-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "an_unfair_zero_counted_lasso_refuses_naming_the_scheduler_constraint",
      "ac_id": "FR-329-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "a_lasso_under_a_bounded_profile_is_refused",
      "ac_id": "FR-329-AC-9",
      "trace": "correct"
    }
  ]
}
```
