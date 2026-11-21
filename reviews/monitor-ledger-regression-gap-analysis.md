---
id: SR-967175026302197768900474181941935084686
title: "Timestamped provisional ledger regression gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; monitor-ledger-regression; ClauseState provisional evaluation and named monitor_incremental tests"
review_set: subset
---

## Summary

FAIL for the frozen correction: actual report-ledger loss and bypassed explicit completeness validation remain. Review of the frozen two-file delta219b4573f7..25895d7615162d0c017f34ea3e7335d357ef4c25. The parent requested one bounded code/Rust and scoped semantic gap pass for the three named monitor regressions, preserving strict watermark rules, Progressed behavior and the existing QSL2750 twelve-field report parity. No full gates, matrix, measurement, runtime, source edit, pin edit or Linear operation was authorized or performed by the reviewer. Parent owns tests and merge. Plan completion: not assessed

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The strengthened changed-absence test checks a genuine explicit assertion only after all delivered positions are covered. It does not cover the same explicit G3 assertion with a later uncovered stamp5, which selects provisional_advance and loses validation. Add that boundary to the existing before/after-checkpoint control; the unchanged QSL2750 full-report test already owns the no-held-ledger case. | fsl-analyze/tests/it/monitor_incremental.rs:4000-4052; FR-993-AC-1 |

## Inspected boundaries

The provisional helper is used after an observation, when restoring an uncovered checkpoint, and from evaluate(Some(progress)) during assert_progress. mark/restore already brackets temporary ordering, canonical tail, tape, retained values, windows, admissions, evaluated cursor, progress and ledger on both success and failure. Cached traces truncate to the committed prefix after restoration. Taking the input ledger inside that transaction is restored correctly as state; the findings concern what validation is skipped and what report is returned, not a missing outer rollback.

The intended opening-ledger observation path must not treat implicit G0 as a new explicit watermark or compare partial redelivery against a complete covered record. In contrast an actual explicit assertion must retain its strict watermark/fingerprint checks. The production call origin, not merely Option<ProgressAssertion>, distinguishes these: an observation may already carry Some(latest assertion). Checkpoint regeneration must follow observational semantics without inventing progress. Progressed and other trace kinds must keep their existing evaluate branch. MonitorSession's separate regression check does not replace ClauseState's covered-content validation.

Existing a_regressing_stamp_is_refused_and_the_ledger_equals_the_batch_run and the_opening_ledger_orders_stamps_and_assertions require inclusive stamp refusal and retained opening progress. The changed-absence test is a real production ClauseState seam and retains its explicit Completeness contradiction; its new report/checkpoint ledger assertions are useful but do not cover the later-uncovered-tail branch. The existing qsl_2750_missing_window_matches_batch_and_checkpoint compares all twelve nonusage report fields with independently authenticated batch input and already detects the fresh-ledger loss. It must not be weakened.

## Scope, assurance and validation

The changed behavior is owned by FR643/820 report parity, FR641/502 progress-ledger semantics and FR993 covered-content integrity. No unowned new public API, second checker, test-only production bypass, unsafe path, external service or CI/gate weakening is introduced. Rust API/implementation/safety/wire/testing conventions and repository AGENTS apply. The two-file diff changes no Cargo manifest, dependency pin, lockfile or source license. No applicable AssuranceProfile is asserted; this bounded review is not a whole-repository reverse audit.

Parent queued the original three runtime tests on Titan at25895; no result or compilation PASS is inferred here. No reviewer Cargo/runtime command ran. Artifact validation uses installed quire and the installed SpecReview contract. Findings are derived from the concrete call graph and existing behavioral oracles; no matrix result, measurement or broader assurance PASS is claimed. No Linear comments are posted per the parent's explicit instruction.

## Review data

```yaml
clean: false
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-993-AC-1
  related_ids:
  - FR-820
  path: fsl-analyze/tests/it/monitor_incremental.rs
  lines: 4000-4052
  excerpt: 'Given the ledger a batch `Progressed` request over record-sample levels
    1, 2 with `Complete { through: 1 }` returns, a later batch request over levels
    1, 5, 3 with `Complete { through: 2 }` is refused `Completeness` and returns the
    input ledger unchanged, and one over levels 1, 2, 3 is accepted. A carried state
    opened on the same ledger (`ClauseOpen.ledger`) and advanced over the same positions,
    and the same state written by `to_bytes` after the first position and read by
    `from_bytes`, gives the same two verdicts and equal ledgers.'
  finding: The strengthened changed-absence test checks a genuine explicit assertion
    only after all delivered positions are covered. It does not cover the same explicit
    G3 assertion with a later uncovered stamp5, which selects provisional_advance
    and loses validation. Add that boundary to the existing before/after-checkpoint
    control; the unchanged QSL2750 full-report test already owns the no-held-ledger
    case.
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round1 at3f6c967f96100391b72da02c1eee74359cf1ac6e: The original changed-absence test now executes all four restored/uncovered combinations. Both explicit covered-prefix and stamp5-tail G3 assertions must refuse Completeness, and state bytes before/after refusal must be equal. |

## Fix round 1 verdict

CLEAR for the original bounded static findings at3f6c967f96100391b72da02c1eee74359cf1ac6e. Original findings and FAIL verdict above are retained as the original review; all of their current dispositions are fixed. No new defect identified in this narrow correction. Final acceptance remains CONDITIONAL pending the parent's exact-head four runtime tests and owned merge checks.

The bounded delta25895d7615..3f6c967f96 adds an explicit call-origin boolean to the existing private evaluate/provisional path. advance passes false, assert_progress passes true, and from_bytes provisional regeneration passes false. A true call keeps its input ledger available to the existing run and does not replace the resulting output ledger. A preview may temporarily take a held ledger to avoid treating partial redelivery/default G0 as a new assertion; it restores a present held ledger to the report, while a computed fresh ledger survives when input was None. The existing mark/restore always runs before propagating the result, with cached listings truncated to the committed prefix. No strict watermark or fingerprint condition in the underlying evaluator is weakened. Non-Timestamped evaluate branches, including Progressed, are unchanged.

The original real changed-absence control still requires Completeness rather than a fabricated success. It now covers both before/after checkpoint restoration and both fully covered/extra uncovered stamp5 delivery, and checks exact checkpoint-byte rollback after each refusal. Report ledger assertions before and after restoration remain. The original QSL2750 twelve-field batch oracle and its ordering/absence/late-input assertions are untouched and are included in the parent's corrected runtime selection. The correction copies the original reviewer artifacts without rewriting their findings.

Reviewer ran no Cargo, runtime, matrix, metric, broad gate, source edit, pin edit or Linear operation. git diff --check passes. No final corrected-head runtime or merge-gate result has been received in this artifact yet; the old25895 selection is historical only. This is not a whole-repository assurance PASS or a new full review.

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "        if uncovered {\n            state\n                .advance(admitted(&world,\
    \ \"Twos\", &text(1), 5), &cancel)\n                .expect(\"an uncovered observation\
    \ asserts no progress\");\n        }\n        let before = state.to_bytes().expect(\"\
    writes before the assertion\");\n        let refused = state.assert_progress(&progress(3),\
    \ &cancel);"
  reason: The original changed-absence test now executes all four restored/uncovered
    combinations. Both explicit covered-prefix and stamp5-tail G3 assertions must
    refuse Completeness, and state bytes before/after refusal must be equal.
```
