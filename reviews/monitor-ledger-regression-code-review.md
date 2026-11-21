---
id: SR-967175026302197768900474181941935084685
title: "Timestamped provisional ledger regression code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; monitor-ledger-regression; ClauseState provisional evaluation and named monitor_incremental tests"
review_set: subset
---

## Summary

FAIL for the frozen correction: actual report-ledger loss and bypassed explicit completeness validation remain. Review of the frozen two-file delta219b4573f7..25895d7615162d0c017f34ea3e7335d357ef4c25. The parent requested one bounded code/Rust and scoped semantic gap pass for the three named monitor regressions, preserving strict watermark rules, Progressed behavior and the existing QSL2750 twelve-field report parity. No full gates, matrix, measurement, runtime, source edit, pin edit or Linear operation was authorized or performed by the reviewer. Parent owns tests and merge. Plan completion: not assessed

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unconditional replacement erases the fresh batch ledger when the incoming ledger is None. temporal.rs2523–2587 creates an accepted fresh ledger for progress traces with source identity, so the unchanged twelve-field QSL2750 oracle now observes None instead of batch Some on its first uncovered prefix. Preserve the computed fresh output in the no-held-ledger case. | fsl-replay/src/spine/clause/clause_state.rs:1377; FR-643-AC-13 |
| FND-002 | high | The helper also handles a real assert_progress when the asserted watermark is below the newest observation. Removing its ledger suppresses required covered-content/completeness validation, then assert_progress stores the assertion as accepted. Use the existing heldW3 changed-absence fixture, redeliver indices0..3 without the held absence at1, append an uncovered stamp5, then explicitly assert G3: evaluable is false and this path drops the evidence needed for the required Completeness contradiction. Distinguish observation/readback projection from actual explicit assertion. | fsl-replay/src/spine/clause/clause_state.rs:1369,1398; assert_progress1097; FR-993-AC-1 |

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
  method: code-review
  check: code-bug
  artifact_id: FR-643-AC-13
  related_ids:
  - FR-820
  - FR-641
  path: fsl-replay/src/spine/clause/clause_state.rs
  lines: '1377'
  excerpt: 'Feed FR-194-AC-17''s timestamped observations one at a time, marking the
    aggregate atom absent with `{position: 18, atom: A}` on observation 18, then assert
    its closed progress complete through 300000. Each pending report equals the corresponding
    batch prefix with the same `missing` entries; the final report is `indeterminate`
    with `MissingValuation { position: 18, atom: A }`, never `invalid-value` or a
    quantile over the other values. A checkpoint immediately after observation 18,
    resumed before observation 19 and the progress assertion, gives the same reports
    and ledger as the uninte'
  finding: "The unconditional replacement erases the fresh batch ledger when the incoming\
    \ ledger is None. temporal.rs2523\u20132587 creates an accepted fresh ledger for\
    \ progress traces with source identity, so the unchanged twelve-field QSL2750\
    \ oracle now observes None instead of batch Some on its first uncovered prefix.\
    \ Preserve the computed fresh output in the no-held-ledger case."
- fnd: FND-002
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-993-AC-1
  related_ids:
  - FR-820
  - FR-643
  path: fsl-replay/src/spine/clause/clause_state.rs
  lines: 1369,1398; assert_progress1097
  excerpt: 'Given the ledger a batch `Progressed` request over record-sample levels
    1, 2 with `Complete { through: 1 }` returns, a later batch request over levels
    1, 5, 3 with `Complete { through: 2 }` is refused `Completeness` and returns the
    input ledger unchanged, and one over levels 1, 2, 3 is accepted. A carried state
    opened on the same ledger (`ClauseOpen.ledger`) and advanced over the same positions,
    and the same state written by `to_bytes` after the first position and read by
    `from_bytes`, gives the same two verdicts and equal ledgers.'
  finding: 'The helper also handles a real assert_progress when the asserted watermark
    is below the newest observation. Removing its ledger suppresses required covered-content/completeness
    validation, then assert_progress stores the assertion as accepted. Use the existing
    heldW3 changed-absence fixture, redeliver indices0..3 without the held absence
    at1, append an uncovered stamp5, then explicitly assert G3: evaluable is false
    and this path drops the evidence needed for the required Completeness contradiction.
    Distinguish observation/readback projection from actual explicit assertion.'
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round1 at3f6c967f96100391b72da02c1eee74359cf1ac6e: At3f6c967f96 only a present preview input ledger replaces report.ledger; when no ledger was held, the fresh output computed by the unchanged batch run survives. The existing twelve-field QSL2750 oracle remains unchanged. |
| FND-002 | fixed | Round1 at3f6c967f96100391b72da02c1eee74359cf1ac6e: Actual assert_progress calls evaluate with asserting_progress=true, retaining self.ledger for real watermark/fingerprint/completeness admission even when newest exceeds the assertion. advance and from_bytes explicitly pass false. Outer mark/restore still restores all state on refusal/cancellation. |

## Fix round 1 verdict

CLEAR for the original bounded static findings at3f6c967f96100391b72da02c1eee74359cf1ac6e. Original findings and FAIL verdict above are retained as the original review; all of their current dispositions are fixed. No new defect identified in this narrow correction. Final acceptance remains CONDITIONAL pending the parent's exact-head four runtime tests and owned merge checks.

The bounded delta25895d7615..3f6c967f96 adds an explicit call-origin boolean to the existing private evaluate/provisional path. advance passes false, assert_progress passes true, and from_bytes provisional regeneration passes false. A true call keeps its input ledger available to the existing run and does not replace the resulting output ledger. A preview may temporarily take a held ledger to avoid treating partial redelivery/default G0 as a new assertion; it restores a present held ledger to the report, while a computed fresh ledger survives when input was None. The existing mark/restore always runs before propagating the result, with cached listings truncated to the committed prefix. No strict watermark or fingerprint condition in the underlying evaluator is weakened. Non-Timestamped evaluate branches, including Progressed, are unchanged.

The original real changed-absence control still requires Completeness rather than a fabricated success. It now covers both before/after checkpoint restoration and both fully covered/extra uncovered stamp5 delivery, and checks exact checkpoint-byte rollback after each refusal. Report ledger assertions before and after restoration remain. The original QSL2750 twelve-field batch oracle and its ordering/absence/late-input assertions are untouched and are included in the parent's corrected runtime selection. The correction copies the original reviewer artifacts without rewriting their findings.

Reviewer ran no Cargo, runtime, matrix, metric, broad gate, source edit, pin edit or Linear operation. git diff --check passes. No final corrected-head runtime or merge-gate result has been received in this artifact yet; the old25895 selection is historical only. This is not a whole-repository assurance PASS or a new full review.

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "                if ledger.is_some() {\n                    report.ledger\
    \ = ledger;\n                }"
  reason: At3f6c967f96 only a present preview input ledger replaces report.ledger;
    when no ledger was held, the fresh output computed by the unchanged batch run
    survives. The existing twelve-field QSL2750 oracle remains unchanged.
- fnd: FND-002
  outcome: fixed
  after_excerpt: "                let ledger = if asserting_progress {\n         \
    \           None\n                } else {\n                    self.ledger.take()\n\
    \                };"
  reason: Actual assert_progress calls evaluate with asserting_progress=true, retaining
    self.ledger for real watermark/fingerprint/completeness admission even when newest
    exceeds the assertion. advance and from_bytes explicitly pass false. Outer mark/restore
    still restores all state on refusal/cancellation.
```
