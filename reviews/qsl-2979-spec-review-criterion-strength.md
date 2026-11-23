---
id: SR-3603003
title: "Criterion-strength review of FR-201-AC-76 (filament-spec-language PR 3603)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language@32ca5b98116c0ca890097e2604f1e680ed721a1f; spec/functional/FR-201-check-a-probability-certificate.md (FR-201-AC-76)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
---

# Criterion-strength review of FR-201-AC-76 (PR 3603)

## Summary

Ticket: QSL-2979. No Jev (typesafe.ai System One) client is installed on this
host, so the calibrated semantic judgment was skipped. This review was done
by hand over the spec text only.

AC-76 is falsifiable on its values. The accept at 9/10 and the reject at
99/100 bracket 391/400, so a wrong value or a wrong comparison fails. The
accept requires `n`'s 0/0 to be read as vacuous, not as a failure. The
tampered numerator 1/2 also misses the threshold, so it tells
conditions-before-threshold apart from the reverse order. The all-vacuous
case separates the `Ratio` rejection from the `Vacuous` acceptance.

Weakness kind: `partial`. It does not pin some loci, and it lacks some
adverse cases. Adverse-case coverage of the profile is partial. The
only-`n` contradiction is SR-3603001 FND-005.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Loci are not pinned. The `ThresholdMissed` case states no `at`. The tampered case states `part` `numerator` but not the key, `m`'s initial key. So a checker that reports at the wrong key or part (for example the numerator-part locus PR #3599 writes for ThresholdMissed) passes AC-76. Add `at` `m`'s initial key with no `part` for ThresholdMissed, and `at` `{m's initial key, part: numerator}` for the tamper. | spec/functional/FR-201-check-a-probability-certificate.md:1713 |
| FND-002 | medium | Adverse cases of the profile's own rules are missing. No case covers a missing, extra or duplicate row (`Malformed`), a denominator-part Bellman failure (`part` `denominator`), a denominator of 0 at a key whose activation is reachable, a `Ratio` document offered under every scheduler (`Malformed` at `mode`), or a form outside the profile (cyclic graph, fairness, timed) staying unsupported. Each rule can regress with AC-76 still passing. | spec/functional/FR-201-check-a-probability-certificate.md:1713 |

## Verdict

Partial. The numeric conjuncts are strong. The loci and the adverse cases
need FND-001 and FND-002 before the criterion tells a correct checker from a
merely plausible one.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-76 pins ThresholdMissed at `m`'s initial key with no part. AC-78 pins every tamper at `m`'s initial key with its part. |
| FND-002 | fixed | AC-78 adds the denominator-part and rank cases, AC-79 the row-coverage faults per part, and AC-80 the every-scheduler `mode` and out-of-profile cases. A denominator of 0 where activation is reachable is covered by the denominator Bellman check (AC-78). |
