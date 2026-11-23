---
id: SR-967175026302197768900474181941935084771
title: "EARS review of PR 3546 cyclic always/until refutation spec"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR 3546 d54fb98379243552ea60685fb4f62ecf23622e6b; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/test-cases/TC-3162..TC-3165"
review_set: subset
---

## Summary

Ticket: QSL-2923. Checked the new and edited requirement statements in FR-202 and FR-201 for EARS form. The statements follow the repo's prose-SHALL style; two are compound or carry normative content without SHALL.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | One bullet carries five obligations (run admission first; do not start the search; emit the certificate with the scheduler rule; emit no path set; do not settle unsupported) plus an unkeyworded 'the max_witness_paths rule above does not apply'. Split into a trigger statement ('When ... has a cycle, EN-5 SHALL ...') and separate prohibitions, and make the max_witness_paths exemption a SHALL. | spec/functional/FR-202-replay-a-probabilistic-witness.md:119-131 |
| FND-002 | low | Normative checker conditions (union value 1, lower inequalities, ranks) are stated without SHALL, unlike the neighbouring sentences. Write 'The checker SHALL require value 1 at every union state and SHALL apply ...'. | spec/functional/FR-202-replay-a-probabilistic-witness.md:263-266 |

## Verdict

FR-201 edits are informative pointers that add no obligation, which is correct for them. The path-set entry edit is a well-formed SHALL statement.

## Scope and evidence

No applicable enforced AssuranceProfile was found; review set is the subset named by the dispatch (integrity, ears, evidence, failure-domain). Spec-only PR; no cargo was run.

- FR-202 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): For an unwindowed `eventually holds(E)`, `always holds(P)` or `holds(A) until holds(B)` claim under `INFINITE-TRACE`, EN-5 SHALL run the path-set entry's original-graph acyclicity admission (Acyclic infinite-trace state-predicate path sets below) before any path-set search, for `always` and `until` as for `eventually`. When the original reachable graph has a cycle apart from implicit terminal stutter, EN-5 SHALL NOT start the path-set search and the `max_witness_paths` rule above does not apply. With the engine's value past the threshold, EN-5 SHALL emit the event-avoidance `lower` subsyste...
- FR-202 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): For `always holds(P)` and `holds(A) until holds(B)`, the event-avoidance check SHALL use the same machinery over the complete reachable finite induced chain, reading each state predicate with the actual checked occurrence, infinite-trace profile and binding, and absorbing each decided state as the acyclic state-predicate subset below decides it: for until, a B state accepts even when A is false and a state satisfying neither A nor B rejects; for always, a not-P state rejects, including before a later P state. A genuine model terminal keeps its state predicates on its infinite stutter. The c...
- FR-202 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): For the unbounded and infinite-trace subsets this entry SHALL report a cyclic subject unsupported, without a token. Such a subject's refutation is the checked-subject subsystem entry's (Emission above); EN-5 SHALL NOT settle the item `unsupported` because this entry does not admit it. The checked-subject path-set subsets SHALL report unsupported for subsystem evidence, randomized schedulers, fairness, multiple initial states/instances, timed or other claim forms outside this slice. It SHALL NOT project them onto this slice. Mean subsystem evidence uses the separate checked-subject mean-cert...
- FR-201 (spec/functional/FR-201-check-a-probability-certificate.md, examined): Admission SHALL require the existing unbounded acyclic model graph apart from implicit terminal stutter ... Cyclic, nested temporal, bounded, event-predicate and other profile/optimization/kind cases SHALL remain explicitly unsupported. ... Refutations of until and always over a cyclic subject use FR-202's checked-subject subsystem entry; they do not widen this proof profile's acyclic admission.
- FR-201 (spec/functional/FR-201-check-a-probability-certificate.md, examined): FR-202's checked-subject entry separately admits the specified untimed eventual, always, until, long-run and expected-reward subsystem refutations, including over cyclic subjects; this does not widen proof-side admission or threshold checks.
