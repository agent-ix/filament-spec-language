---
id: SR-967184
title: "QSL-3408 spec review of hyper object parameter instancing (filament-spec-language PR 4022)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@a0e8d76981b7667c64a915cc6c2e8b016843c714; spec/decisions/ADR-023, spec/functional/FR-125, FR-172, FR-176, FR-181, FR-183, FR-315, spec/test-cases/TC-3219, TC-597; context FR-128, FR-151, FR-163, FR-165, FR-169, FR-182, FR-621, ADR-018, ADR-021; FSpec agent-ix/filament-specification@691ac5a6748edc324f014e26ca8370eb15b127d5 FR-361, FR-395, FR-399, FR-401"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3408. PR filament-spec-language#4022, head
a0e8d76981b7667c64a915cc6c2e8b016843c714, spec only. FSpec read at the pinned
revision 691ac5a6 (Cargo.toml `filament-specification` rev).

Compared against FSpec: FR-125's hyper bullets and ADR-023 HS-3 match FSpec
FR-361 "Instancing" and FR-395 "Object parameters" on the admitted-key set,
the population set (every population that overlaps `T`), deduplication of a
key several populations list, excluded keys, written binding type and the
outcome order (`requires-bound`, `no-initial-state`, `conflicting-identity`,
`no-instance`). The one textual divergence is the no-overlapping-population
case (FND-001). The "first parameter varying slowest" product order is FSL's
own; FSpec is silent.

Expected values of the new criteria were recomputed by hand. FR-125-AC-19
(instances `d`, `a`; excluded `[u]`; refuted at `a` typed `Switch`;
disjunction proved), FR-125-AC-20, FR-172-AC-10, FR-176-AC-6, FR-181-AC-8 and
AC-9, FR-183-AC-13 and FR-315-AC-59 are right by FR-361's rules, and each
fails under the old universe-key reading or a single-population reading.
Where a criterion is wrong or cannot be tested, it is listed below.

Author choices: the FR-315 empty-initial-list carve-out is accepted, because
FSpec FR-399 itself settles an inductive HP-2 proof over a subject with no
initial state `no-initial-state` "whatever tuple comes with the proof", so an
identity must exist. The FR-183 replay check is accepted. It is stricter than
FSpec FR-401 and FSL FR-128 and agrees with FR-361 ("a refuted item never
names an excluded key"). "Excluded keys are carried by no verdict member" is
right for verdict members (FSpec: "without a further wire member"), but its
extension to evidence and certificates is wrong (FND-002).

## Verdict

CHANGES REQUESTED. FND-001, FND-002 and FND-004 block this PR. FND-003 is a
cheap fix and should go in the same edit. FND-005, FND-006 and FND-008 need
Must-fix tickets. FND-007 and FND-009 to FND-013 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-172 keeps an S3 refusal for a parameter whose type no population overlaps. FSpec FR-395, FR-125 temporal step 4 and the arch ruling all state `NoInstance`, and the refusal has no SHALL and no code. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:107 |
| FND-002 | high | "No verdict member, evidence or certificate carries them" and AC-20's "no evidence or certificate names `z`" cannot hold. `make` creates the object under `z` in reachable states, so witness paths and closure certificates carry it. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:253-258,385 |
| FND-003 | medium | FR-181-AC-8 asserts that `settle_existential` "refuses the instance `m`", but no SHALL in FR-169 or FR-181 states that refusal or its code. | spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md:172 |
| FND-004 | medium | No criterion covers the universal (HP-2) package route. `package_hyper.rs` runs `check_hyper` and resolves replay instances with `hyper_instances(&parameter.population)`. | spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md:147 |
| FND-005 | medium | FR-621 "Items over several claim instances" states one instance per universe object, ordered in FSpec FR-181 key order, which contradicts FR-361's admitted keys and normative population-then-list order. No FSL text states multi-instance aggregation for hyper items. | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:225-237 |
| FND-006 | medium | FR-151 "Instances" and ADR-021 SYM-6 partition the universe into orbits and check the least key. A symmetry class of unadmitted keys then yields an instance at an excluded key, and HP-2 key symmetry uses this rule (ADR-023 PH-2). | spec/functional/FR-151-admit-a-request-s-symmetry-declarations.md:103 |
| FND-007 | low | ADR-018 QS-5 and FR-165 still say "one instance per object of the (parameter's) universe". | spec/functional/FR-165-check-state-graph-claims-at-s3.md:137 |
| FND-008 | medium | `hyper_route::single` returns the first instance that is not proved. A refutation at a later instance is reported as inconclusive, and zero instances is a `Failed(BROKEN)` fault. | fsl-analyze/src/analyze/hyper_route.rs:298-353 |
| FND-009 | low | FR-125-AC-19 does not state that the model has no operation or which profile applies, and it uses `a` both as a key and as the trace variable. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:384 |
| FND-010 | low | FR-125-AC-21's NoInstance variant does not state a non-empty initial list. Its two-alias variant cannot apply to the single-existential form the AC says each variant runs on. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:386 |
| FND-011 | low | The "first parameter varying slowest" product order is normative, but no criterion exercises a two-parameter order. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:251 |
| FND-012 | low | FR-181-AC-10 says "as the package route settles it today", which is time-relative wording in a criterion. | spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md:174 |
| FND-013 | low | FR-128 temporal replay has no counterpart to the new FR-183 check: it does not refuse an `over_binding` at an excluded key. | spec/functional/FR-128-replay-a-model-counterexample.md |

## Coverage

- Criteria examined: FR-125-AC-15, AC-19, AC-20, AC-21; FR-172-AC-1, AC-10;
  FR-176-AC-6; FR-181-AC-8, AC-9, AC-10; FR-183-AC-13; FR-315-AC-59; TC-3219;
  TC-597 step 1.
- Statements examined: ADR-023 HS-3; the FR-125 hyper instancing bullets;
  the FR-172 object-parameter bullets; the FR-181 no-instance bullet; the
  FR-183 replay checks; the FR-315 `instance-binding` rule.
- Old-rule search: grep over the 143 spec files that mention hyper or object
  parameters, plus ADR-018, ADR-021, FR-151, FR-165 and FR-621.
- `quire validate` passes on all nine changed files.
- code-review was not run: the diff has no code. FND-008 is from reading
  code, at the brief's request.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | low | FR-181-AC-11's third variant ("a universe holding only keys no initial state admits") has no fixture of its own. Read with the stated initial state, which holds `p` and `q`, those records sit outside the universe, and that is an admission refusal, not `no-instance`. | spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md:187 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-015 | low | FR-177 "Instances" lets the engine skip later instances after the first refuted one but does not require it, so exploration statistics and any shared-limit stop on a later instance differ between conforming engines. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:174-176 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | low | No criterion covers FR-621's single-outcome sentence: an HP-2 or HP-3 engine whose one returned refutation fails replay settles `replay-parity`. FR-621-AC-24 exercises only per-instance state-graph records. | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:257 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-017 | low | Step 3 of "The proved record over several instances" takes "the smallest depth or horizon" across `bounded-complete{depth}` (steps) and a `bounded-solver` horizon (FSpec FR-193, continuous time). Those units do not compare. Say that only bases of one kind compare, or that one item's instances never mix the two. | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:277 |
| FND-018 | low | FR-621-AC-25 has no case for the author's two choices (an `inductive{depth}` basis is not depth-bounded; a bounded basis wins over inductive and witness bases) or for a state-constrained run in step 4. | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:375 |

## Dispositions

Round 1, head 29b21bd1b99313f9c760544b08da617557f39721.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-172 now says S3 SHALL NOT refuse a parameter no population overlaps, and FR-125 reports NoInstance; FR-172-AC-11 and FR-176-AC-7. |
| FND-002 | fixed | The FR-125 bullet and AC-20 now limit the claim to verdict members and instance bindings, and allow evidence and certificate states to hold z's object. |
| FND-003 | fixed | FR-181 now states the `ProductClosureRefusal::Input("HP-5-instance")` refusal; AC-8 asserts it. |
| FND-004 | fixed | FR-176-AC-7 covers the universal package route. |
| FND-005 | fixed | FR-621 now gives admitted keys, FR-125 order and the combining rule; FR-621-AC-23. |
| FND-006 | fixed | FR-151 and ADR-021 SYM-6 now partition the admitted keys; FR-151-AC-5. |
| FND-007 | fixed | FR-165 and ADR-018 QS-5 now say admitted keys. |
| FND-008 | deferred | Code defect; the QSL-3408 code PR follows this spec PR. FR-181-AC-11 and FR-125-AC-21 now assert the fix. |
| FND-009 | fixed | AC-19 states the profile and no operation, and renames the trace variable to `t`. |
| FND-010 | fixed | AC-21 states the non-empty initial list and limits the two-alias variant to the two-universal clause. |
| FND-011 | fixed | FR-125-AC-22. |
| FND-012 | fixed | AC-10 drops "today"; a SHALL bullet in FR-181 states the package outcome. |
| FND-013 | fixed | FR-128 refuses a non-instance over_binding; FR-128-AC-17. |

Round 2, head d89b4cf82ee6ead81221b4ee5bf28bdd9ffef6f4.

| FND | outcome | reason |
| --- | --- | --- |
| FND-014 | fixed | AC-11's third variant names universe `[r]` and an initial state holding no Counter. |

Round 3, head 91ecf8672dfd7dd4826037b6282013f2f127843d.

| FND | outcome | reason |
| --- | --- | --- |
| FND-015 | fixed | 97b1832f9b: FR-177 now says the statistics SHALL count only the instances the engine explored, which makes the records definite for each engine. |

Round 4, head 1b3f884a598ecac83500db1f592ae18a4496995b. No finding was open from earlier rounds other than FND-016 (low), which this head does not address.

| FND | outcome | reason |
| --- | --- | --- |
| FND-016 | still-open | 1b3f884a59 adds no single-outcome replay-parity case; low, accepted-no-change is acceptable. |
