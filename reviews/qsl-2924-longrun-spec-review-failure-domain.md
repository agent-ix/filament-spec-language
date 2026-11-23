---
id: SR-967175026302197768900474181941935084784
title: "failure-domain review of PR #3550 every-scheduler long-run proof profile"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/test-cases/TC-3190-check-every-scheduler-long-run-proofs.md, spec/test-cases/TC-3191-refute-a-long-run-fraction-over-every-scheduler.md (git diff origin/main...900ddd14)"
review_set: subset
---
# failure-domain review of PR #3550

## Summary

Ticket: QSL-2924 (item 3). PR: filament-spec-language#3550 at 900ddd1477c116b0c5ad487671b073327f60e92d. Unstated failure modes, scope leaks and topological edge cases in the MDP long-run profile and the profile admission rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The admission rule silently widens the weighted DTMC profile (and FR-202 long-run refutation) to event-position claims, with no pointer in that profile and no AC | spec/functional/FR-201-check-a-probability-certificate.md:997-1003,1128-1131 |

## Detail

**FND-001** (medium, coverage, confidence high): 'every long-run document profile below SHALL ... admit the claim under any temporal profile' also covers the closed weighted DTMC long-run profile (FR-201:1128). The PR edits the unweighted DTMC intro to point at the admission section, but the weighted intro has no pointer. AC-74 covers only the unweighted MDP and unweighted workload DTMC cases under event-position. A weighted `using ev` claim is therefore newly required to be admitted and certified, and to 'SHALL NOT settle unsupported', without the weighted profile saying so and without an AC. The same code gate (fsl-analyze/src/analyze/exact/long_run.rs:30-35) guards both. The refutation side under event-position (FR-202's subsystem entry) is likewise untested. Fix: either add the pointer to the weighted profile plus an AC (for example FR-201-AC-31's chain `using ev` accepts at 600/601) and one FR-202 refutation under event-position, or limit the admission section to the unweighted profiles and state that weighted event-position claims stay outside.

## Verdict

The MDP profile handles undefined P, cancellation and exhaustion, zero instances, terminal stutter, producer-partition distrust (AC-73's omitted-MEC false proof) and bogus cycle fixed points (AC-72) explicitly. The only scope leak is the implicit widening of the weighted profile and of refutations to event-position claims. Open questions 2 and 3 are outside scope and drop no behaviour: weighted every-scheduler and fair every-scheduler long-run claims are 'outside this profile', not unsupported, and FR-201 Emission (lines 135-142) together with FR-199 and FR-200 still require them through the general certificate form or QSL-2830.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The weighted DTMC long-run profile's temporal-profile admission is still not stated anywhere, and 'not changed by this rule' points at an implicit inheritance whose source changed | spec/functional/FR-201-check-a-probability-certificate.md:1137-1156 |

**FND-002** (low, ambiguous, confidence medium): The admission section now says 'The closed weighted DTMC long-run profile's admission is not changed by this rule', which correctly removes the silent widening. But the weighted profile never stated its temporal profile itself. It 'SHALL retain the unweighted profile's untimed ... DTMC admission', and the unweighted intro now admits both event-position and infinite-trace through the admission section. So a reader can still infer that weighted `using ev` is admitted, which contradicts 'not changed' and the code gate at fsl-analyze/src/analyze/exact/long_run.rs:32-33. This was implicit on main too, so it is low. Fix with one clause in the weighted intro: 'in the untimed infinite-trace profile'.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified at 11c18bc0 (round 1) |
