---
id: SR-1478
title: "Gap analysis — timed interval boundary cases"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2081; codex/qsl-2081-zone-interval-cases@de858f003fbaabc2369744734667968443f33f99; added interval_boundaries tests in fsl-analyze/tests/it/zone_mitl.rs; exact FR-234, FR-237, FR-242 and FR-1034 sections"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Compared the new real-source interval cases with their current owning requirements and the assigned acceptance scope. Open/closed endpoints, punctual windows, always, and typed interval refusal cases have meaningful executable assertions. Two remaining required scenarios preserve strict owning defects. Generic replay is incorrectly bound to a concrete Rpc acceptance criterion.

## Verdict

**PASS** — the incorrect new trace bindings are fixed; the two explicit product blockers remain pending and excluded from coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The eight new FR-237-AC-1 trace bindings overstate coverage of the criterion's named Rpc/Settles cases. | fsl-analyze/tests/it/zone_mitl.rs:367; spec/functional/FR-237-replay-a-timed-counterexample.md:219 |

## Coverage

Plan completion: not assessed

Reconciliation: computed matrix not rerun, as explicitly instructed; no repository-wide assurance or coverage-total claim is made. This is the assigned diff's requirement/test/code check, including reverse ownership and hollow-assertion inspection.

- FR-234 Behavior and FR-242 Behavior: open/closed lower and upper pairs, origin punctual windows, and always variants exercise real zone model checking. Valid counterparts and moved-event refusals distinguish the interval endpoints.
- FR-242-AC-2 and the bounded-window half of FR-234-AC-3: the punctual case proves the exact instant and refutes earlier/later witnesses.
- FR-234-AC-2: the real semantic checker refuses inverted and empty-open intervals with the required typed reason, code, cause and exact interval span.
- FR-237: all returned timed counterexamples are replayed; the concrete AC-1 Rpc examples are not these new fixtures and must not be claimed by their tags.
- FR-242/FR-234 nested intervals and FR-1034 unbounded upper: strict paired scenarios exist but remain pending under QSL-2689 after the accepted public `admitted-witness` faults. They count as no completed coverage; no later pair result is invented.
- Reverse ownership: every added model/helper and assertion serves the interval or replay obligations above. No unowned production feature or stub was introduced.
- The expected verdict is not copied from an arbitrary constructor: actual public model checking, independent core certificate validation and real replay can fail on implementation errors.

## Evidence boundaries

Author receipts `.agent/qsl-2081-checkpoint.md` and `.agent/qsl-2081-new-intervals-focused.log/.exit` are accepted in-lane evidence. Review performed no runtime, baseline measurement, old-test rerun, matrix, inventory or ownership audit. The known lint blockers remain limitations, not green lint. One final applicable component gate remains with the original coder after the confined binding correction.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Record-only disposition at cbcc2089ba5a61e01bf10ec2773a13040f0d2a7d accepts the in-lane author receipt: all eight new replay bindings now name FR-237, while the existing Rpc bindings and runtime behavior remain unchanged. |

No code reread, additional semantic review, matrix or runtime was performed for this metadata-only correction. The original finding remains recorded above. QSL-2689's two required scenarios remain strictly pending, not completed coverage; the original coder runs its one final applicable component gate next.
