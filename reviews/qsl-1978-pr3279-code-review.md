---
id: SR-202611210022442313144108500
title: "code-review of PR 3279 source plan synthesis wiring (QSL-1978)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3279 head b3b6113ff55cc85fcbadb46386156a387d62e381; fsl-analyze/src/plan_source.rs, fsl-analyze/src/lib.rs, fsl-analyze/tests/it/plan_search.rs, fsl-command/src/command/plan_synthesis.rs, fsl-command/src/command/synthesis.rs, fsl-command/src/command.rs, fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/model_based_test.rs, fsl-eval/src/value/expression/mod.rs, fsl-eval/src/value/mod.rs, fsl-package/src/emit.rs, fsl-semantics/src/check/check.rs, fsl-semantics/src/check/lowering.rs, fsl-semantics/src/check/lowering/synthesis.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/plan_value.rs, fsl-semantics/src/check/synthesis.rs, fsl-semantics/src/model/intake/model_unit.rs, fsl-semantics/tests/it/main.rs, fsl-semantics/tests/it/model_unit_input.rs, fsl-semantics/tests/it/synthesis_plan_value.rs, tests/it/plan_export_surface.rs, tests/it/request_surface_s1_16.rs, tests/it/synthesis_request.rs, tests/support/mod.rs, tests/support/plan_model.rs"
review_set: subset
---

## Summary

Ticket: QSL-1978. PR filament-spec-language#3279, reviewed at its frozen head. This is the code review of `git diff origin/main...head`, with the rust-review lane (rust-style baseline and the repository's AGENTS.md conventions) folded in. It was checked against FSL FR-1010 to FR-1013, FR-1036 and FR-1037, and FSpec FR-650 and FR-800 to FR-806. FR-802 was read as clarified by filament-specification#780.

The wiring is real, not a stub. S3 types a plan record, step record, state record and operation enum. `SourcePlan` evaluates the checked goal, objective and policies under one meter. The command routes the program path to `plan_search::search` and writes `filament.plan-outcome/v1`. The package route stays an explicit typed refusal. Several paths still give a definite certified answer where the spec requires a refusal or a different value. The worst case is a floating-point objective, which yields a certified `infeasible`. In other places the implementation contradicts a requirement this PR claims (FR-1036-AC-2, FR-801 metering, the FR-806 refusal causes).

The reviewer ran nothing, as the brief requires. No cargo, test, fmt, clippy or ci command was run. Runtime status comes only from the PR body: earlier heads passed the focused tests listed there; Root17 has not been green; the current-head focused checks are pending.

## Verdict

**FAIL**: three high findings. Floating-point objectives are certified `infeasible`. FR-1036-AC-2 contradicts the new candidate typing. Medium gaps against FR-801, FR-804, FR-806 and FR-1013 remain. Runtime gates are pending, so the PR is not mergeable.

Clean areas examined: the package-route refusal (typed `not_routed` with an accurate reason, no silent fallback); value-synthesis regression on the no-`initial` path (refusal pointers and deadline unchanged); the `roles: []` producer fix (it matches every sibling producer in `model_unit.rs`); emission of the plan record, enum and source-owner rows; scope swap and restore in `Lowering::synthesis_request`; `read_clause` vacuity and closed-reading semantics; and the fault-to-exit-30 escalation after search. Duplication and vendoring: none found. `tests/support/plan_model.rs` compiles an authored model unit; it copies nothing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `Float` objective passes S3 (`is_numeric` admits `Float`), but `SourcePlan::exact` returns `None` for every non-Integer/Rational/Decimal value. Every plan's objective is then undefined, so the search settles a certified `infeasible` (general) or scores only the empty plan (per-step sum, since `is_step_sum` ignores FR-802's exact-type condition). `objective()` also writes the form as `Integer`. | fsl-analyze/src/plan_source.rs:472, fsl-analyze/src/plan_source.rs:525, fsl-command/src/command/plan_synthesis.rs:120, fsl-semantics/src/check/synthesis.rs:253 |
| FND-002 | high | FR-1036-AC-2 (unchanged in this PR) requires the plan candidate to be typed by the `ConfigVersion` node. The PR types it by the generated plan record, and the new test asserting that record is traced to FR-1036-AC-2. The non-ignored FR-1036-AC-2 test still runs `satisfies { Reach = Reach }` under `filament.model.complete/v1`, now over the plan record, and is not in the focused run list. | fsl-semantics/src/check/synthesis.rs:197, fsl-semantics/src/check/lowering/synthesis.rs:57, tests/it/synthesis_request.rs:540, tests/it/synthesis_request.rs:166, tests/it/synthesis_request.rs:232 |
| FND-003 | high | An `evaluate` wildcard arm maps `Outcome::Refused` and every `FamilyEvaluated` result to `Evaluated::Undefined`. A refused or unsupported goal or objective evaluation then reads as "goal not reached", which can certify `infeasible`. This breaks the rust-style rule against wildcards over closed enums. | fsl-analyze/src/plan_source.rs:407 |
| FND-004 | medium | FR-801 and FR-1011 require model operations, clauses and invariants to be charged to the request's evaluation meter. The plan route instead gives `ModelSystem` the default accounting (`call_accounting(&Accounting::default())`), and each system evaluation gets a fresh meter. Request `accounting` overrides and the shared budget apply only to goal, objective and policy reads. | fsl-command/src/command/plan_synthesis.rs:191, fsl-command/src/command/synthesis.rs:834 |
| FND-005 | medium | Plan expansion is unbounded before any charge. `max_candidates: u64::MAX` and `EmitLimits::new(u64::MAX)`, while `candidates()` materializes every successor before the first `max_work` charge or cancel check. S3 has no unbounded-parameter refusal (FR-1013 Behavior 1, FSpec FR-806 row 4). `carried()` even admits unbounded `Integer`. | fsl-command/src/command/plan_synthesis.rs:189, fsl-semantics/src/check/plan_value.rs:142, fsl-analyze/src/plan_search.rs:842 |
| FND-006 | medium | A policy monitor's state is an id over the whole prefix (`trace_ids` keyed by every prefix state key), not an automaton state. Under any policy no two prefixes merge, so `explored` counts and certificates diverge from FR-801's canonical exploration (FR-1012 Behavior 2), and node growth is exponential in length. Verdicts stay sound. | fsl-analyze/src/plan_source.rs:462 |
| FND-007 | medium | The FR-804 step order is not canonical. `ArgumentKey::Text` orders enum, decimal, rational, text and reference arguments by their JSON text (enum by case name, so `"10" < "9"`), contrary to its own doc. Operations are ordered by object-type key, not model declaration order. Op enum cases are alphabetical. | fsl-analyze/src/plan_source.rs:56, fsl-semantics/src/check/plan_value.rs:295, fsl-semantics/src/check/plan_value.rs:304 |
| FND-008 | medium | Step `arguments` carry only Boolean and Integer/Int parameters. Reference, enum and other parameters, and same-named parameters of differing types, are silently dropped. A read such as FSpec FR-800-AC-1's `arguments.s` refuses `missing-name` instead of a typed unsupported refusal. | fsl-semantics/src/check/plan_value.rs:142, fsl-semantics/src/check/plan_value.rs:149 |
| FND-009 | medium | Plan members are read strictly only after S1 to S3 compile, and plan versus value is decided by whether `initial` is present. `engines` or a bad `limits` refuses after the stages run (FR-1010 Behavior 1, FR-823-AC-4 "before any stage runs"). A plan request without `initial` but with `policy` or plan `limits` refuses `unknown-member` instead of `missing-member` at `/initial`. | fsl-command/src/command/synthesis.rs:663, fsl-command/src/command/synthesis.rs:669, fsl-command/src/command/plan_synthesis.rs:144 |
| FND-010 | medium | An initial snapshot the model does not admit is flattened to `invalid_runtime_input`/`invalid-value`, with a `{failure:?}` Debug dump as the reason. The snapshot's own admission refusal (FR-1013 Behavior 2, FSpec FR-806) is lost. Evaluation faults also put Debug text on the wire. | fsl-command/src/command/plan_synthesis.rs:198, fsl-analyze/src/plan_source.rs:410 |
| FND-011 | medium | Any plan domain other than a non-negative `Int` range, including unbounded `Integer`, refuses `ill_typed`/`type-mismatch` at the declaration. FSpec FR-806 requires `unsupported_projection`/`unbounded-extent` for an unbounded domain, located at the `domain` name. | fsl-semantics/src/check/synthesis.rs:172 |
| FND-012 | medium | `render` degrades silently. A state that cannot be rendered writes `plan: null` on an `optimal`/`feasible` outcome that still carries an objective and certification, exit 0. Argument text that fails to parse becomes JSON `null`. An unknown operation renders `""`. Each should be a method failure. | fsl-command/src/command/plan_synthesis.rs:326, fsl-command/src/command/plan_synthesis.rs:346, fsl-analyze/src/plan_source.rs:201 |
| FND-013 | medium | Root plan tests silently overwrite `spine-model.semantic-ir.json` with the authored four-value model whenever `initial` is `spine-snapshot.json`. ACs that name "the spine model" (FR-1010-AC-1/AC-3, FR-1012-AC-1, FR-1013-AC-2) are therefore verified on a different model under the spine's file name. The substitution is invisible at call sites, and planning over the real spine stays untested. | tests/it/plan_export_surface.rs:66, tests/it/request_surface_s1_16.rs:90 |
| FND-014 | medium | `readable()` makes any goal or objective that reads a grammar attribute (`Reach.final.versionNumber`) undefined whenever a state holds other than exactly one object of `G`. Over the two-object spine snapshot, every such goal is false and the run is certified `infeasible` (exit 10), not refused. | fsl-analyze/src/plan_source.rs:483 |
| FND-015 | low | A policy's `over` type must equal the grammar type exactly. FSpec FR-803 admits any object of `G`'s model, or the model itself, so a policy over another object type of the same model is refused `type-mismatch`. | fsl-command/src/command/plan_synthesis.rs:215 |
| FND-016 | low | The plan's operations, Op enum cases and argument fields come from every object type in the unit scope (`types.object_types()`), including types of other selected models, not only `G`'s model (FSpec FR-800). | fsl-semantics/src/check/plan_value.rs:295 |
| FND-017 | low | The Typer turns every unknown field on any `filament.plan-value` record (object, state, step, arguments) into `MissingName`, dropping `UnknownField`'s owner and span. FSpec FR-806 specifies `missing-name` only for `elapsed` and `probability`. | fsl-semantics/src/check/check.rs:5702 |
| FND-018 | low | The `"filament.plan-value"` package literal is repeated in `check.rs` and `lowering.rs` beside the private `PLAN_PACKAGE` const. Under the duplication rule this is one constant defined three times. | fsl-semantics/src/check/check.rs:5702, fsl-semantics/src/check/lowering.rs:3328, fsl-semantics/src/check/plan_value.rs:36 |
| FND-019 | low | The new code looks unformatted for the repo's `cargo fmt --check` gate. The `evaluate_plan_body` re-export line is 118 columns. `mod plan_value;` and `mod synthesis_plan_value;` break `reorder_modules` order. The new `model_unit_input` test has hand-wrapped call arguments. Not run, per the brief. | fsl-eval/src/value/mod.rs:94, fsl-semantics/src/check/mod.rs:202, fsl-semantics/tests/it/main.rs:87, fsl-semantics/tests/it/model_unit_input.rs:1041 |
| FND-020 | low | Idiom gaps. `prefix()` silently returns an empty prefix for an unknown monitor id instead of faulting. `reads` is a redundant closure over `reads_field`. `argument_fields()` reallocates for every step value. `lengths()` converts `Integer` through `to_string().parse()`. | fsl-analyze/src/plan_source.rs:417, fsl-analyze/src/plan_source.rs:144, fsl-command/src/command/plan_synthesis.rs:76 |
| FND-021 | low | The plan universe is the initial snapshot itself (`universes: vec![selection]`). An operation that creates an object can never introduce a key the snapshot lacks, contrary to FSpec FR-800 "an object created by an earlier step is a candidate argument". | fsl-command/src/command/plan_synthesis.rs:175 |

## Gates

| Gate | Status |
| --- | --- |
| `cargo fmt --check`, `clippy -D warnings`, crate tests, `cargo deny` | not run by the reviewer (forbidden by the brief); FND-019 records the likely fmt deltas |
| Focused runtime tests | per the PR body: earlier heads passed 3 plan-value checker tests, 3 policy tests and the emission/candidate regression; the current-head checks (`a_named_integer_model_type_admits_and_retains_its_bounds`, then `an_optimal_plan_writes_the_plan_outcome_document`) are pending; the remaining 15 root tests are held; Root17 has not been green |
| CI workflow diff | none in this PR |


## Disposition round 1 — current verdict

**CONDITIONAL — runtime gate pending.** All original findings receive explicit outcomes below; required staged behavior is deferred rather than counted implemented. No additional production defect identified in the fixes inspected. Frozen head e28930774c7f3c2bf2c8665bd6ca03f24383a43e. Native Codex continuation of the same reviewer role; exact native model ID unavailable. Round 1, not a new initial code review. No cargo, build, runtime, replay or aggregate gates run by reviewer. Exact-head focused 57 definitions are pending from the service. Earlier 752/af155 evidence is historical after main integration; all 5795 service evidence is discarded for source-identity mismatch. No runtime pass or mergeability claim. QSL-1978 must remain In Progress for finite/inline/Float/Quantity/native numeric/policy/remaining adapters. Named aliases preserve approved bounds and oracles; the inline producer defect remains required later.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Float/Quantity objectives now refuse by named numeric family before search; exact Integer/Rational/Decimal retain their forms. Full Float/Quantity implementation remains required on QSL-1978. |
| FND-002 | fixed | The candidate-versus-grammar contradiction is corrected in FR-1036 and both positive binders. New negative reader evidence gap is recorded separately as gap FND-009. |
| FND-003 | fixed | Refused/family-refused results now set a method fault; only semantic undefined stays undefined. |
| FND-004 | fixed | SourcePlan shares the native system evaluation meter; operations, goals and policies use the same request allowance. |
| FND-005 | fixed | Native Begin gates each invocation before evaluation, lazy finite domains replace eager source expansion, retention is occurrence-bounded before push. No raw-trial alias for MaxWork. |
| FND-006 | deferred | Positive-length policies now refuse before exploration, removing the noncanonical certified path. Canonical monitor states and positive FR-1012-AC-1/FSpec FR-803 outcomes remain required on QSL-1978 after lambda; zero-length closed reads remain supported. |
| FND-007 | fixed | Supported Boolean/finite Int arguments use typed keys; selected-model source declaration order precedes typed receiver and arguments. Sorted nominal Op membership is separate and source injection finds member by name. |
| FND-008 | deferred | Silent parameter dropping is removed by named S3 unsupported refusals and conflicting normalized-type checks. Reference/Enum/other finite and optional arguments remain required on QSL-1978 in the finite-argument adapter after lambda, not positive FR-800 completion. |
| FND-009 | fixed | Closed request members and malformed limits validate before intake; checked grammar chooses plan execution and missing initial is reported at /initial. |
| FND-010 | fixed | Admission records retain their native catalog, fields and file; evaluation fault messages no longer dump Debug payloads. |
| FND-011 | fixed | Unbounded domain components use SynthesisUnboundedDomain at form.domain.span before length typing. |
| FND-012 | fixed | Rendering returns Result and fails closed for operation/signature corruption, unrenderable state and invalid typed argument JSON. |
| FND-013 | fixed | Authored finite fixtures use explicit planning-model paths; genuine spine fixtures use the original corpus document, preserving its domain. |
| FND-014 | deferred | Ambiguous grammar-attribute projection now refuses before search for multiple/missing initial grammar objects or deleting operations. General multi-object final projection remains required on QSL-1978 with later adapters. |
| FND-015 | fixed | Policy binding compares admitted object model package identity, accepting a different object of the same model; cross-model fixture remains a typed mismatch. |
| FND-016 | fixed | The source operation inventory filters named.plan_operations by the selected model alias. |
| FND-017 | fixed | PlanUnknownField preserves owner, field and exact member span. All unknown plan fields intentionally use missing-name under the owner contract; the old cause restriction is rejected. |
| FND-018 | fixed | Both consumer checks use the single plan_value::PLAN_PACKAGE constant. |
| FND-019 | fixed | Inspected cited re-export/module/test formatting is corrected. No formatter or aggregate gate was executed by reviewer; author reports scoped formatting. |
| FND-020 | fixed | Unknown monitor ids fault, the redundant reads closure is gone, fields are cached as a slice, and lengths use Integer::to_u64. |
| FND-021 | deferred | Object-creating operations now refuse at S3 with the specific finite-key-universe form. Actual object-creation universe semantics and later created reference arguments remain required on QSL-1978 finite adapter work; no arbitrary allocator is introduced. |

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
- id: FSpec-FR-800-AC-3
  path: filament-specification/spec/functional/analysis/FR-800-define-the-plan-and-goal-model.md
  role: context_only
  excerpt: With `domain` `Int[0, 3]` and a goal true in the initial state, the result
    is the empty plan with `length` 0.
- id: FSpec-FR-800-AC-4
  path: filament-specification/spec/functional/analysis/FR-800-define-the-plan-and-goal-model.md
  role: context_only
  excerpt: A goal `Reach.final.value / Reach.final.divisor = 2` that is undefined
    on a plan whose final `divisor` is 0 does not reach the goal on that plan.
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
- id: FSpec-FR-804-AC-3
  path: filament-specification/spec/functional/analysis/FR-804-order-plans-deterministically.md
  role: context_only
  excerpt: 'With a step `move(r: Robot, c: Cell)` the plans `move(a, c1)` and `move(b,
    c0)` order by `r` first (`a` before `b`), and `move(a, c0)` before `move(a, c1)`
    by enum declaration order of `Cell`.'
- id: FSpec-FR-805-AC-1
  path: filament-specification/spec/functional/analysis/FR-805-read-the-plan-request-and-write-the-plan-outcome.md
  role: context_only
  excerpt: FR-650-AC-1's request writes a `filament.plan-outcome/v1` document with
    `status` `optimal`, `objective.value` 3, three steps with their arguments, four
    states, `replay.outcome` `reproduced`, certificate `potential` `accepted`, `certification`
    `certified`, and exits `0`.
- id: FSpec-FR-806-AC-1
  path: filament-specification/spec/functional/analysis/FR-806-refuse-plan-requests-with-located-causes.md
  role: context_only
  excerpt: 'A `domain` naming `Integer`, and an operation parameter `n: Integer`,
    each refuse `unsupported_projection`/`unbounded-extent` at S3, located at the
    `domain` name and at the parameter respectively, and no plan request is read.'
- id: FSpec-FR-806-AC-2
  path: filament-specification/spec/functional/analysis/FR-806-refuse-plan-requests-with-located-causes.md
  role: context_only
  excerpt: A goal reading `Reach.elapsed` over an untimed model refuses `missing_declaration`/`missing-name`
    at `elapsed`; a `domain` `Int[-1, 3]` refuses `ill_typed`/`type-mismatch`.
- id: FSpec-FR-806-AC-4
  path: filament-specification/spec/functional/analysis/FR-806-refuse-plan-requests-with-located-causes.md
  role: context_only
  excerpt: A request whose initial snapshot violates the model's population bound
    refuses with that snapshot's own admission cause located at `/initial`, exit `20`,
    and writes no plan.
- id: FSpec-FR-823-AC-4
  path: filament-specification/spec/functional/analysis/FR-823-read-the-synthesis-request-and-write-the-synthesis-outcome-document.md
  role: context_only
  excerpt: A request selecting the value synthesis `Pick` with `initial`, `policy`
    or `dwell_grid` refuses `invalid_runtime_input`/`unknown-member` at `/initial`,
    `/policy` or `/dwell_grid`; a request selecting the plan synthesis `Reach` without
    `initial` refuses `missing-member` at `/initial`; a member `engines` refuses `unknown-member`
    at `/engines` before any stage runs; under a native run's `synthesis` m
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
- id: fsl-analyze/src/plan_source.rs:1
  path: fsl-analyze/src/plan_source.rs
  role: context_only
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-800 to FR-803: the goal, objective and policy rules of a plan

    //! synthesis written in FSL source, evaluated over the checked package.

    //!

    //! [`SourcePlan`] is the [`PlanRules`] of one checked `synthesis` declaration

    //! whose grammar is a model object type. The declaration''s `satisfies` block

    //! and objective were typed over the plan va'
- id: fsl-command/src/command/plan_synthesis.rs:1
  path: fsl-command/src/command/plan_synthesis.rs
  role: context_only
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-650 and FR-805: the plan search behind the native-run

    //! `synthesis` member, for a declaration whose grammar is a model object

    //! type.

    //!

    //! The member is read by [`PlanRequest`], the model and the initial snapshot

    //! are admitted into a [`ModelSystem`], the declaration''s checked goal and

    //! objective judge the plan through [`Source'
- id: fsl-semantics/src/check/plan_value.rs:1
  path: fsl-semantics/src/check/plan_value.rs
  role: context_only
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-800 "The plan value": the S3 types of the plan a plan

    //! synthesis''s `satisfies` block and objective read.

    //!

    //! The declaration''s name denotes a record with `length`, `final`, `initial`

    //! and `steps`. A model state (`final`, `initial`, a step''s `before` and

    //! `after`) is a record of one collection per population of the model, each

    '
- id: fsl-eval/src/simulation/model/model_based_test.rs:3720
  path: fsl-eval/src/simulation/model/model_based_test.rs
  role: context_only
  excerpt: "/// What a temporal clause says of a run of model states (FSpec FR-803).\n\
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum ClauseReading {\n    ///\
    \ Every instance holds.\n    Holds,\n    /// Some instance is violated.\n    Violated,\n\
    \    /// An instance settles neither way.\n    Undecided,\n    /// The shared\
    \ evaluation meter denied a charge.\n    Incomplete {\n        /// The exhausted\
    \ accounting coun"
- id: fsl-package/src/emit.rs:4142
  path: fsl-package/src/emit.rs
  role: context_only
  excerpt: "/// Checked composite declarations whose nodes can be emitted. A plan's\n\
    /// records belong to its isolated checking scope, alongside the unit's\n/// ordinary\
    \ types, and need the same producer identity rows.\nfn emission_composites(\n\
    \    graph: &CheckedGraph,"
- id: fsl-semantics/src/model/intake/model_unit.rs:1334
  path: fsl-semantics/src/model/intake/model_unit.rs
  role: context_only
  excerpt: "        json!({\n            \"identity\": identity,\n            \"displayName\"\
    : identity,\n            \"kind\": {\"module\": package, \"name\": \"value_type\"\
    },\n            \"roles\": [],\n            \"origin\": source_origin(parsed,\
    \ Span { start, end }),\n            \"extensions\": [], \"unknownPolicy\": \"\
    reject\", \"scalar\": scalar,\n            \"constraints\": constraints,"
- id: tests/support/plan_model.rs:1
  path: tests/support/plan_model.rs
  role: context_only
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! A small authored ConfigVersion\
    \ model for source planning requests.\n\nconst MODEL: &str = r#\"language \"ix:native\"\
    \ edition \"1-draft\";\ndomain \"example/config-version\";\ntype VersionNumber\
    \ = Int[0, 3];\nobject ConfigVersion {\n  versionNumber: VersionNumber;\n  parent:\
    \ ConfigVersion?;\n  operation attemptUpdate(): Boolean modifies self.versionNumber;\n\
    }\npopul"
```
