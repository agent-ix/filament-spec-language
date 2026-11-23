---
id: SR-3621001
title: "Spec review of the FR-875 witness-path limit gap bullet (filament-spec-language PR 3621)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@664481afbba4f0b1ff00266fd4d0708ccc69e618; spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md (Settling the item, new max_witness_paths bullet)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: references
---

# Spec review of the FR-875 witness-path limit gap bullet (PR 3621)

## Summary

Ticket: QSL-2979. Scope `git diff origin/main...HEAD` at head
664481afbba4f0b1ff00266fd4d0708ccc69e618: one five-line bullet in FR-875.
`quire validate` passes with no warnings.

Grounding:

- FSpec FR-413 (rev 3200a4e, the Cargo.lock dependency) says the provider
  emits "a path set when at most `max_witness_paths` paths reach the bound".
  Otherwise "the evidence is an FR-412 certificate", and "A subsystem
  certificate for a quantile claim or an `on each` instance carries a
  `lower` certificate for `V` and one for `N`" (FR-413 lines 136-137,
  172-176). The bullet's "lower certificates for V and N" matches.
- FSL FR-202 Emission requires the same fallback ("otherwise subsystem
  evidence"). The bullet records that it is not built.
- An `incomplete` `LimitReached` settlement is the FR-203 row for "Another
  limit reached" (`Incomplete(LimitReached{limit, value, setting})`). It is
  an honest non-verdict, never a wrong one, so it is acceptable as a
  recorded gap.
- The code on the QSL-2979 work branch (origin/QSL-2979-work,
  fsl-analyze/src/analyze/exact/instances.rs:50-58) passes
  `limits.max_witness_paths` into `on_each_outcome` and settles its errors
  through `failure(error, &limits, subject)`. The path-set cap check is in
  exact.rs and witness.rs:879-883 (`ModelCheckLimit::ExactProbMaxWitnessPaths`).
- Limit names: `ModelCheckLimit::as_str` gives `max_witness_paths`
  (fsl-replay/src/model_check/limits.rs:319), and `setting()` gives
  `exact_prob.max_witness_paths` (:382; fsl-foundation/src/setting.rs:158;
  FR-255 row at line 288). The request member is `limits.max_witness_paths`
  (fsl-analyze/src/analyze.rs:792).
- Style: the bullet is descriptive, with no SHALL and no AC, the same as the
  QSL-3003 undefined-read bullet just above it. An AC is optional. One with
  `max_witness_paths` 0 (as FR-202-AC-4 does) would pin today's behaviour,
  but it would then have to change when QSL-3004 lands, so leaving it out is
  acceptable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "likewise settles `incomplete`" misreads the neighbouring bullets. The profile-boundary bullet and the QSL-3003 bullet both settle `unsupported`, so "likewise" suggests the same settlement, and the bullet then has to add "(not `unsupported`)". Candidate: "An in-profile on-each refutation that needs more than `max_witness_paths` path sets settles `incomplete` instead (not `unsupported`) ...". | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:136-140 |
| FND-002 | low | Only the `limit` is named. FR-203's `LimitReached{limit, value, setting}` and FR-255 require the raising setting too, and the code writes `limit` `max_witness_paths` and `setting` `exact_prob.max_witness_paths` (limits.rs:319, 382). Candidate: "`LimitReached` with `limit` `max_witness_paths`, its `value`, and `setting` `exact_prob.max_witness_paths` (FR-255)". | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:137-138 |
| FND-003 | low | The gap contradicts two SHALLs, and only FR-875 records it. FR-202 Emission says "EN-5 SHALL emit a path set when at most `max_witness_paths` paths ... otherwise subsystem evidence", and FR-875 says "the item SHALL be `refuted` when any instance fails". FR-202 has no matching current-gap note, so a reader of FR-202 alone sees no exception. Candidate: add to FR-202 Emission "(current) for an `on each` instance the subsystem-certificate fallback is not built until QSL-3004; FR-875 states the settlement", or cite FR-202 Emission in this bullet. | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:136-140; spec/functional/FR-202-replay-a-probabilistic-witness.md:123-127 |

## Verdict

Mergeable. The gap is grounded in FSpec FR-413 and FSL FR-202, and settling
`incomplete` `LimitReached` is an allowed honest settlement under FR-203.
The style matches the QSL-3003 bullet. The three low findings are wording
and cross-reference, and none is blocking. EARS and atomicity: no
requirement statement was added (descriptive gap prose), so no separate
EARS, integrity or criterion-strength pass was needed.
