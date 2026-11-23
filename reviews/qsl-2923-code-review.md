---
id: SR-967175026302197768900474181941935084768
title: "code-review of PR 3546 cyclic always/until refutation spec"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR 3546 d54fb98379243552ea60685fb4f62ecf23622e6b; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/test-cases/TC-3162..TC-3165"
review_set: subset
---

## Summary

Ticket: QSL-2923. The diff (origin/main...d54fb98) changes only spec markdown (FR-201, FR-202, TC-3162..TC-3165); there is no production or test code. Checked the markdown for broken table rows, ID collisions (TC-3162..3165 are new on main), relationship targets and hash/digest/pin use: none found. Semantic spec defects are recorded by the spec-review sub-analysis artifacts of this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for markdown structure. quire validate exits 0 on all six changed files. No hash, digest, pin or tracking-record use was introduced.

## Scope and evidence

No applicable enforced AssuranceProfile was found; review set is the subset named by the dispatch (integrity, ears, evidence, failure-domain). Spec-only PR; no cargo was run.

- FR-202 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): For an unwindowed `eventually holds(E)`, `always holds(P)` or `holds(A) until holds(B)` claim under `INFINITE-TRACE`, EN-5 SHALL run the path-set entry's original-graph acyclicity admission (Acyclic infinite-trace state-predicate path sets below) before any path-set search, for `always` and `until` as for `eventually`. When the original reachable graph has a cycle apart from implicit terminal stutter, EN-5 SHALL NOT start the path-set search and the `max_witness_paths` rule above does not apply. With the engine's value past the threshold, EN-5 SHALL emit the event-avoidance `lower` subsyste...
- FR-202 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): For `always holds(P)` and `holds(A) until holds(B)`, the event-avoidance check SHALL use the same machinery over the complete reachable finite induced chain, reading each state predicate with the actual checked occurrence, infinite-trace profile and binding, and absorbing each decided state as the acyclic state-predicate subset below decides it: for until, a B state accepts even when A is false and a state satisfying neither A nor B rejects; for always, a not-P state rejects, including before a later P state. A genuine model terminal keeps its state predicates on its infinite stutter. The c...
- FR-202 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): For the unbounded and infinite-trace subsets this entry SHALL report a cyclic subject unsupported, without a token. Such a subject's refutation is the checked-subject subsystem entry's (Emission above); EN-5 SHALL NOT settle the item `unsupported` because this entry does not admit it. The checked-subject path-set subsets SHALL report unsupported for subsystem evidence, randomized schedulers, fairness, multiple initial states/instances, timed or other claim forms outside this slice. It SHALL NOT project them onto this slice. Mean subsystem evidence uses the separate checked-subject mean-cert...
- FR-202-AC-21 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): AC-17/18 two evidence paths demanding different actions at one complete choice key fail policy consistency; a policy under a resolved workload fails replay. Distinct supported draws reaching the same post-state contribute their separate exact masses once, while repeating a draw cannot increase mass. A raw reachable cycle beyond initial B for until or initial not-P for always is not admitted to this path-set subset despite immediate absorption and yields no path-set token; EN-5 refutes such a subject through the checked-subject subsystem entry instead (FR-202-AC-36 to FR-202-AC-39) and does ...
- FR-202-AC-36 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): A two-state workload chain: `ok` satisfies P and moves to `ok` and to `down` with `1/2` each; `down` does not satisfy P and returns to `ok` with 1. `probability >= 0.9 [always holds(P)]`, no window, has exact value 0. EN-5's acyclicity admission finds the cycle, and an instrumented path-set search observes no call. EN-5 emits no scheduler and a `lower` certificate with `x(ok) = 1`, `x(down) = 1`, ranks `ok` 1 and `down` 0. Replay, using the independent checker over its own rebuilt chain, derives `{down}` as the only rejecting component, checks the certificate and `1 > 1/10`, returns matchin...
- FR-202-AC-37 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): Workload chain: `s` satisfies P and moves to `t` and to `u` with `1/2` each; `t` does not satisfy P and returns to `s`; `u` satisfies P and loops at `u`. `always holds(P)` has value `1/2`. `probability >= 1/2` and `probability <= 1/2` each settle `proved` as before, with no counterexample. `probability >= 3/5` settles `refuted` with the rejecting-union certificate `x(s) = 1/2`, `x(t) = 1`, ranks 1 and 0 (`1/2 > 2/5`); `probability <= 2/5` settles `refuted` with the accepting-union certificate `x(s) = 1/2`, `x(u) = 1`, ranks 1 and 0 (`1/2 > 2/5`). The same two certificates offered for `>= 1/...
- FR-202-AC-38 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): Every scheduler: `ok` satisfies P and has actions `stay`, to `ok`, and `leave`, to `down`; `down` does not satisfy P and returns to `ok`. For `probability >= 0.9 [always holds(P)]` the minimum is 0. EN-5 emits the deterministic memoryless scheduler `leave` at `ok` and AC-36's certificate; replay rebuilds the `leave`-induced chain and settles `refuted`. A missing or randomized scheduler is rejected without a token; the scheduler `stay` with the same certificate yields no token, since `{ok}` is then the induced chain's only component and it accepts. For `probability <= 1/2 [always holds(P)]` ...
- FR-202-AC-39 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): Until, under a workload. Two states satisfying A and not B alternate deterministically, so `holds(A) until holds(B)` has value 0. `probability >= 1/2` settles `refuted`: the component with no decided state is rejecting, and the certificate with value 1 and rank 0 at both states gives `1 > 1/2`. In a second chain, `s` satisfies A and not B and moves to `s` with `1/2`, to a B-and-not-A state `b` with `1/4` and to a neither-A-nor-B state `n` with `1/4`; `b` and `n` each return to `s`. The value is `1/2`. `>= 3/5` is refuted with the rejecting union `{n}` (`x(s) = 1/2`, `x(n) = 1`), and `<= 2/5...
- TC-3162 (spec/test-cases/TC-3162-refute-cyclic-always-under-a-workload.md, examined): Verify FR-202-AC-36: an unwindowed always claim on a cyclic workload chain is refuted through an event-avoidance subsystem certificate, without a path-set search.
- TC-3163 (spec/test-cases/TC-3163-cyclic-always-refutation-boundaries.md, examined): Verify FR-202-AC-37: both bound sides of a cyclic always claim, and the strict comparison at equality.
- TC-3164 (spec/test-cases/TC-3164-refute-cyclic-always-over-every-scheduler.md, examined): Verify FR-202-AC-38: an every-scheduler always refutation carries and replays its deterministic memoryless witness scheduler.
- TC-3165 (spec/test-cases/TC-3165-refute-cyclic-until.md, examined): Verify FR-202-AC-39: unwindowed until over a cyclic workload chain is refuted through the event-avoidance subsystem certificate on either bound side.
