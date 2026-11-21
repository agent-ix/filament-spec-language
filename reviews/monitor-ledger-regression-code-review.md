---
id: SR-967175026302197768900474181941935084685
title: "Timestamped provisional ledger regression code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; monitor-ledger-regression; ClauseState provisional evaluation, settled session ledger continuation/checkpoint, and named monitor_incremental tests"
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
| FND-003 | still-open | Checkpoint correction rejects direct malformed cases but still rejects a valid partial-watermark/out-of-order continuation because native ledger coverage includes an old uncovered canonical identity; add G5, observe7, G5, observe6, G7 and preserve its valid checkpoint. |
| FND-003 | fixed | Final confined correction at7d808d2c7bc6674b2c86a79e59ec9be05ad9366b: Required/forbidden tracker placement, real ledger/deciding-state prefix validation and logical sample-index counting now reject fabricated metadata while accepting the legitimate partial-watermark reordered tail; historical ledger rows remain intact. |

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

## Architecture-directed ledger continuation

The parent reports the corrected3f6 exact four-test run as1PASS/3FAIL and obtained an architecture ruling: preserve FR643's first decisive settlement and frozen full B(s), including its report ledger, while advancing the separate session ledger on real assertions after settlement. The frozen-record oracle is aligned to FR643 Batch equality lines195–212 and Finality/Settling lines301–329. No report field is removed from the twelve-field comparison, and delaying a genuinely decisive verdict is not the correction.

The bounded new-seam delta147a118395d7..3aea770bd07616aeb0e46a21e85eaaaf6fc7ad3c adds ClauseProgress, holding Seen position identities/stamps/order keys/absent atom IDs independently of the dropped evaluator state. It uses the existing canonical_order, sample fingerprint/index and admit_progress, not a second atom/clause evaluator. A real assertion validates and updates the shared session ledger; a frozen record and its evaluation usage stay unchanged. The transaction clones the tracker with the prior slot, checks cancellation before committing a new ledger, and restores it on failure. Position count is bounded by native covered-position capacity. Covered metadata is dropped after admission, with first retaining its global index offset. This path introduces no source arena, new source lookup, semantic value evaluation or charge to a settled clause.

The changed existing regression compares the decisive full report to B(s) under G2 and independently compares the session ledger to the later batch under G7. It checkpoints after settlement before real G5 and then feeds the future stamp7. The original QSL2750 twelve-field oracle remains. The provisional preview now preserves a present held report ledger only with no asserted progress; actual assertion contexts keep the computed batch report ledger. The first-pass explicit-call-origin guard and fresh-None output correction remain in place. These normal paths were inspected without a new full review.

Runtime and compilation at3aea remain pending from the parent at this writing. The new checkpoint seam below blocks static clearance. No Cargo, runtime, matrix, measurement or Linear operation was performed by the reviewer.

## New findings (architecture-directed fix2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | A malformed settled-clause checkpoint can disable or falsify future ledger tracking while resume accepts it. Null/omitted progress_state becomes None even for a settled Timestamped/Progressed slot; alternatively first=seen, positions=[], fixed=0 passes the tracker count check and discards an uncovered observation. Future real progress silently omits its fingerprint. A forged fixed=len can also bypass tail sorting. Require tracker presence/placement and retained-prefix consistency with the actual session ledger/deciding state, not only internal wire counts. | fsl-analyze/src/monitor_session.rs:1447; fsl-replay/src/spine/clause/clause_state/progress.rs:174; FR-643-AC-4 |

### FND-003 reproduction and correction

Use the new valid post-settlement checkpoint with ledger W2, total seen3 and an uncovered stamp5. Replacing only progress_state with null (or omitting it) resumes successfully: slot.state is None and slot.progress is None, so every later assertion skips that clause's progress admission. Separately retain the string field but set tracker first=3, positions=[], fixed=0; first+len==seen still holds, but real G5 updates the session watermark without the stamp5 content entry. The local Wire shape is closed, but those state-consistency failures are not rejected. Required settled progress metadata must be present or reconstructed from validated existing data; pending/nonprogress placements must also be rejected. Validate any dropped-prefix/canonical cursor against actual held progress, and add negative checkpoint controls to the same regression. This is an invariant failure in the newly introduced persistence seam, not an extra whole-repository audit.

Current bounded verdict: FAIL until FND-003 is dispositioned, with exact-head runtime still conditional. Original FND-001/FND-002 remain statically fixed and their text/dispositions stay unchanged.

## Confined checkpoint correction disposition

Inspected c0bb7692188506438ce57694eef6e2655909236d, with the parent-reported runtime target0f4d8940e6997b64c2df9635b55bf68f6da25f09 differing only by an Option<MonitorState> test annotation fixing E0282. No additional full review was started. Required/forbidden tracker placement now rejects null/omitted tracker on a settled progress slot and rejects a tracker on a pending/nonprogress slot. The five direct mutation controls address the previously described malformed shapes. Deciding metadata and covered prefix are cross-checked with the real session ledger and opening history; the canonical prefix is retained instead of incorrectly re-sorting indexed10 lexically before2.

The combined checkpoint finding remains still-open for a valid-state false rejection in the new prefix-count check. Native ProgressLedger::admit_continued extends its covered set with ALL supplied positions (ledger.rs310), even positions stamped after the asserted watermark. Concrete legal continuation: after settlement and real G5, tracker first=3; observe stamp7 and explicitly assert G5 again; tracker retains the uncovered position7, but ledger gains canonical index3@7. Observe stamp6 (legal because6>held watermark5), then assert G7. The pending tail sorts into index3@6 and4@7, while the native ledger still retains old3@7 as well. Tracker first and session seen are5, but count(ledger.covered with stamp<=7) is6. The checkpoint this valid session writes is therefore rejected by validate_checkpoint's new first/count equality. The accumulated ledger also retains the stale canonical identity relative to a fresh batch over the current prefix.

The correction must preserve legitimate partial-watermark/out-of-order-tail continuation while rejecting fabricated dropped prefixes; add that concrete control to the same regression. Null/placement tests alone do not establish this. This is the original first/fixed/canonical consistency boundary, not a third full review or a new baseline audit. Original first-pass findings remain fixed. Corrected-head runtime/compile results are pending; no result is inferred from the unconfirmed c0 runtime request or the failed test annotation compile. Reviewer ran no Cargo/runtime/Linear/matrix/measurement and made no source edit.

Current bounded verdict: FAIL / runtime conditional until the still-open checkpoint/canonical case is resolved.

## Final confined logical-count disposition

Inspected only the original checkpoint/canonical-count boundary at7d808d2c7bc6674b2c86a79e59ec9be05ad9366b, delta0f4d8940e6..7d808d2c7b. The c0 checkpoint consistency and placement checks remain, with the 0f4 change only annotating Option<MonitorState> in the test. The new correction counts logical covered positions by native identity: a record-sample row carrying its native fingerprint uses (identity,None), while a snapshot row keeps (identity,Some(stamp)). It does not parse arbitrary identity text, drop historical rows, change native admission, or replace identity/fingerprint/deciding-seed/capacity proofs. The original forged first=seen with an empty tracker still fails under W2's genuinely covered prefix, and forged fixed/placement controls remain.

Normative correction to my earlier analysis, preserved above: FR502 explicitly retains omitted covered facts and requires equal ledgers for the SAME REQUEST SEQUENCE. Therefore historical index3@7 from the prior partial G5 is legitimate retained history; six rows for this sequence are not themselves a native-ledger defect. The earlier fresh final-prefix/None-ledger comparison was an incorrect SESSION-ledger oracle. This correction leaves that original review text unchanged but withdraws that inference. The actual defect was treating the number of historical rows as the number of logical positions for checkpoint validation. FR643's frozen full B(s) report remains independent and still compares all twelve nonusage fields against its deciding batch prefix.

The existing regression now executes observations/assertions0/G0,2/G2,5/G5,7/G5,6/G7. It obtains the actual earlier partial G5 ledger through a typed checkpoint/resume/finish, supplies that ledger to the later batch request, and requires exact session/batch ledger equality for that history. It separately asserts all six historical rows remain across five observations, and successfully resumes the final valid checkpoint. The five negative checkpoint mutation controls, earlier post-settlement restore, strict watermark/completeness checks and frozen full-report oracle remain. This is meaningful positive and negative evidence for the same persistence finding, not a third full review.

Current bounded static verdict: CLEAR; every code/gap finding's latest disposition is fixed. Parent/author reported focused compile PASS at0f4; compilation and exact-head runtime/gate checks at7d808d2c7b remain pending from the parent. Runtime acceptance remains CONDITIONAL until those results. No new runtime test, Cargo command, matrix, measurement, Linear operation, source edit or broad audit ran in this reviewer lane.

```yaml
dispositions:
- fnd: FND-003
  outcome: fixed
  after_excerpt: "                .map(|position| {\n                    let stamp\
    \ = position.fingerprint.is_none().then_some(position.stamp);\n              \
    \      (position.identity, stamp)\n                })\n                .collect::<BTreeSet<_>>()\n\
    \                .len()"
```
