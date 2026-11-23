---
id: SR-3599001
title: "code-review (with rust-review lane) of PR3599 settle each instance of a probabilistic on-each claim"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3599 5a77ef036cf4df145c532e41e0be3f586d124812; fsl-replay/src/probability_certificate/document/instances.rs, fsl-replay/src/probability_certificate{,/document,/document/key,/document/measure,/witness}.rs, fsl-replay/src/{lib,outcome,proof_result}.rs, fsl-replay/src/outcome/counterexample.rs, fsl-analyze/src/analyze/{document,exact}.rs, fsl-analyze/src/analyze/exact/instances.rs, fsl-eval/src/simulation/model/state_predicate.rs, fsl-semantics/src/check/probabilistic/claim.rs, tools/fsl-probability-check/tests/{it/on_each.rs,it/bounded.rs,fixtures/link.semantic-ir.json}"
review_set: subset
---

## Summary

Ticket: QSL-2979. Review of `git diff origin/main...HEAD` (28 files, +1550/-26). The change
settles an `on each` probabilistic claim (unbounded `eventually`, a workload with no choice,
untimed, no fairness) through analyze. It builds one phase graph per (initial state, `over`
instance, `on each` instance), with the activation phase in the FR-196 product key. It solves
`Pr(E and act)` and `Pr(act)` exactly in rationals over the acyclic graph and emits an FR-201
`conditional`/`ratio` document under a new profile. A checker rebuilds the graphs and verifies
each row against its successors. A refutation is a path set naming the instance, replayed against
the rebuilt graph. The rust-review lane is folded in. Tests were run filtered, one crate at a
time, with JOBS=3 and `--test-threads=3` (see Verdict).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `ItemCause::inconclusive` moves every `InconclusiveCause::NoInstance` from `kind` to `type`, the state-graph routes included. The outcome document now uses two discriminators, and `timed_state_graph_items_report_empty_instance_and_initial_causes` (it asserts `cause["kind"] == "no-instance"`) now FAILS at head (timed_state_graph.rs:349: left Null, right "no-instance"; the record reads `"cause":{"type":"no-instance"}`). Spec defect: FR-875 says `{"type": "no-instance"}`, but FR-286/FR-291-AC-1 and `ItemCause` spell outcome-document causes in `kind` | fsl-replay/src/outcome.rs:908 |
| FND-002 | high | Through the on-each route, a state or edge limit (`ExactProbabilityGraphError::LimitReached`/`EdgeLimitReached`), `RequiresBound`, an extent limit, a refused expansion and an undefined guard or target read all reach `document_failure`, whose `Graph(_)`/`UndefinedPredicate` arm is `broken()`. They settle `failed` (internal failure), not `incomplete` `LimitReached` naming `exact_prob.max_states` (FR-741 Budgets), `requires-bound`, or FR-203/FR-741's undefined-path refutation | fsl-analyze/src/analyze/exact/instances.rs:203 |
| FND-003 | high | FR-875 behaviours named in the ticket are not built and have no ticket. Under every scheduler or a workload with free delays (optimum by the reset construction, `Optimum` certificate, FR-201-AC-52/53, TC-3061), under statistical evidence (`measured`), for a cyclic phase graph, and for forms other than unbounded `eventually`, timed claims or fairness sets, the item settles `unsupported` (`not_yet`). FR-875 says "The item SHALL NOT be settled `unsupported` for its activation" | fsl-replay/src/probability_certificate/document/instances.rs:88-108 |
| FND-004 | medium | When every instance is vacuous, the producer emits a `ratio` document and the checker returns `AcceptedVacuous` for it. FR-201 says that when no initial key has reachable activation the checker "SHALL reject the numeric `Ratio` certificate `InequalityFails` at the first initial complete product key", and the claim is certified "only through an accepted `Vacuous` conditional certificate". `{mode: "vacuous"}` is neither produced nor checked | fsl-replay/src/probability_certificate/document/instances.rs:1067 |
| FND-005 | medium | The certificate wire form is not FSpec FR-412/FR-201's. Each `numerator`/`denominator` part is a bare `{values}`, not "an `exact` certificate of this form" (no `kind`, `ranking`, identity or objective), and the profile `filament.probability-objective/on-each-reachability/v1` is defined by no requirement. A part whose `kind` is not `exact` cannot be rejected `Malformed` at the part, and a misplaced mode is `Unsupported` instead of `Malformed` at `mode` | fsl-replay/src/probability_certificate/document/instances.rs:771-865 |
| FND-006 | medium | The refutation path set does not follow FSpec FR-413. Its steps carry `transition`, `action` and `draw` null (FR-413 asks for "FR-181 trace step content plus each step's drawn random vector"), and it holds no not-activated paths. The bound used is the recomputed `(1-θ)·Pr(act)` instead of FR-413's self-contained `V > (1 − θ) · (1 − N)`, so an FR-413 replayer reading the evidence alone cannot reproduce it when `Pr(act) < 1`. There is no `max_witness_paths` cap or most-probable-first order, and replay does not compare `initial` with `initial_key` | fsl-replay/src/probability_certificate/document/instances.rs:596-674 |
| FND-007 | medium | `entries` puts the `on each` instance in `ExactEntry.binding` ("the bound instance, absent when the claim has no `over` parameter") when there is no `over`, and drops it when there is one. FR-741 Binding requires `binding` (the `over` binding) and `instance` as two members, and "Two instances of one `over` binding are two bindings, each with its own ... entry" | fsl-replay/src/probability_certificate/document/instances.rs:56-65 |
| FND-008 | medium | `restrict_spelled_states` cuts every population of the spelled counterexample `state` to the failing instance's object, including the `over` object and objects of other types. The spelled state then differs from the state its sibling `key` encodes, which breaks `spell_states`' own contract ("the state it names ... read back from the key"). The cut exists only to satisfy a test that asserts more than FR-875-AC-2 says | fsl-analyze/src/analyze/document.rs:4450-4477 |
| FND-009 | medium | Resources are not bounded across the product. `limits.states`/`limits.edges` and key storage admission reset for each (initial × over × instance) walk, so N instances get N times the budget, and the checker merges every walk's graph with no total ceiling. `solve` clones the whole numerator and denominator maps for every state (`let rows = rows.clone()` inside the `from` closure). That is O(n²) unmetered work with no cancel poll during the clone | fsl-replay/src/probability_certificate/document/instances.rs:269-282, 452-461 |
| FND-010 | medium | The checker's negative tests are thin. `on_each.rs` checks only that an altered first row exits nonzero. No test asserts the rule and locus (`InequalityFails` with `part`), a wrong vacuity claim, an extra or missing row (`Malformed` at `values`), `ThresholdMissed` or an identity mismatch. FR-741-AC-5's "391/400 and denominator 1 at `m`'s initial key, `n`'s confirmed vacuous" is asserted only as "some row holds this value". | tools/fsl-probability-check/tests/it/on_each.rs:43-72 |
| FND-011 | low | Independence: the checker reuses the producer's `roots`/`walk`/`postorder` graph code. FR-201 and FSpec FR-412 say the checker rebuilds the activating sets "by its own graph code". Value checking is independent (local equations, no `solve`), and the graph is rebuilt from the subject, not read from the certificate. Vacuity is confirmed by the denominator equations, not by the reachability search FR-201 names | fsl-replay/src/probability_certificate/document/instances.rs:971-981 |
| FND-012 | low | The PR's own hunks are not rustfmt-clean: lib.rs:486-496, outcome.rs:910, probability_certificate/document.rs:17-20, document/key.rs:401, fsl-semantics check/mod.rs:212, check/probabilistic.rs:35, fsl-probability-check tests/it/bounded.rs:203 | fsl-replay/src/lib.rs:493 |
| FND-013 | high | The shared fixture edit breaks two consumers the coder did not run. `timed_product.rs::domain()` rewrites `operations[*].frame.modifies` but leaves the new `modifies_receiver`, so intake refuses `InvalidModelBinding`/`ConflictingBinding` ("`ix://test/link/Msg/delivered` is scoped to the receiver here and to every object at ..."). `timed_product_certificates_pass_the_separate_checker` and `elapsed_quantile_certificate_passes_the_separate_checker` both FAIL at bounded.rs:116 | tools/fsl-probability-check/tests/fixtures/link.semantic-ir.json:1 |

## Verdict

NOT MERGEABLE. FND-001, FND-002 and FND-013 are wrong results or test regressions caused by this PR. FND-003 is
specified behaviour named in the ticket's own items 4 and 5 that is unbuilt and has no ticket.
FND-004 to FND-010 are spec-conformance, resource and coverage gaps to fix in this PR or to
ticket explicitly.

Soundness of what is built: the claim value is right. Rows are exact rationals (`Arith` over
`filament_exact` with a meter and scalar check), and the ratio `Pr(E and act)/Pr(act)` is an
exact `Divide` at each root. Vacuity (`Pr(act)=0`) is never a refutation and never
inconclusive. Each root is skipped when it has no value, and `meets` is inclusive for
`AtLeast`/`AtMost`. The target is read only in `active` states, so an event before activation
does not count. No instance gives `NoInstance` before any walk, and the instance and `over`
loops do not walk when a population is empty. Keys are ordered by BTreeMap, instances in
population key order, and the first failing root is returned, so a rerun names the same
instance (test-confirmed). On an acyclic graph the checker's local equations have a unique
solution, so a tampered numerator, denominator or vacuity claim fails `InequalityFails` at
that key and part, and a row set that differs from the rebuilt keys fails `Malformed` at
`values`. A refutation is a recomputed path-set mass over the rebuilt graph. It is sound
(distinct trails to terminal violating states are prefix-free) but not FR-413-shaped
(FND-006). Cancel is polled in every loop.

Disclosures adjudicated:
(1) `type` versus `kind`: FSpec FR-331 spells a provider entry's cause as a bare `type`
(FR-331-AC-51, line 852 "Each of the six causes below is a bare `type`"). FSL's outcome
document spells the same causes in `kind`: FR-291-AC-1 says "the cause `{"kind": "tool-failure", "defect": D}`,
FR-331's `tool-failure{defect}` in the outcome document's spelling (FR-286)", `ItemCause` is
documented "its FR-331 spelling in `kind`", and 40 analyze test asserts read `cause["kind"]`,
`no-instance` among them in timed_state_graph.rs:349. FR-875 copied the provider spelling
`{"type": "no-instance"}`. SPEC DEFECT: FR-875 Behavior "cause `{"type": "no-instance"}`"
contradicts FR-291-AC-1 and FR-286. The test's `cause["type"]` is wrong for the outcome
document, and so is the code's special case (FND-001). Fix FR-875's text and the test to
`kind`, then drop the special case. Changing the test needs the owner's say-so, because the
ticket forbids it.
(2) Full state or cut state: neither FR-621 nor FR-875 says to spell the full state beside
each key. FR-621's only rule is "Every value in a counterexample state ... SHALL use FSpec
FR-181's typed canonical forms". The full-state rule is the code's own `spell_states`
contract. FR-875-AC-2 says "its counterexample naming `m` and not `n`", which is naming the
instance (FSpec FR-413 `instance`; FR-741 "names the failing instance in its binding's
`instance` member"). The tests assert `!counterexample.contains("\"n\"")` over the whole
serialized counterexample. That is stronger than the AC and forces the state cut. The spec
does not contradict itself here: the test over-asserts. Recommended fix: relax the two test
asserts to the `instance` member, with owner approval, and drop `restrict_spelled_states`
(FND-008).
(3) Fixture edit: legitimate. `link.native`'s posts constrain only `self`
(`self.attempts - pre(self.attempts) = 1 and self.delivered = (not lost)`), so the intended
frame is receiver-scoped (FR-154 `modifies_receiver`, "`modifies self.f`, grants writes to field `f`
on the operation's receiver only"). The old type-wide `modifies` left the other Msg free with
two objects, which made the model non-Markov. So the edit is right in intent. But it is NOT
neutral for other consumers. `timed_product.rs` rewrites the fixture's `frame.modifies` and now
collides with the new `modifies_receiver`, so its two tests fail (FND-013). Fix the fixture's
consumers in this PR, by clearing `modifies_receiver` in `timed_product::domain()` or rewriting
it there. Every other consumer re-run passed (below).
(4) The unbuilt items are SPECIFIED by FR-875 and named in ticket items 4 and 5 (FND-003). The
refusal is honest: every one settles `unsupported`, never a wrong `proved` or `refuted`
(`subject` refuses `Every`, timed and fairness claims and non-`Eventually` forms; a choice
with more than one distribution refuses; a cycle maps through `Graph::Cycle` to `Unsupported`).
The limit and undefined paths are not honest (FND-002). Counterexample `transition` null
(FND-006) is specified by FSpec FR-413.
(5) See FND-011 and the soundness paragraph.
(6) The five target tests: the diff touches only their five `#[ignore]` lines, and every
assertion is unchanged. The checker's negative tests are thin (FND-010). Formatting is in
FND-012.

Tests run at head 5a77ef03 (titan, one crate at a time, JOBS=3, --test-threads=3):
- fsl-probability-check `--test it` filtered `roundtrip:: bounded:: timed_product:: bounded_fair:: on_each::`: 33 passed, 3 failed. The failures are `timed_product::timed_product_certificates_pass_the_separate_checker` and `timed_product::elapsed_quantile_certificate_passes_the_separate_checker` (both pass at merge-base 0cd30408: PR REGRESSION, FND-013), and `bounded::bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document` (Incomplete LimitReached{ExactProbMaxRationalBits} where PrecisionBudget is expected). That one also fails at merge-base 0cd30408, so it is PRE-EXISTING and not this PR's. `on_each::an_on_each_conditional_certificate_is_accepted_and_each_row_is_checked` passed, and so did all 18 roundtrip, 5 bounded_fair and 9 of 10 bounded tests.
- fsl-analyze `--test it` filtered: the five `probabilistic_on_each_*` tests pass, as do 11 package_probabilistic_{refutation,parity} tests (1 ignored, QSL-2830). `timed_state_graph::timed_state_graph_items_report_empty_instance_and_initial_causes` FAILS (FND-001).
- fsl-replay `--lib probability_certificate`: 52 passed, 0 failed (these include the `link.model-key-0/1.json` fixture tests `bounded_model_product_key_uses_the_complete_fr196_v1_tuple` and the document codec tests over the changed `admit_parts`).
- Not run: the root `tests/it/probability_source_routes.rs` (a full workspace build; titan load).

## Scope and evidence

- FR-875 (spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md): examined, Behavior in full.
- FR-875-AC-1, FR-875-AC-2, FR-875-AC-3: examined.
- FR-741-AC-5 and FR-741 Binding (spec/functional/FR-741-request-and-report-probability-items-through-analyze.md): examined.
- FR-196 "Conditioned readings" (spec/functional/FR-196-build-the-probabilistic-product.md): examined.
- FR-201 "Conditional certificates" (spec/functional/FR-201-check-a-probability-certificate.md): examined.
- FR-621 (spec/functional/FR-621-...md): examined for state spelling.
- FR-286, FR-291-AC-1, FR-290 cause table: examined for the cause discriminator.
- FR-154: examined for the fixture frame.
- FSpec FR-331, FR-412, FR-413 (agent-ix/filament-specification): examined.
