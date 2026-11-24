---
id: SR-982
title: "Base specification review — QSL-3092 exact profile batch"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-195-AC-8, FR-196-AC-17, FR-204-AC-21, FR-877-AC-5, FR-940-AC-39; related FR-201"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
---

## Summary

Reviewed all five new criteria against their owning FR prose, ticket intent, IDs, links, and current acceptance evidence. Four criteria fit their cited requirements; FR-196-AC-17 conflicts with FR-201's explicitly narrower exact certificate profile for bounded event-position forms.

## Verdict

**CONDITIONAL** — resolve the exact product versus certificate-profile conflict before implementation.

## Assurance Context

No applicable AssuranceProfile was found in the repository specification tree. This is a direct base review, not an enforced assurance run.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-196-AC-17 requires the event-position product to build bounded `until` formulas and avoid unsupported outcomes, but FR-201's bounded certificate subset admits only `eventually[0,h]` and `always[0,h]` and says other temporal forms remain unsupported. FR-203 requires an accepted certificate on a proved record, so this `until` item cannot complete the exact settlement path as written. Align the owning certificate profile or limit the product claim to forms with an accepting exact evidence path. | spec/functional/FR-196-build-the-probabilistic-product.md:957; spec/functional/FR-201-check-a-probability-certificate.md:197 | wrong-requirement |

## Examined scope

| ID | Role | Excerpt |
| --- | --- | --- |
| FR-195-AC-8 | examined | When all applicable EN-4 gates are present, the gate returns the first cause in the specified order: a `FairTerminates`-shape claim under every scheduler with no confidence parameters is refused `EveryScheduler`, before `ExactOnlyForm` or `MissingConfidence`; a `Link` expected-reward claim under a workload with no confidence parameters is refused `ExactOnlyForm`, before `MissingConfidence`; a bounded event under a workload with no confidence parameters is refused `MissingConfidence`; and `P95` under `Steady` with the stated confidence parameters passes. No refusal builds a `StatisticalClaim` or draws a sample. |
| FR-196-AC-17 | examined | Under `filament.temporal.event-position.false-extension/v1`, each admitted bounded event formula is built with its FR-338 formula monitor in the product: the `eventually[0,2]` formula of FR-201-AC-13, the bounded `holds(A) until[0,1] holds(B)` and `always[0,1] holds(P)` shapes admitted by FR-186/FR-326, and the nested `eventually[0,100] always[0,10] holds(n.healthy)` of FR-186-AC-3. Each product reads a formula position once at its model state and applies the event-position false-extension closure at terminal model states; the route builds a formula monitor and does not refuse these admitted formulas as `unsupported` or `not-yet-implemented` or substitute the timed-deadline monitor. |
| FR-204-AC-21 | examined | Over the timed `Retx` subject of AC-1, under the timed profile EN-5 runs every bounded-window formula that FR-186 admits and retains a monitor for every bounded operator occurrence, including nested occurrences and their nonzero lower bounds. Representative admitted formulas are `eventually[1 ms, 4 ms] holds(m.delivered)`, `always[0 ms, 4 ms] holds(m.delivered)`, `holds(not m.delivered) until[1 ms, 4 ms] holds(m.delivered)`, and nested `eventually[0 ms, 4 ms] always[1 ms, 1 ms] holds(m.delivered)`. The first, third and nested forms each have minimum `99/100` and settle `proved` at threshold `0.99`; the `always` form has value `0` and settles `proved` for `probability <= 0.01`. Every bounded-window formula admitted for the timed profile is decided by EN-5 and does not settle `unsupported` or `not-yet-implemented` (FR-186, FR-204). The timed profile refuses `probability >= 0.99 [eventually holds(m.delivered)]` as `unsupported_construct`/`expression-form` at `eventually`, and refuses `probability >= 0.99 [eventually[1 ms, *] holds(m.delivered)]` as `unsupported_construct`/`expression-form` at the interval, before EN-5 is called (FR-186). |
| FR-877-AC-5 | examined | Over `Retx` with `M` = `elapsed from holds(not c.late) until holds(c.late) within 4 ms`, `quantile 0.5 of (M) <= 4 ms` under workload `One` (the scheduler-selected send delay and minimum `99/100` of FR-204-AC-5) settles `proved` with an accepted `timed-product/v1` exact certificate. |
| FR-940-AC-39 | examined | For each case named in FR-195-AC-1/2, FR-198-AC-2, FR-199-AC-3/4, FR-186-AC-12/16, FR-1043-AC-1/4/5, FR-204-AC-2, FR-877-AC-1, FR-196-AC-5 and FR-1043-AC-11, paired `program` and `package` requests over that case's same checked unit exercise (a) `Reach` (`eventually`, `until`, `always`) and `ExpectedReward`; (b) unweighted and duration-weighted `LongRunFraction`; (c) bounded `steps`, reward `accumulate`, `elapsed`, weighted and unweighted fraction measures, quantile and mean-of-fraction; (d) timed `MeanTime` and elapsed quantile; or (e) an undefined bounded-form read or undefined fraction read, respectively. For every source-decided item the package request returns the same item record, exact certificate or replayed witness, and process exit; it never returns `unsupported`/`not-yet-implemented` for one of those source-decided items. |
| FR-201 bounded certificate profile | context_only | The bounded subset SHALL admit origin-activated event-position `eventually[0,h] holds(P)` and `always[0,h] holds(P)` with a finite nonnegative position horizon and a checked state predicate P; ... Other temporal forms remain unsupported by this profile. |

## Evidence and limits

Scoped Quire validation of the five changed FRs exits 0. The computed matrix reports these new Test criteria untagged; no implementation coverage is claimed because this is the spec-first PR. The remaining partial-ticket subscopes were checked against the FR map and left stopped where the existing FR prose is silent, rather than assigned new behavior. No hash, digest, pin, or file-tracking requirement was introduced. Existing FR-877 grammar warnings at lines 59 and 63 are unchanged and outside the new criterion.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-196-AC-17 now limits its claim to the `reachability/v1` subset admitted by FR-201, with examples covered by FR-201-AC-13 and FR-201-AC-16. |

After excerpt: `Under filament.temporal.event-position.false-extension/v1, EN-5 builds the bounded formula product with its FR-338 monitor for the reachability/v1 subset FR-201 admits: eventually[0,2] holds(P) over FR-201-AC-13's horizon-2 model settles with its exact value 3/4, and always[0,1] holds(P) over FR-201-AC-16's model has minimum 1/4 and maximum 3/4, accepted at those bounds.`
