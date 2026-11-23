---
id: SR-3603001
title: "Spec review of the on-each reachability conditional profile (filament-spec-language PR 3603)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@32ca5b98116c0ca890097e2604f1e680ed721a1f; spec/functional/FR-201-check-a-probability-certificate.md (Closed on-each reachability conditional profile, FR-201-AC-76); spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md (Instances); spec/test-cases/TC-3208-an-on-each-reachability-ratio-document-is-checked-per-instance.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3208
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: references
---

# Spec review of the on-each reachability conditional profile (PR 3603)

## Summary

Ticket: QSL-2979. Scope `git diff origin/main...HEAD` at head
32ca5b98116c0ca890097e2604f1e680ed721a1f: the FR-875 cause spelling, the new
FR-201 section "Closed on-each reachability conditional profile",
FR-201-AC-76 and TC-3208. Grounding sources read: FR-201 Inputs, rejection
table, "General checking conditions" and "Conditional certificates"; FSpec
FR-412 and FR-331 at the dependency rev 3200a4ed (Cargo.lock); FR-196
"Activation of an on each instance" and the product key; FR-741-AC-5;
FR-875; FR-286; FR-291-AC-1; ADR-028 section 15.4.

Numbers, by hand. `Link` (ADR-028 section 15.4): `send_a` loses with 1/10,
`send_b` with 1/5, at most 2 attempts; `Even` weights both 1. After one loss
the value is 1/2 * 9/10 + 1/2 * 4/5 = 17/20. At the start it is
1/2 * (9/10 + 1/10 * 17/20) + 1/2 * (4/5 + 1/5 * 17/20)
= 1/2 * 197/200 + 1/2 * 194/200 = 391/400. ADR-028 states the same values.
The unbounded `inf` form equals the bounded one, because after two attempts
no action is enabled and the state stutters undelivered. With `n` delivered,
`n`'s actions are disabled, so the workload resolves over `m`'s actions only.
`m` is undelivered at position 0, so the guard holds there: denominator 1,
numerator 391/400. `n` is delivered and never reverts, so it never activates:
denominator 0 and numerator 0. Comparisons: 391/400 >= 360/400 (9/10) holds;
391/400 < 396/400 (99/100) holds. The tampered numerator 1/2 breaks Bellman
equality at `m`'s initial key. The rejection table checks conditions before
the threshold, so the result is InequalityFails, not ThresholdMissed.
391/400 is derivable from the spec text, from ADR-028 section 15.4, which
FR-741-AC-2 names as the `Link` model. FR-741 and FR-875 alone give the value
but not its derivation.

The four open questions:

- (a) Vacuous case. Grounded and unambiguous. FR-201 "Conditional
  certificates": "when no initial key has reachable activation, it SHALL
  reject the numeric `Ratio` certificate `InequalityFails` at the first
  initial complete product key in canonical order ... and the claim SHALL be
  certified only through an accepted `Vacuous` conditional certificate". The
  profile's last bullet repeats this. The code's AcceptedVacuous for a
  `Ratio` document is wrong and must change. FSpec FR-412 alone ("where the
  denominator is 0 ... that instance holds vacuously") has no all-vacuous
  carve-out. FR-201's stricter rule fits FR-412's `vacuous` mode, "for a
  reading in which every instance is vacuous".
- (b) Part shape. The profile text is wrong. FSpec FR-412 `conditional`
  says "each part an `exact` certificate of this form". FR-412 Absence lists
  "with a part whose `kind` is not `exact`" and "an `exact` certificate
  without `ranking`" as `malformed`. FR-201's Inputs give
  `Ratio { numerator: Box<ProbabilityCertificate>, .. }`, and its table
  rejects "a part that is not `Exact`" as Malformed. Bare `{values: [...]}`
  parts are malformed by both FSL and FSpec text (FND-001, FND-002).
- (c) Vacuous document objective. FR-412 Properties: `identity`,
  `objective` and `kind` are 1..1, and the provider emits "exactly the
  members below". `vacuous` "carries no parts and an empty `values`". So a
  `vacuous` document carries an objective. The profile does not say which
  objective, or what the top-level member set is (FND-003).
- (d) ThresholdMissed locus. Partly grounded. FR-201 "General checking
  conditions" rejects at "the first failing condition and state in canonical
  order". The table and FR-412 put ThresholdMissed "at the initial state".
  `part` belongs only to "a part of a `Conditional` certificate [that] fails
  its own conditions", and the threshold comparison is a condition on the
  ratio, not on a part. So the locus is the first failing initial key in
  canonical order, with no `part`, and the code's numerator-part locus is
  wrong. The spec should still say this explicitly for several initial keys
  (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The profile defines each ratio part as "a `values` array of `{state, value}` rows". FSpec FR-412 says each part is "an `exact` certificate of this form", and FR-412 Absence and the FR-201 table reject a part that is not `exact`, or an `exact` without `ranking`, as `malformed`. So every conforming profile document is malformed under FR-412. Fix: make each part a full `exact` document `{identity, objective, kind: "exact", values, ranking}`, and state each part's objective (numerator: the form's target read from activation, never-activating paths 0; denominator: the first activating states), all under `workload`. | spec/functional/FR-201-check-a-probability-certificate.md:882-884 |
| FND-002 | high | Soundness. Parts are checked by Bellman equality alone, with no ranking, no Prob0/Prob1 recompute and no acyclic restriction. The profile admits any untimed unbounded `eventually` under `inf`, including cyclic graphs. On a cycle, `x = F(x)` has spurious fixed points (for example a waiting self-loop valued 1 in the denominator, or a non-target loop valued 1 in the numerator), so a false ratio passes. `reachability/v1` requires `ranking` and an acyclic graph for this reason. Fix: check each part by the `Exact` conditions (CE-3 rank progress, decided-state recompute) once FND-001 is fixed, or restrict admission to acyclic continuation graphs and require ranks. | spec/functional/FR-201-check-a-probability-certificate.md:893-897 |
| FND-003 | medium | The document envelope is unspecified. The profile gives the objective's members but not the document's (are top-level `values`, `ranking` or `policy` present or forbidden?). It gives no `Vacuous` document shape and no reader rules (canonical RFC 8785 bytes, unknown or duplicate members, key order). FR-412 needs `identity`, `objective` and `kind` 1..1, and `vacuous` "carries no parts and an empty `values`". Candidate: "The `Ratio` document SHALL have exactly `identity`, `objective`, `kind` and `conditional`. A `Vacuous` document of this profile SHALL have exactly `identity`, the same objective, `kind` `"conditional"`, `values` `[]` and `conditional` `{mode: "vacuous"}`. The `reachability/v1` reader rules apply." | spec/functional/FR-201-check-a-probability-certificate.md:877-884 |
| FND-004 | medium | The ThresholdMissed locus is undetermined with several initial keys. The profile says only "rejects `ThresholdMissed` by 'Conditional certificates'", and the table row reads "of the initial state", singular. FR-875 names "the first failing instance in key order", which need not match canonical product-key octet order. Candidate: "rejects `ThresholdMissed` at the first initial complete product key, in canonical order, whose ratio misses the threshold, with no `part`, because the comparison is a condition on the ratio and not on either part." | spec/functional/FR-201-check-a-probability-certificate.md:898-903 |
| FND-005 | high | FR-201-AC-76 contradicts the profile. Over a snapshot holding only `n`, "the same document" still carries rows for `m`'s keys, which the checker does not rebuild there. The profile's coverage rule then rejects it `Malformed` at `values`, not `InequalityFails` at `n`'s initial key. Nothing orders the coverage check before or after the all-vacuous rule. Fix: make the AC use the document over that snapshot (`n`'s rows only, numerator and denominator 0), or state the check order. | spec/functional/FR-201-check-a-probability-certificate.md:1713 |
| FND-006 | low | The FR-875 cause fix is right in spelling. The code (fsl-analyze tests read `record.cause.kind`) and FR-291-AC-1 agree on `kind`. But the new parenthetical cites FR-286 for "spells every cause under `kind`", and FR-286 says only "in FR-331's wire spelling". FSpec FR-331 entries carry a bare `type` (FR-331-AC-51), so the citation does not support the claim. Cite FR-291-AC-1 alone, or state the outcome-document cause tag where FR-286 owns it. Also, code PR #3599 must change fsl-analyze/tests/it/probability_from_source.rs:702, which still asserts `cause.type`. | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:70 |

## Verdict

Not mergeable. The numbers are right, and the vacuous rule is grounded and
unambiguous. But the part shape contradicts FSpec FR-412 (FND-001). Without
ranks or acyclicity the checker is unsound on cyclic graphs (FND-002).
AC-76's only-`n` case contradicts the profile's own coverage rule (FND-005).
FND-003 and FND-004 are spec gaps the code PR must agree with. Wording for
each is above.

## New findings (disposition pass 1)

Head 1afe025a0e745374f4015b3614bef662a192dba5 (diff 32ca5b98..1afe025a).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The profile's admission limit, as AC-80 tests it, contradicts FR-875. AC-80 says a claim of the on-each form "over a cyclic continuation graph, with authored fairness or over a timed subject stays unsupported", and the profile says every other on-each form "SHALL stay unsupported". FR-875 says an `on each` claim "settles from source exactly as an `on origin` claim does; it is never `unsupported` for its activation" and "SHALL NOT be settled `unsupported` for its activation". A cyclic on-origin eventual claim is certified by the closed cyclic DTMC profile, so a cyclic on-each claim settling `unsupported` is unsupported only because of its activation. Limiting the codec's admission is fine. Stating it as item settlement is not, unless FR-875 records the gap. Candidate: in AC-80 write "is not admitted by this profile: no `on-each-reachability/v1` document is written or accepted for it", and add to FR-875 Behavior "(current) An on-each claim outside FR-201's `on-each-reachability/v1` profile settles `unsupported` `not-yet-implemented` until QSL-2995; this is an implementation gap, not a refusal of its activation." Or name the item settlement explicitly as the owner's decision. | spec/functional/FR-201-check-a-probability-certificate.md:868-877; spec/functional/FR-201-check-a-probability-certificate.md:1749 |
| FND-008 | medium | A `Vacuous` document offered where activation is reachable has no stated locus with several initial keys, and no AC tests it (author open question 3). The table and FSpec FR-412 say `InequalityFails` "at the initial state". Over AC-76's snapshot both `m` and `n` are initial keys, and "first in canonical order" alone could name `n`, from which nothing activates. Candidate: "A `Vacuous` document SHALL be rejected `InequalityFails` at the first initial complete product key, in canonical order, from which a first activating state is reachable, with no `part`". Add to AC-77: "the `Vacuous` document over AC-76's snapshot is rejected `InequalityFails` at `m`'s initial key with no `part`". | spec/functional/FR-201-check-a-probability-certificate.md:931-936 |
| FND-009 | medium | The rank rule is ambiguous. "ranks SHALL be the checker-verified acyclic height ranks the profile above allows" can mean the ranks must equal the heights (terminal 0, live 1 + max successor), or only that height ranks are a valid choice. Under the first reading, a rank that satisfies CE-3 progress but is not the height (for example 5 at `m`'s initial key) has no stated rule, and AC-78's rank-0 `RankBroken` could instead be a height-mismatch rejection. reachability/v1 says "MAY emit". Candidate: "ranks SHALL satisfy CE-3 progress; the checker-verified acyclic height ranks are one valid ranking; a rank that breaks progress SHALL be rejected `RankBroken` (`RankMissing` when absent)". | spec/functional/FR-201-check-a-probability-certificate.md:910-920 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Lines 891-900: each part is a full `Exact` document with exactly identity, objective, kind "exact", values and ranking. A non-exact part or one without ranking is Malformed. |
| FND-002 | fixed | Admission is limited to acyclic continuation graphs apart from terminal stutter (lines 871-872). Parts are checked by the Exact conditions, including CE-3 rank progress (lines 910-920). |
| FND-003 | fixed | Lines 883-890 give the Ratio and Vacuous envelopes and import the reachability reader rules. |
| FND-004 | fixed | Lines 921-928: ThresholdMissed is at the first failing initial complete key in canonical octet order, with no part, and the text notes FR-875's key order. |
| FND-005 | fixed | AC-77 uses the document built over the only-n snapshot. The check order is stated at lines 935-936. |
| FND-006 | fixed | FR-875:70 now cites FR-291-AC-1 for `kind` and names FSpec FR-331's bare `type` as the provider-entry spelling. |
