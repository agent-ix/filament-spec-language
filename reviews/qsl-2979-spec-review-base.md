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
