---
id: SR-96723
title: "QSL-3490 spec review of ADR-038 route admission and the FSL timed restatements (filament-spec-language PR 4042)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@aeb96979847e81c7470b77c72b7db548ad556cc5; spec/decisions/ADR-026, ADR-028, ADR-038; spec/functional/FR-196, FR-204, FR-243, FR-254, FR-877, FR-921, FR-922, FR-924; spec/test-cases/TC-709; docs/timed-window-tail-bound.md; spec/spec.md"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3490 (parent QSL-3489). Independent soundness review of filament-spec-language#4042 at head aeb96979847e81c7470b77c72b7db548ad556cc5, read against DESIGN.md C1-C12, the QSL-3490 comments and the arch rulings of 2026-11-25 (Q1 adopt FR-921 amendment as data, Q2 FR-407 infinite, Q3 FR-420 to the exact side, Q4 and Q5 as written, mixed delays and the minimum side with a reachable time-lock leave digital).

Checked as a mathematician against KNPS 2006 (FMSD 29, author PDF fetched): Lemma 17 (<=, =, >= survive every threshold), Definition 8 (almost-sure divergent adversaries, for every state), Proposition 23 (randomized integral adversary per threshold), Theorem 24 (location reachability, min and max), Theorem 27 (expected cost, uniform rate) state what the rows say, except as FSL FND-007 notes. Arguments A (FR-243 (a)-(e): threshold frac(r) for future upper bounds, just below it for lower-bounded and past cases; operand restrictions make one violating position suffice), B (round-up keeps t <= D, round-down t < D, from position 0), D (deadlock at capped clocks; closed guards from an integer valuation are met by a whole delay) and G (all given: every pre-tail stamp whole) held against my attempts with small closed models. Argument M failed (counterexample in FND-001). Hand vectors recomputed: Gap (dense max 1, integer 0), later-position (dense min 0, integer 1/2), Split (dense min 0, integer 1/2), Burst ((1+m)/(2+m), whole stutters <= 2/3). Changed values recomputed: FSpec FR-420-AC-7 / TC-374 ST-07 and FSL FR-254-AC-4 / TC-709 step 4 (position at 1/2 ms in B: estimate 1, rejected; fraction unsupported); FR-416-AC-12 to AC-16 hold. Almost-sure divergence agrees with FR-204-AC-16 / FR-924-AC-4 (min = max = 1; under sure divergence no scheduler is admitted) and with ADR-028 13a's probability-1 reading, apart from FSL FND-002. C9 untouched (ADR-026 CK-4, EZ-4, RT-8 not in the diff). Not verified: Alur-Dill 1994, Behrmann et al. 2006, KNSS 2002 and Henzinger-Manna-Pnueli 1992 statements (not fetched); FR-1051 (#4035, not in tree); no build or test run; quoin sub-analysis tools not run, review done by hand.

## Verdict

CHANGES REQUESTED. Argument M is false (high). ADR-028 13a still excludes schedulers that head into a time-lock. The arch rulings Q1 to Q3 and the mixed ruling leave FR-921, FR-924, ADR-026 SD-4 and ADR-038 text to change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Argument M is false: a race of a given delay against a free one is a strict comparison, so the mixed rows admit a wrong digital value; arch has ruled mixed leaves digital. | spec/decisions/ADR-038-one-timed-run-layer-and-route-admission.md:118-120,170-174 |
| FND-002 | medium | 13a's amendment lets a prefix end at a time-lock, but R and 'admissible continuations exist exactly in R' still exclude states that lead to a lock, so a literal reading gives Split's minimum 1/2 instead of 0. | spec/decisions/ADR-028-exact-probabilistic-engine.md:516-525 |
| FND-003 | medium | C8 residue: ADR-028 SP-2 still says EN-5 reuses model_check's product, monitors and SCC decomposition with no timed exception, against FR-196's new timed clause and ADR-038 RL-2; SP-2 is not in 'Passages replaced'. | spec/decisions/ADR-028-exact-probabilistic-engine.md:96 |
| FND-004 | medium | Arch Q2: an expected-until whose target is not reached w.p. 1 is infinite (FSpec FR-407); FR-924 still gives a locked run no further elapsed reward, i.e. a finite value. | spec/functional/FR-924-certify-time-divergent-verdicts.md:85-89 |
| FND-005 | medium | Arch Q1 adopted the FR-921 amendment; FR-921 items 1 and 5, ADR-038's 'Drafted amendment (not applied)', the FR-1051 trusted-base exception and the row acceptance test are not yet written. | spec/functional/FR-921-check-a-timed-product-certificate-without-the-engine.md:56-78 |
| FND-006 | medium | Arch Q3: a bounded-delay state with no racing identity is a time-lock (FR-204-AC-17, FR-924-AC-5 exact 0); ADR-026 SD-4 still settles it NotStochastic. | spec/decisions/ADR-026-dense-time.md:242 |
| FND-007 | low | Citation precision: no KNPS theorem states 'measures of the discrete step sequence'; it is a corollary of Proposition 23 plus the converse inclusion, and should be marked as an argument. Lemma 20 is the duration identity; the divergence equivalence is drawn in Proposition 23's proof. | spec/decisions/ADR-038-one-timed-run-layer-and-route-admission.md:114,118 |

## Coverage

- ADR-038 (examined): RL-1 to RL-6; drafted FR-921 amendment; route admission table; arguments A, B, D, G, M; hand vectors; passages replaced.
- ADR-026 (examined): RU-5, TD-1, TD-6, EZ-10, section 9, SS-7, section 15, consequences; C9 rows CK-4/EZ-4/RT-8 confirmed untouched.
- ADR-028 (examined): Section 1 context, CE-7, TA-1 to TA-5, section 13a amendment, zone-based alternative.
- FR-196 (examined): Timed subjects use the one concrete run and their own abstraction.
- FR-204 (examined): Description; Admission route and time-lock rules.
- FR-204-AC-16 (examined): Retx variant with no guard/reset: min and max both 1.
- FR-204-AC-6 (context_only): Race: given A at 2 vs free B; min 0, max 1 (single race, no strictness effect).
- FR-243 (examined): Rules (a)-(f) and the pointer to argument A.
- FR-254-AC-4 (examined): Lock: eventually estimate 1 Rejected; fraction unsupported TimeLockedSample.
- FR-877 (examined): M <= c digital only from position 0 or class given.
- FR-921 (examined): Exception list; item 1 route; item 5.
- FR-922 (examined): Region route for every claim FR-204 does not give the digital MDP.
- FR-924 (examined): Time-lock bullet; ACs 3-5.
- TC-709 (examined): Step 4 expected values.
- docs/timed-window-tail-bound.md (examined): Almost-sure divergence now stated by FR-416.

## New findings (disposition pass 1)

Head checked: dfada161883cc6eca32769fe9e69d14f1dbf3828.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | The digital rows do not exclude fairness sets; KNPS covers none, and argument G does not cover enabledness inside a race delay; a strong fairness set gives a wrong digital minimum. | spec/decisions/ADR-038-one-timed-run-layer-and-route-admission.md:110-114 |
| FND-009 | medium | Arch ruling (2): the kept mixed-race criteria must be marked in the admission table as pending the region route; ADR-038 lists them in Consequences only and omits FR-202-AC-11 and FSpec FR-421-AC-6. | spec/decisions/ADR-038-one-timed-run-layer-and-route-admission.md:264-267 |
| FND-010 | low | Row E's 'deadline event' lacks 'on origin'; argument E rounds with argument B, which keeps deadlines only from position 0. | spec/decisions/ADR-038-one-timed-run-layer-and-route-admission.md:115,182-194 |

## New findings (disposition pass 2)

Head checked: 1f28bffa4099537db59f78ed16c6f52d15f3dc34.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | The pending row's list of kept mixed-race criteria omits FSpec FR-413-AC-8 (a workload refutation with a free delay), which FSpec FR-421 names as pending. | spec/decisions/ADR-038-one-timed-run-layer-and-route-admission.md:117 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
| FND-010 | fixed | - |
