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

## New findings (disposition pass 1)

Round 1 was reviewed at head dda382f07f102a7e5336271c76a72619ff23f731, which is rebased on origin/main 4f2cbc21 and so includes spec PR #3603 (FR-201 on-each profile, AC-76..80; FR-875 `kind` and the "(current)" bullet).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | medium | The document shape does not follow FR-201's on-each profile. The profile says a `Ratio` document "SHALL have exactly `identity`, `objective`, `kind` and `conditional`", and a `Vacuous` one exactly `identity`, `objective`, `kind`, `values` (empty) and `conditional`. `Events::encode_into` always writes top-level `values: []` and `ranking: []` (a `Ratio` gets both extra members, a `Vacuous` gets an extra `ranking`). `WireOnEach` makes both members required under `deny_unknown_fields`, so a spec-exact document is refused | fsl-replay/src/probability_certificate/document/instances.rs:1093-1100, 1188-1196 |
| FND-015 | medium | The every-scheduler case of FR-201-AC-80 is not met. A `Ratio` document "offered for FR-875's claim under every scheduler" decodes (its objective says `workload`), then `subject()` returns `Unsupported` from `claim_workload(...)?.ok_or(..)`, so the check fails with an error, not `Malformed` at `mode`. The AC-80 test only edits the document's `objective.optimization` and never uses an every-scheduler claim. AC-80's cyclic, fairness and timed clauses are untested in fsl-probability-check | fsl-replay/src/probability_certificate/document/instances.rs:1600-1605 |
| FND-016 | medium | When a refutation needs more than `max_witness_paths` paths, `evidence` returns `ExactProbMaxWitnessPaths`, and the item settles `incomplete` for a claim that is false. FSpec FR-413 says to emit a subsystem certificate instead ("A subsystem certificate for a quantile claim or an `on each` instance carries a `lower` certificate for `V` and one for `N`"). That is unbuilt, and neither QSL-2995 nor QSL-3003 lists it | fsl-replay/src/probability_certificate/document/instances.rs:862-864 |
| FND-017 | medium | SPEC GAP: FR-875's "Out-of-profile forms (current)" bullet covers "an every-scheduler optimum, statistical evidence, a cyclic continuation graph, a timed subject, authored fairness or another form". It does not cover an undefined guard or target read in an in-profile claim, which `failure()` now settles `unsupported`/`not-yet-implemented` (QSL-3003). FR-741 says "An undefined evaluation refutes with FR-202's `Undefined` evidence". FR-875 says the item "SHALL NOT be settled `unsupported` for its activation" | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:58 |
| FND-018 | medium | SPEC CONFLICT behind FND-007. FR-186-AC-10 says the on-each item's "proof basis equals the `on origin` claim's (`391/400`)", and that `on origin` claim is `over (m: M::Msg)`, so its entry is binding `m` with instance null. FR-741 Binding says every entry "has two members: the `over` binding and `instance`, the FR-181 reference of the `on each` object ... `instance` is null for an `on origin` claim". For on-each `E`, which has no `over`, that means binding null and instance `m`, so the two bases cannot be equal | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:373 |

## Dispositions

Round 1 tests at dda382f0 (titan, JOBS=3, --test-threads=3; filters on_each::, timed_product::, bounded::, roundtrip::, then probabilistic_on_each, timed_state_graph_items_report_empty, package_probabilistic): fsl-probability-check 36 passed, 0 failed; fsl-analyze 27 passed, 0 failed, 1 ignored (QSL-2830). Both timed_product tests and the timed state-graph test pass again, so FND-001 and FND-013 are confirmed fixed.


| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The `ItemCause` `type` special case is removed (outcome.rs is no longer in the diff). FR-875 now reads `{"kind": "no-instance"}` (#3603). The test asserts `cause["kind"]`. The timed state-graph path is byte-identical to main |
| FND-002 | deferred | Limits are fixed: `failure()` sends `Graph(_)` to `graph_failure`, so a limit settles `incomplete` naming `exact_prob.max_states` (new test). Undefined guard or target reads still settle `unsupported` (`not-yet-implemented`), deferred to QSL-3003. FR-875's "(current)" bullet does not cover this, which is new FND-017 |
| FND-003 | deferred | QSL-2995. FR-875's "Out-of-profile forms (current)" bullet (#3603) now states the gap and names QSL-2995 |
| FND-004 | fixed | When no instance activates, the producer emits a `vacuous` document. The checker rejects a `Ratio` document there `InequalityFails` at the first initial key, and rejects a `Vacuous` document where activation is reachable `InequalityFails` at the first activating key (FR-201-AC-77 test) |
| FND-005 | fixed | Both parts are full `exact` documents (identity, objective, kind, values, ranking), and the profile is now FR-201 "Closed on-each reachability conditional profile". The top-level shape is still not exact, which is new FND-014 |
| FND-006 | fixed | Steps carry action, transition, draw and schedule. Paths come most probable first under `max_witness_paths`, not-activated paths are counted, and the rule is FR-413's `V > (1-θ)(1-N)`. Replay checks each step's identity, transition, draw and post-state, prefix-freeness, and `initial` against `initial_key` |
| FND-007 | still-open | When there is no `over`, entries still put the on-each object in `binding` and leave `instance` empty. FR-741 Binding wants binding null and `instance` set, as the counterexample already does (`binding` null, `instance` `m`). The coder's justification is FR-186-AC-10's basis-equality clause, a spec conflict filed as new FND-018. Needs a spec ruling, then a code change |
| FND-008 | fixed | `restrict_spelled_states` is removed. The tests now assert `counterexample.instance.identity` (`m`, `a`), which is the owner-sanctioned relaxation and matches FR-875-AC-2/3 "naming `m`" / "naming `a`" literally |
| FND-009 | fixed | One shared `Spent` (states, edges, key storage) covers every walk and the checker's rebuilt graph. `solve` no longer clones the row maps for each state. The full model states held in `models` are still not charged, which is minor |
| FND-010 | still-open | Most of it is fixed: the on_each.rs tests now assert rule and locus for AC-76 to AC-80 (InequalityFails with part, RankBroken, Malformed at the part, ThresholdMissed with no part, vacuity both ways). Still missing: FR-201-AC-76 and FR-741-AC-5's numerator `391/400` *at `m`'s initial key*, and `n`'s denominator `0`. Only the denominator at `m` and the numerator 0 on `n`'s rows are asserted |
| FND-011 | still-open | The checker now rebuilds its graph independently (`rebuilt::build`, `has_cycle`, `reaches_activation`). It still shares `Arith::expected` (the target and Bellman row semantics), `conditioned` and `meets` with the producer, so the module comment's "shares no ... row code" overclaims. The witness replay walks the producer's `roots`/`walk` and classifies not-activated paths with the producer's `activatable`. FSpec FR-413 Replay says "checking by its own graph search that a not-activated path ends where no activating state is reachable". The fix is cheap: replay over `rebuilt::build` and `reaches_activation` |
| FND-012 | still-open | The PR's own test hunks are not rustfmt-clean: probability_from_source.rs:720 and :738 (the two `instance.identity` asserts). Every other PR hunk is clean; the remaining diffs at exact.rs:358/368/1177/2987, lib.rs:441, probability_certificate.rs:1582/3324/3376 and bounded.rs:84/95 are pre-existing |
| FND-013 | fixed | `timed_product::domain()` now clears `modifies_receiver` on both operations, so the `ConflictingBinding` refusal is gone. The coder is to re-run it |
| FND-007 | fixed | Round 2 (466cc8a6): `entry_binding` returns the `over` object, or null with no `over`, and always the on-each object as `instance`. `ExactEntry.instance` is always serialized, so `on origin` entries read `"instance": null`, as FR-741 Binding states. `ExactEntry` is Serialize-only, and no reader, wire golden or schema in the repo reads `entries`; FSpec's `exact-value` basis schema has no `entries`. The test follows the amended FR-186-AC-10 |
| FND-010 | fixed | Round 2: on_each.rs asserts `391/400` and `1` at `m`'s initial key and `0`/`0` at `n`'s initial key, with keys found by start state and not by value. The analyze test asserts one entry with `instance` `m`, `binding` null and value 391 |
| FND-011 | fixed | Round 2: witness replay classifies not-activated paths with `rebuilt::reaches_activation` over `rebuilt::build`, which is FSpec FR-413 Replay's "own graph search". It still matches each step's action, transition and draw against the producer's walk edges. Those come from `moves()`, the model's native weighted-step relation, so this is the "native transition relation" FR-413 names and is acceptable. The module comment now states the shared row semantics honestly |
| FND-012 | fixed | Round 2: the PR's own hunks are rustfmt-clean. What remains is block-level in lists and macros that are already unformatted on origin/main (lib.rs:485 and bounded.rs:296 report diffs on main itself), and the PR rightly did not reflow foreign lines |
| FND-014 | fixed | Round 2: a `Ratio` document has exactly identity, objective, kind and conditional. A `Vacuous` one adds `values: []`. `WireOnEach.values` is optional, and decode requires its absence in `ratio` and an empty array in `vacuous`. The test checks the member sets, a hand-written spec-exact document being accepted, and extra members being refused |
| FND-015 | fixed | Round 2: `free_scheduler` runs before `subject()` and rejects a `Ratio` for an every-scheduler claim as `Malformed` at `mode` (a new test uses a real every-scheduler claim). The cyclic and fair clauses are tested both ways: no document is written, and an in-profile document is refused. The timed clause is tested on the written half only. The accepted half holds by code (`subject()` refuses a timed subject before any arithmetic). That is enough for AC-80 at low risk, and a one-line test would close it |
| FND-016 | still-open | No change: refuting beyond `max_witness_paths` still settles `incomplete` (instances.rs `paths_limit`), with no FSpec FR-413 subsystem certificate for an on-each instance. No ticket was found: QSL-2995's list has no such item, and QSL-3003 to 3018 do not cover it. File a ticket and name it in FR-875's "(current)" bullet, or build it |
| FND-017 | fixed | Spec #3611/#3613: FR-875's "Unbuilt on-each settlements (current)" now says "An in-profile on-each claim whose guard or target read is undefined at a reachable state of an instance's phase graph likewise settles `unsupported` ... until QSL-3003" |
| FND-018 | fixed | Spec: FR-186-AC-10 now reads "its proof basis carries the exact value (`391/400`) of the `on origin` claim ...; its FR-741 `entries` row names `m` as its `instance` with a null `binding`, where that claim's row names `m` as its `binding`". The test follows it |
