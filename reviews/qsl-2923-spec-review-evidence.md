---
id: SR-967175026302197768900474181941935084772
title: "evidence review of PR 3546 cyclic always/until refutation spec"
type: SpecReview
analysis: evidence
scope: "agent-ix/filament-spec-language; PR 3546 d54fb98379243552ea60685fb4f62ecf23622e6b; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/test-cases/TC-3162..TC-3165"
review_set: subset
---

## Summary

Ticket: QSL-2923. Checked the verification method and TC binding of AC-21 and AC-36..AC-39 and the retargeting of the existing AC-21 test. Test is the right method for every AC and each new AC has its own TC; AC-21's new EN-5 clause has no test whose fixture can show it, and two certificates are underspecified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-21 now asserts that EN-5 refutes the 'initial B until / initial not-P always' cyclic fixtures through the subsystem entry and does not settle them unsupported. Its test original_cycles_are_rejected_even_beyond_initial_state_boundaries (fsl-analyze/tests/it/state_probabilistic_replay.rs:278) checks only the path-set entry (Err(Graph(Cycle))), which still matches the new path-set clause, and uses `<= 0.5` for both events: `always holds(false)` has value 0, so `<= 0.5` holds and cannot be refuted. Keep that test for the path-set clause; add an EN-5 test that uses `<= θ` for initial-B until (value 1) and `>= θ` for initial not-P always (value 0). AC-36..39 use no initial-decided fixture. | spec/functional/FR-202-replay-a-probabilistic-witness.md:638 |
| FND-002 | low | 'Giving u value 1 in the rejecting-union certificate rejects RankBroken at u' does not give u a rank. Under FR-201's rules a positive non-target state without a rank rejects RankMissing, not RankBroken. Name u's rank (e.g. 0). | spec/functional/FR-202-replay-a-probabilistic-witness.md:654 |
| FND-003 | low | The second chain's certificates (x(s)=1/2, x(n)=1 and x(s)=1/2, x(b)=1) give no ranks, unlike AC-36..38. Add rank 1 at s and 0 at n/b so the accepted certificate is determined. | spec/functional/FR-202-replay-a-probabilistic-witness.md:656 |

## Verdict

Bindings: TC-3162->AC-36, TC-3163->AC-37, TC-3164->AC-38, TC-3165->AC-39, each `verifies FR-202` with matching 'Verify FR-202-AC-NN' text and the AC table's Test (TC-NNNN) cell. AC-36's 'instrumented path-set search observes no call' is testable with a counting hook.

## Scope and evidence

No applicable enforced AssuranceProfile was found; review set is the subset named by the dispatch (integrity, ears, evidence, failure-domain). Spec-only PR; no cargo was run.

- FR-202-AC-21 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): AC-17/18 two evidence paths demanding different actions at one complete choice key fail policy consistency; a policy under a resolved workload fails replay. Distinct supported draws reaching the same post-state contribute their separate exact masses once, while repeating a draw cannot increase mass. A raw reachable cycle beyond initial B for until or initial not-P for always is not admitted to this path-set subset despite immediate absorption and yields no path-set token; EN-5 refutes such a subject through the checked-subject subsystem entry instead (FR-202-AC-36 to FR-202-AC-39) and does ...
- FR-202-AC-36 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): A two-state workload chain: `ok` satisfies P and moves to `ok` and to `down` with `1/2` each; `down` does not satisfy P and returns to `ok` with 1. `probability >= 0.9 [always holds(P)]`, no window, has exact value 0. EN-5's acyclicity admission finds the cycle, and an instrumented path-set search observes no call. EN-5 emits no scheduler and a `lower` certificate with `x(ok) = 1`, `x(down) = 1`, ranks `ok` 1 and `down` 0. Replay, using the independent checker over its own rebuilt chain, derives `{down}` as the only rejecting component, checks the certificate and `1 > 1/10`, returns matchin...
- FR-202-AC-37 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): Workload chain: `s` satisfies P and moves to `t` and to `u` with `1/2` each; `t` does not satisfy P and returns to `s`; `u` satisfies P and loops at `u`. `always holds(P)` has value `1/2`. `probability >= 1/2` and `probability <= 1/2` each settle `proved` as before, with no counterexample. `probability >= 3/5` settles `refuted` with the rejecting-union certificate `x(s) = 1/2`, `x(t) = 1`, ranks 1 and 0 (`1/2 > 2/5`); `probability <= 2/5` settles `refuted` with the accepting-union certificate `x(s) = 1/2`, `x(u) = 1`, ranks 1 and 0 (`1/2 > 2/5`). The same two certificates offered for `>= 1/...
- FR-202-AC-38 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): Every scheduler: `ok` satisfies P and has actions `stay`, to `ok`, and `leave`, to `down`; `down` does not satisfy P and returns to `ok`. For `probability >= 0.9 [always holds(P)]` the minimum is 0. EN-5 emits the deterministic memoryless scheduler `leave` at `ok` and AC-36's certificate; replay rebuilds the `leave`-induced chain and settles `refuted`. A missing or randomized scheduler is rejected without a token; the scheduler `stay` with the same certificate yields no token, since `{ok}` is then the induced chain's only component and it accepts. For `probability <= 1/2 [always holds(P)]` ...
- FR-202-AC-39 (spec/functional/FR-202-replay-a-probabilistic-witness.md, examined): Until, under a workload. Two states satisfying A and not B alternate deterministically, so `holds(A) until holds(B)` has value 0. `probability >= 1/2` settles `refuted`: the component with no decided state is rejecting, and the certificate with value 1 and rank 0 at both states gives `1 > 1/2`. In a second chain, `s` satisfies A and not B and moves to `s` with `1/2`, to a B-and-not-A state `b` with `1/4` and to a neither-A-nor-B state `n` with `1/4`; `b` and `n` each return to `s`. The value is `1/2`. `>= 3/5` is refuted with the rejecting union `{n}` (`x(s) = 1/2`, `x(n) = 1`), and `<= 2/5...
- TC-3162 (spec/test-cases/TC-3162-refute-cyclic-always-under-a-workload.md, examined): Verify FR-202-AC-36: an unwindowed always claim on a cyclic workload chain is refuted through an event-avoidance subsystem certificate, without a path-set search.
- TC-3163 (spec/test-cases/TC-3163-cyclic-always-refutation-boundaries.md, examined): Verify FR-202-AC-37: both bound sides of a cyclic always claim, and the strict comparison at equality.
- TC-3164 (spec/test-cases/TC-3164-refute-cyclic-always-over-every-scheduler.md, examined): Verify FR-202-AC-38: an every-scheduler always refutation carries and replays its deterministic memoryless witness scheduler.
- TC-3165 (spec/test-cases/TC-3165-refute-cyclic-until.md, examined): Verify FR-202-AC-39: unwindowed until over a cyclic workload chain is refuted through the event-avoidance subsystem certificate on either bound side.

## Dispositions

Round 1, fix commit ab6cad3cda1b25490823424c05fce8c90fd260ec (diff d54fb98..ab6cad3).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in ab6cad3 by hand; see after_excerpt in the Linear dispositions comment. |
| FND-002 | fixed | Verified in ab6cad3 by hand; see after_excerpt in the Linear dispositions comment. |
| FND-003 | fixed | Verified in ab6cad3 by hand; see after_excerpt in the Linear dispositions comment. |
