---
id: SR-202611210022442313144108501
title: "gap-analysis of PR 3279 source plan synthesis wiring (QSL-1978)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3279 head b3b6113ff55cc85fcbadb46386156a387d62e381; spec/functional/FR-1010, FR-1011, FR-1012, FR-1013, FR-1036, FR-1037, FR-710; changed source and tests of the PR"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ticket: QSL-1978. PR filament-spec-language#3279. This audit covered the PR's owning requirements: FSL FR-1010 to FR-1013, FR-1036, FR-1037 and FR-710-AC-5, plus the FSpec criteria the new tests cite (FR-650, FR-800 to FR-806, FR-823). It ran the computed matrix, the coverage report, a reverse-gap pass over the changed source, and a stub check. The audit was repository-driven, and plan completion was not assessed.

Every live FSL criterion in scope carries a tag. The tags overstate coverage in several places. The FSpec FR-800-AC-1 binders test none of its step-record content, and the implementation cannot satisfy that criterion. One FR-1036-AC-2 binder asserts the opposite of the criterion. The FR-1013-AC-2 snapshot-refusal clause is tagged, but the implementation does not meet it and the tagged test only checks the pointer. Several FSpec retags bind tests to criteria they do not verify.

## Verdict

**FAIL**: three high findings: inflated FR-800-AC-1 coverage, an FR-1036-AC-2 binder contradicting its criterion, and an FR-1013-AC-2 clause the implementation fails behind a weak tagged test. Three FR-1012 criteria are tagged only by ignored tests. They belong to separately ordered scope (the lambda-objective PR, and timed or probabilistic planning), so this PR is not asked to implement them; they are recorded under Coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FSpec FR-800-AC-1 is claimed by three tests, and none reads `Reach.steps`, a step `index`, `operation`, `arguments` or the `before`/`after` keys. The implementation cannot satisfy the criterion: `carried()` drops reference-typed parameters, so `arguments.s` refuses `missing-name`. This is coverage inflation behind a tagged criterion. | fsl-semantics/tests/it/synthesis_plan_value.rs:86, tests/it/plan_export_surface.rs:125, tests/it/synthesis_request.rs:540, fsl-semantics/src/check/plan_value.rs:142 |
| FND-002 | high | FR-1036-AC-2 says the plan candidate is typed by the `ConfigVersion` node. The new binder `a_source_plan_candidate_emits_its_record_types_and_source_regions` asserts that the candidate is typed by the generated plan record. The criterion and its tagged test now disagree, and neither the spec nor the QSL-2235-ignored binder was updated. | tests/it/synthesis_request.rs:540, tests/it/synthesis_request.rs:232, spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md |
| FND-003 | high | FR-1013-AC-2 requires the population-bound snapshot to refuse "with the snapshot's own admission refusal". The implementation flattens that refusal to `invalid_runtime_input`/`invalid-value`, and the tagged test `plan_requests_refuse_at_admission` asserts only exit 20 and the pointer, so the test cannot detect the gap. | tests/it/plan_export_surface.rs:1413, fsl-command/src/command/plan_synthesis.rs:198 |
| FND-004 | medium | `a_plan_objective_is_typed_without_an_expected_domain_type` is traced to FSpec-FR-802-AC-1, the exact decimal step sum of 0.25 with a `potential` certificate. It asserts only that `minimize Reach.length` checks. | fsl-semantics/tests/it/synthesis_plan_value.rs:108 |
| FND-005 | medium | `source_policy_acceptance_requires_closed_truth_and_preserves_vacuity` is traced to FSpec-FR-803-AC-2 (the optimal plan `inc, inc, inc, double`) and FR-803-AC-5 (certified `infeasible` under `always holds(c.value != 8)`). It checks a single-state `read_clause` and empty-population vacuity, not those outcomes. | fsl-analyze/tests/it/plan_search.rs:1170 |
| FND-006 | medium | `the_check_is_not_charged_to_the_search_work_limit` was retagged from FSpec-FR-823-AC-1 to FR-823-AC-5. AC-5 is a checker rejection with exit 30. The test asserts a validated candidate and a work-limited `incomplete`, so the new binding is wrong. | fsl-command/src/command/synthesis.rs:1736 |
| FND-007 | medium | FR-1013 Behavior 1 requires S3 to refuse an unbounded operation parameter. Nothing implements it: `plan_value::declare` has no parameter-bound check, and `carried()` admits unbounded `Integer`. No test is tagged for it (FSpec FR-806-AC-1, FR-800-AC-5). This is an implied requirement missing from code and evidence. | fsl-semantics/src/check/plan_value.rs:178, spec/functional/FR-1013-refuse-plan-declarations-and-requests-by-stage.md |
| FND-008 | low | Reverse gap: the plan/value request discriminator is the presence of `initial` (`Requested::has_initial`). No requirement states it. FR-1010 and FR-823 say the selected declaration's grammar decides. | fsl-command/src/command/synthesis.rs:669 |

## Coverage

- Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2). `quoin matrix --repo` (quoin 0.36.8) ran and reported no run evidence for every in-scope criterion, so the evidence store has not been fed.
- Plan completion: not assessed
- Criteria in scope (FR-1010, FR-1011, FR-1012, FR-1013, FR-1036, FR-1037, FR-710-AC-5), per the engine:
  - tagged: 17
  - untagged: 0
  - tagged-by-ignored-test: 3 (FR-1012-AC-3 lambda decimal step cost, FR-1012-AC-4 timed, FR-1012-AC-5 probabilistic; all separately ordered scope, not demanded here)
  - method-without-symbol: 0
- FSpec-prefixed tags bind FSpec criteria, which this repository's matrix does not compute. They were checked by hand against filament-specification `origin/main`; FND-001 and FND-004 to FND-006 come from that check.
- `quire coverage`: no untracked or stale tags in lines changed by this PR. Its `trace-id-outside-marker` diagnostics are prose references in doc comments, the repo's established convention, and not findings.
- The root plan tests run over the authored four-value model substituted for the spine model; see code-review FND-013 on the same PR. Evidence for "over the spine model" criteria is therefore evidence over that model.
- Untraced behaviours / stubs: 1 reverse gap (FND-008). No source stubs; `tests/support/plan_model.rs` compiles a real model unit. No test stubs; every changed test asserts behaviour, though FND-003 to FND-006 assert less than they claim.
- Semantic review: skipped (not opted in). Intent mismatches found during trace reading are reported above.
- Runtime: none run by the reviewer. Per the PR body, focused current-head runs are pending and Root17 has not been green.


## Disposition round 1 — current verdict

**FAIL — FND-009 is still open; exact-head runtime also pending.** All original findings receive explicit outcomes below; required staged behavior is deferred rather than counted implemented. The newly explicit FR-1036 negative obligation has no demonstrated enforcement vector; see FND-009. Frozen head e28930774c7f3c2bf2c8665bd6ca03f24383a43e. Native Codex continuation of the same reviewer role; exact native model ID unavailable. Round 1, not a new initial code review. No cargo, build, runtime, replay or aggregate gates run by reviewer. Exact-head focused 57 definitions are pending from the service. Earlier 752/af155 evidence is historical after main integration; all 5795 service evidence is discarded for source-identity mismatch. No runtime pass or mergeability claim. QSL-1978 must remain In Progress for finite/inline/Float/Quantity/native numeric/policy/remaining adapters. Named aliases preserve approved bounds and oracles; the inline producer defect remains required later.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Inflated AC tags are removed; finite step fixture now asserts index, Op, arguments, adjacent states, receiver, ordinal, replay and certificate. Reference arguments.s and the complete FR-800-AC-1 scenario remain required on QSL-1978 finite adapters; this is not full AC completion. |
| FND-002 | fixed | Contradictory generated-record oracle now agrees with corrected FR-1036 and the obsolete ignored binder no longer claims AC2. New explicit negative reader gap is tracked separately as FND-009. |
| FND-003 | fixed | Population-bound admission test now asserts native cardinality_out_of_bound/above-maximum and file/pointer; command preserves the admission record. |
| FND-004 | fixed | The typing-only test is tagged at FR-802 requirement level, not decimal-step-sum AC1. Lambda decimal sum remains ordered separate work. |
| FND-005 | fixed | The closed-reading/vacuity test is tagged at FR-803 requirement level, not optimal/infeasible positive-length criteria. |
| FND-006 | fixed | The validated/work-limited test again binds AC1; a separate disagreeing checker binds AC5. |
| FND-007 | fixed | Unbounded Integer and unsupported parameter forms now refuse at S3; no source search silently carries unbounded Integer. Precise finite-parameter origin locus remains staged QSL-1978 work. |
| FND-008 | fixed | initial presence only counts file intake; actual selected checked grammar selects the runner. Early common member validation does not select plan versus value. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | FR-1036-AC-2 now explicitly requires a wrong-grammar candidate refusal, but both bound tests only exercise positive emission/admission. Neither mutates the candidate type to the grammar with valid structural identities nor asserts invalid_package/invalid-value at candidate, leaving the new negative contract unenforced by demonstrated FSL reader evidence. | FR-1036-AC-2; tests/it/synthesis_request.rs:166; tests/it/synthesis_request.rs:547; Cargo.toml:30 |

## Dispositions (new findings, round 1)

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | still-open | Still open in round 1. Architecture requires a narrow FSL reader enforcement check and one behavioral regression in fix round 2 on this PR. Do not defer to closed QSL-2235 or count positive admission as negative proof. If complete enforcement needs contract-IR, enforce the decidable FSL boundary and explicitly retain the owning contract-IR vector on QSL-1978. |

## Round 1 coverage and evidence limits

Plan completion: not assessed

Static quire matrix output at the frozen head reports {"tagged": 17, "tagged-by-ignored-test": 3} for FR-1010/1011/1012/1013/1036/1037 and FR-710-AC-5. Three ignored-only criteria remain separately ordered lambda/timed/probabilistic scope; they are not passes. Static extraction emitted symbol collision/other diagnostics; counts describe returned rows, not a diagnostic-free repository audit. `quire coverage --scope . --json` exited 0. `quoin matrix --repo . --json` exited 1: `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. No evidence-backed complete matrix or current runtime PASS is claimed. No further matrix campaign was run. Reverse-gap recheck of corrected discriminator, parameter bounds, typed refusals, operation ordering and generated records found no additional unowned behavior or source/test stubs in these fixes; complete requirement fulfillment remains staged. Optional broad semantic review and Jev criterion-strength were skipped (not selected/no client).

## Latest gate and publication status

Latest parent-supplied service evidence at exact e28930774c7f3c2bf2c8665bd6ca03f24383a43e: command 1 rc=0, 2 passed/0 failed; command 2 rc=0, 15 passed/0 failed. Thus 17 passed/0 failed, 40 pending across five commands. No 57-pass claim. Reviewer ran no runtime command.

Linear posts, dedup and readback are held by the explicit owner rate-limit freeze until 19:38 UTC / 11:38 PST. No comments were posted this round. Exact validated comment payloads are retained locally for one later post per method; readback has not occurred. Existing has-review label was observed before the freeze and QSL-1978 was In Progress; no state/label mutation was made.

## Inherited initial-review scope (context only)

The original scope excerpts below preserve the first review context. Current dispositions above state exactly which fix was inspected; these inherited excerpts do not claim a new initial review or execution.

```yaml
scope:
- id: FR-1010-AC-1
  path: spec/functional/FR-1010-read-a-plan-request-and-write-the-plan-outcome.md
  role: context_only
  excerpt: A plan request over the spine model writes `synthesis.format` `filament.plan-outcome/v1`
    with `status`, `objective`, `plan`, `replay`, `certificate`, `certification`,
    `explored` and `limits`, and exits `0` for an `optimal` result.
- id: FR-1010-AC-2
  path: spec/functional/FR-1010-read-a-plan-request-and-write-the-plan-outcome.md
  role: context_only
  excerpt: An `infeasible` result writes `plan` `null` and exits `10`; a request with
    an unknown `synthesis` member refuses `invalid_runtime_input`/`unknown-member`
    at `/request/synthesis/<member>` and exits `20`.
- id: FR-1010-AC-3
  path: spec/functional/FR-1010-read-a-plan-request-and-write-the-plan-outcome.md
  role: context_only
  excerpt: 'Over the spine model, an `infeasible` outcome writes `objective` with
    `direction` `minimize`, `expression` `length` and `value` `null`, and `replay`
    and `bound` `null`, with no outcome member absent; an `optimal` one writes `objective.value`
    `{"type": "integer", "value": "1"}`; a `synthesis` member carrying `engines` refuses
    `unknown-member` at `/request/synthesis/engines`, exit `20`.'
- id: FR-1011-AC-1
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: context_only
  excerpt: 'A plan request whose `limits` (FSpec FR-805) is `{"max_work": 1}` stops
    the plan search `incomplete` naming `plan_search.max_work`, exit `22`; `{"max_states":
    1}` names `plan_search.max_states`.'
- id: FR-1011-AC-2
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: context_only
  excerpt: A low `accounting.work_units` names `accounting.work_units`, not a `plan_search`
    setting.
- id: FR-1011-AC-3
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: context_only
  excerpt: 'A plan request whose `limits` is `{"max_steps": 1}` refuses `invalid_runtime_input`/`unknown-member`
    at `/request/synthesis/limits/max_steps`, and `{"max_work": 0}` refuses `invalid-value`
    at `/request/synthesis/limits/max_work`, each exit 20 with no search; a request
    with no `limits` reports every `plan_search.*` limit at its published default
    in the outcome''s `limits`.'
- id: FR-1012-AC-1
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: The spine plan under the policy `eventually holds(c.versionNumber = 3)`
    is evaluated, not refused, and every reported plan visits version 3.
- id: FR-1012-AC-2
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: Two runs of one plan request give byte-identical outcome documents.
- id: FR-1012-AC-6
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: 'Over FSpec''s published `counter` fixture, `limits` `{"max_states": 2}`
    gives `incomplete` with `explored.states` 2 and `explored.work` 6 and plan `null`,
    and `{"max_work": 1}` gives `explored.states` 1 and `explored.work` 1 (FSpec FR-801-AC-8),
    each byte-identical across two runs.'
- id: FR-1012-AC-7
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: Over a model whose invariant is undetermined at every state from some value
    up, a goal reachable only through such a state gives `incomplete` with cause `invariant-undetermined`
    naming the invariant, plan `null` and no certificate, never `infeasible`; a goal
    reached before the first such state gives `incomplete` carrying that plan, never
    `optimal`; and a plan substituted through such a state is no
- id: FR-1013-AC-1
  path: spec/functional/FR-1013-refuse-plan-declarations-and-requests-by-stage.md
  role: context_only
  excerpt: A plan `domain` `Int[-1, 3]` refuses `ill_typed`/`type-mismatch` at S3;
    a goal reading `Reach.elapsed` over the untimed spine model refuses `missing_declaration`/`missing-name`
    at `elapsed`; each exits `20`.
- id: FR-1013-AC-2
  path: spec/functional/FR-1013-refuse-plan-declarations-and-requests-by-stage.md
  role: context_only
  excerpt: Over the spine model, a `synthesis` member whose `declaration` names the
    ordinary `synthesis` `Pick` over the value type `Digit` and carries `initial`
    refuses `invalid_runtime_input`/`unknown-member` at `/request/synthesis/initial`
    (FSpec FR-823-AC-4), and one whose `declaration` is `Nope` refuses `missing_declaration`/`missing-name`
    at `/request/synthesis/declaration`; a `policy` `["Nowhere"]` re
- id: FR-1036-AC-1
  path: spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md
  role: context_only
  excerpt: '`synthesis Pick using v grammar Digit domain Digit satisfies { Pick *
    Pick = 49 };` over `type Digit = Int[0, 9];` emits one `claim`/`synthesis_request`
    node with path `["Pick"]`, the six bindings `grammar`, `domain`, `candidate`,
    `objective`, `satisfies` and `symmetry` in that order, `grammar` and `domain`
    both the `Digit` node, the candidate a `value`/`parameter` node named `Pick`,
    `objective` t'
- id: FR-1036-AC-2
  path: spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md
  role: context_only
  excerpt: The plan synthesis `Reach` over `Config::ConfigVersion` with domain `Lengths`
    emits a node whose `grammar` is the `model`/`object_type` node of `ConfigVersion`,
    whose `domain` is the `bounded_domain`/`integer_range` node of `Lengths`, and
    whose candidate is typed by the `ConfigVersion` node; the emitted package is admitted
    by FSL's reader, which returns the declaration's grammar, domain, objective
- id: FR-1037-AC-1
  path: spec/functional/FR-1037-run-a-value-synthesis-from-the-run-request-s-synthesis-member.md
  role: context_only
  excerpt: 'A `synthesis` member `{"declaration": "Pick"}` over FR-754-AC-3''s `Pick`
    writes `synthesis.format` `filament.synthesis-outcome/v1`, `status` `candidate`,
    `check` `validated`, `candidate` `{"type": "integer", "value": "7"}` and `explored.visited`
    8 with every FSpec FR-823 member present, exit `0`; `= 50` writes `no-candidate`,
    `explored.visited` 10, exit `10`; `minimize Pick` with `Pick >= 3` write'
- id: FR-1037-AC-2
  path: spec/functional/FR-1037-run-a-value-synthesis-from-the-run-request-s-synthesis-member.md
  role: context_only
  excerpt: A unit holding only `Pick` runs it for the member `{}`; a unit holding
    `Pick` and `Other` refuses `{}` `invalid_runtime_input`/`missing-member` at `/request/synthesis/declaration`;
    `declaration` `Nope` and `declaration` naming the predicate `Positive` each refuse
    `missing_declaration`/`missing-name` at `/request/synthesis/declaration`; `Pick`
    with `initial` refuses `unknown-member` at `/request/sy
- id: FR-710-AC-5
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: context_only
  excerpt: A package input that supplies the AC-1 model unit under `example/shop`
    for `model M = "example/shop";` admits it as FR-154 admits the compiled document,
    and the checked package equals the one built from the compiled document supplied
    as bytes; supplied under `example/other` for `model M = "example/other";` it refuses
    `invalid_model_binding`/`wrong-model-selection`.
- id: FSpec-FR-800-AC-1
  path: filament-specification/spec/functional/analysis/FR-800-define-the-plan-and-goal-model.md
  role: context_only
  excerpt: 'Over a model with object type `Server` (population `n1`, `n2`) and operation
    `drain(s: Reference<Server>)`, a plan of length 2 binds `s` to `n1` and then `n2`,
    and `Reach.steps` reads two records with `index` 0 and 1, `operation` `drain`,
    `arguments.s` `n1` and `n2`, and `before`/`after` states whose keys equal the
    runtime''s states.'
- id: FSpec-FR-801-AC-1
  path: filament-specification/spec/functional/analysis/FR-801-bound-and-meter-the-plan-search.md
  role: context_only
  excerpt: 'Over FR-650-AC-1''s counter with `plan_search.max_states` set to 2, the
    result is `incomplete` with `limit-reached{limit: max_states, value: 2, setting:
    plan_search.max_states}`, and rerunning with `--limit plan_search.max_states=1000`
    returns `optimal`.'
- id: FSpec-FR-801-AC-2
  path: filament-specification/spec/functional/analysis/FR-801-bound-and-meter-the-plan-search.md
  role: context_only
  excerpt: With `accounting.work_units` set below the work the goal evaluations need,
    the result is `incomplete` naming `accounting.work_units`, not a `plan_search`
    setting.
- id: FSpec-FR-803-AC-2
  path: filament-specification/spec/functional/analysis/FR-803-hold-plans-to-policy-clauses.md
  role: context_only
  excerpt: 'The policy `eventually holds(c.value = 3)` is not refused: it is evaluated
    on each completed plan, so the result is `optimal` `inc, inc, inc, double` (values
    1, 2, 3, 4, 8) at length 4, the first 4-step plan in canonical order that visits
    3; neither 3-step plan to 8 visits 3.'
- id: FSpec-FR-803-AC-5
  path: filament-specification/spec/functional/analysis/FR-803-hold-plans-to-policy-clauses.md
  role: context_only
  excerpt: With the policy `always holds(c.value != 8)` every plan reaching the goal
    value 8 breaks the policy at its final state, no plan is admissible, and the result
    is `infeasible` with an accepted closure certificate.
- id: FSpec-FR-805-AC-1
  path: filament-specification/spec/functional/analysis/FR-805-read-the-plan-request-and-write-the-plan-outcome.md
  role: context_only
  excerpt: FR-650-AC-1's request writes a `filament.plan-outcome/v1` document with
    `status` `optimal`, `objective.value` 3, three steps with their arguments, four
    states, `replay.outcome` `reproduced`, certificate `potential` `accepted`, `certification`
    `certified`, and exits `0`.
- id: FSpec-FR-806-AC-2
  path: filament-specification/spec/functional/analysis/FR-806-refuse-plan-requests-with-located-causes.md
  role: context_only
  excerpt: A goal reading `Reach.elapsed` over an untimed model refuses `missing_declaration`/`missing-name`
    at `elapsed`; a `domain` `Int[-1, 3]` refuses `ill_typed`/`type-mismatch`.
- id: FSpec-FR-823-AC-4
  path: filament-specification/spec/functional/analysis/FR-823-read-the-synthesis-request-and-write-the-synthesis-outcome-document.md
  role: context_only
  excerpt: A request selecting the value synthesis `Pick` with `initial`, `policy`
    or `dwell_grid` refuses `invalid_runtime_input`/`unknown-member` at `/initial`,
    `/policy` or `/dwell_grid`; a request selecting the plan synthesis `Reach` without
    `initial` refuses `missing-member` at `/initial`; a member `engines` refuses `unknown-member`
    at `/engines` before any stage runs; under a native run's `synthesis` m
- id: FSpec-FR-823-AC-5
  path: filament-specification/spec/functional/analysis/FR-823-read-the-synthesis-request-and-write-the-synthesis-outcome-document.md
  role: context_only
  excerpt: A candidate the independent checker rejects (a checker fault injected through
    a library test seam, never a request member) writes `check` `rejected`, the checker's
    `counterexample` and the `lineage`, exit `30`, never exit `0`.
- id: FSpec-FR-802-AC-1
  path: filament-specification/spec/functional/analysis/FR-802-compute-plan-costs-exactly.md
  role: context_only
  excerpt: '`minimize Reach.steps.sum(s => let cost: Decimal[0,10000;2,2;exact] =
    if s.operation = M::Op::charge then 0.10 else 0.05 in cost)` over three steps,
    two `M::Op::charge` and one other, evaluates to exactly `0.25` of type `Decimal[0,10000;2,2;exact]`,
    with no binary rounding, and the certificate is `potential`.'
- id: FR-1012-AC-3
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: 'A decimal per-step cost objective is written exactly in FSpec FR-181''s
    typed `decimal` form (`{"type": "decimal", "coefficient": "30", "scale": "2"}`
    for `0.30`) and certified with a `potential` certificate.'
- id: FR-1012-AC-4
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: 'Over FSpec''s published `timed-tick` fixture the plan''s `elapsed` is
    `{"type": "rational", "numerator": "30", "denominator": "1", "unit": "min"}` with
    every `delay` of numerator 0 (FSpec FR-808-AC-1); over `timed-go` with guard `x
    >= 2` the result is `optimal` with one step of delay `{"type": "rational", "numerator":
    "2", "denominator": "1", "unit": "s"}` and an accepted `closure` certificate (FR-8'
- id: FR-1012-AC-5
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: Over FSpec's published `probabilistic-try` fixture the goal `Reach.final.done`
    gives `optimal` `retry` (FSpec FR-809-AC-1), and `maximize Reach.probability`
    over lengths 1 to 2 using only `try` gives `optimal` `try, try` with `objective.value`
    the Rational `3/4` (FR-809-AC-2).
```

## Final disposition round 2 — static checkpoint

**CONDITIONAL: static FND-009 fix verified; runtime and actual main integration pending.**

Frozen fix-round-2 head 89de0992412ab37f79c9e71e5616f62ea69e6e8b. This is the same independent reviewer, the second and final disposition round, not a new initial review. Original 29 findings and round-1 history remain unchanged. The only pending finding before this round was gap FND-009. Native model ID unavailable. No cargo, build, runtime, broad matrix, baseline diagnostic, code/spec edit, branch mutation, push or merge was run by reviewer.

Static recheck: the FSL ordinary, budgeted and I2 readers call the same plan-candidate record-family guard after strict IR admission. If the pinned IR rejects the raw model-typed parameter first, a projection is eligible only for the exact semantic_type pointer and matching structural locus, a parameter node and a unique claim/candidate/grammar identity join; it changes only the first pointer and preserves the shared code/cause/node/source fields. Unrelated and ambiguous refusals return unchanged. The regression emits a real plan with constant Boolean goal, admits its positive control, rekeys the candidate type operand and all exact references/source-map joins, requires the generic IR semantic_type refusal, and then asserts the FSL invalid_package/invalid-value at the exact candidate binding. No repaired bytes or false generic-IR acceptance claim. Full shared contract-IR plan-record conformance vector remains existing QSL-1978 work.

The seven native fixtures consistently use legal field number through declaration, contract, snapshot and readback. Counter.value_field defaults to legacy value, and native Counter consumers explicitly choose number. Declared intervals, canonical order, numeric expected values, receiver/ordinal, replay, certificate and accounting assertions are preserved. The arithmetic fixture uses the existing explicit Integer conversion; no grammar/profile widening.

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | fixed | Shared FSL plan-candidate record-family guard plus identity-joined exact-refusal projection; one source-emitted identity-valid mutation regression asserts candidate-binding invalid_package/invalid-value after positive and generic IR controls. Runtime evidence remains pending. |

Evidence at this static checkpoint: author-supplied exact-head corrective cargo check exited 0 in 1m57s; earlier failed compile is not a pass. Old e289 focused set completed 50 passed/7 native parser failures/0 ignored. The requested affected 10 at 89de099 have no result supplied yet. Historical 50 is not a full current-head pass. Actual new-main conflict integration is separately pending architecture repair ruling: only frames.rs conflict is reported; reviewer must inspect the final integration delta/head and affected runtime evidence before any final merge verdict. No main integration/build diagnosis was attempted by reviewer.

## Integrated final-head checkpoint

Final integrated frozen head e13100e4abbd4cb73b9029d300255a1ec7e43cca has parents 89de0992412ab37f79c9e71e5616f62ea69e6e8b and repaired main 8ba8b1bdd5fc4df61caf7ddd9ac1422203c30278; repair 5deeba7d06bfd9b99b9874a3766a302be9b0c3e2 is an ancestor. Integration review is part of this same final disposition round. The sole manual conflict preserves upstream EnabledVector.transition and its complete invocation identity in the shared apply path. Against 89, frames.rs changes only those three lines; native outcome retention and continue remain before legacy FirstResult/AllResults, with recommendations/enabled shortcuts absent in the native caller. The reader fix, negative regression and seven native fixtures are byte-unchanged; the checked_v2 canonical_bytes accessor and other accepted upstream automatic interface joins preserve this PR's paths. No new finding. No unrelated accepted-main audit. Final merge gate remains pending the one affected root3+native7 result at this exact integrated head.

## Final round 2 — authoritative disposition and gate

This final section supersedes prior checkpoint gate statements, preserving their history.

**FINAL ROUND-2 VERDICT: code approval CONDITIONAL only on the mandatory lead-owned premerge compile/current-main window. No finding-based blocker remains.** Reviewed final body head 0c00aca8d4751048f0494ff1bec379ea2aa1815c. All original 29 findings retain their round-1 outcomes (24 fixed, five accepted-scope deferrals). The sole new gap FND-009 is fixed; no later finding was added. Across all 30 finding records, latest outcomes are 25 fixed and five deferred. Original finding text and prior disposition rows are unchanged.

Actual focused runtime: at a673ebae5ead7378b78df2ad6e9a61cf92b45892, root3 passed and native6 passed, with only the single reserved-clause-name fixture failing before its assertions (combined rc101, 9 passed/1 failed/0 ignored). At 0c00aca8d4751048f0494ff1bec379ea2aa1815c, that one native definition reran with rc0, 1 passed/0 failed/0 ignored. The entire delta is one fixture string, post Set -> post Settles; model, interval, operation, postcondition body and every singleton-ordinal/wrong-receiver/replay/certificate assertion are unchanged. Thus all ten affected definitions have passing evidence, including the actual FR-1036 negative candidate-binding regression. Nine passes remain attributed to a673 and one to 0c00; this is not a full 57/58-test run at the final head. Old e289 50 passes remain historical; all 5795 evidence remains discarded. The e131 compile-stopped attempt executed zero runtime tests and is not a pass.

Mandatory premerge gate remains pending: latest SR custody must be committed first; the lead updates/true-rebases onto current origin/main if it moved, verifies any body-equivalent custody/rebase proof, and runs cargo check --locked --all-targets for filament-spec-language, fsl-analyze, fsl-command, fsl-eval, fsl-package, fsl-replay and fsl-semantics at the EXACT final custody head. Merge within ten minutes of PASS; repeat if main moves. Reviewer runs no cargo. No circular post-check SR commit is required or authorized by this report; final compile/time evidence belongs to the lead gate record.

Scoped code/Rust disposition is clear; scoped FR-1036 base and EARS wording reviews remain PASS. Gap disposition is clear within accepted staging, while full computed-matrix assurance remains CONDITIONAL: the earlier quoin matrix request was refused for accepted size; static extraction diagnostics and three ignored-only separately ordered criteria were not converted into passes. No matrix/baseline repeat or broad runtime gate was run in round2. Full QSL-1978 MUST stay In Progress for finite/reference/Enum/optional arguments, native inline bounds producer, Float/Quantity objectives, native numeric result domains, canonical positive policy monitors, object-creation universe, multi-object projection and remaining adapters. The shared contract-IR plan-record conformance vector remains existing QSL-1978 work; local FSL boundary and refusal locus now have actual negative evidence.
