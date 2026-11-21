---
id: SR-2026112117504300190525
title: "gap-analysis of QSL-1978 lambda step-sum objective"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@ae8dc866d3d4d5e91a73d71f78fe62476f1d5bf6 vs 5795c03d1c5a1cf0ea1645306fc1f8463018ef70; lambda delta only (fsl-cst, fsl-forms, fsl-semantics, fsl-eval, fsl-analyze, tests/it)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1012
    type: references
---

## Summary

Ticket: QSL-1978 (PR filament-spec-language#3353). This is a planless audit, limited to the
lambda delta.

The computed matrix came from `quire matrix --scope . --format json` (quire 0.52.0, engine
0.62.2; exit 0). It binds the one local criterion this delta touches, FR-1012-AC-3, to
`a_decimal_step_cost_is_exact_and_certified`.

The governing contract is FSpec FR-802 (AC-1 to AC-8, merged literal clarification). FR-802
is a criterion of the FSpec repository, so this repository's matrix cannot compute
FSpec-prefixed tags. I audited those tags by hand against the FR-802 AC text.

The two stub/reverse-gap results:

- **Reverse gap: none.** Every new production item has an owning requirement (FR-802
  classification, evaluation order, objective types). `evaluate_plan_node` has its caller
  in fsl-analyze.
- **Stubs: none on a live path.** The trait defaults `objective_offset` (zero) and
  `accumulate_step_cost` (Rational add) are real behaviour for rule sets with no offset.

Semantic review (intent to test to code): not run as a separate pass. It was not requested.
Test oracles were read directly, as recorded below.

How the FR-802 ACs are backed, with all runtime tests UNRUN at ae8dc866:

- **AC-1:** forms, typing and the root `a_decimal_step_cost_is_exact_and_certified`
  (expects 0.25, potential).
- **AC-2:** Rational exact 1 and 4/3; `integer_bits` incomplete; `decimal_digits` incomplete
  versus the authored domain.
- **AC-4:** negative step gives closure.
- **AC-7:** converted Cost at 0.30 and 0.35, potential.
- **AC-8:** primary and constructor ambiguity, at typing and at the root.
- **Authored Decimal domain:** overflow gives infeasible with `undefined_objectives` > 0,
  distinct from the named `incomplete` resource stops.
- **AC-3 and AC-5 (Quantity/Float runtime):** intentionally unsupported in this PR and
  still open on QSL-1978. They are not completion claims of this delta.

## Verdict

**CONDITIONAL: two medium and three low findings.**

No local criterion is untagged. The medium findings are:

- a forms-only test that claims the runtime ACs AC-3 and AC-5;
- missing tests for two behaviours this delta adds: cancellation distinguished from
  accounting in the new objective-arithmetic paths, and the checker rejecting a forged
  accumulated total or potential `initial` value under an offset.

A PASS for runtime ACs needs the 32 focused tests run, which has not happened.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_step_sum_retains_float_and_quantity_constant_forms` checks only that S2 keeps Float and Quantity constant syntax. It is tagged `FSpec-FR-802-AC-3` (seconds-unit tie at 60 s) and `FSpec-FR-802-AC-5` (NaN exclusion and `totalOrder` ranking). Both are runtime behaviours that `SourcePlan::new` still refuses as unsupported, so the tags inflate coverage of two open ACs. | fsl-forms/tests/it/synthesis_verify_forms.rs:246 |
| FND-002 | medium | No test in the delta drives a cancellation through the new objective arithmetic: `add_objective_values`, `accumulate_step_cost`, `objective_offset` and the potential build. Nothing shows that it reports `Cancelled` rather than an `accounting.*` incomplete. The brief requires cancellation to be distinguished from named resource stops, and the tests cover only the two `accounting` settings. | fsl-analyze/src/plan_source.rs:1005 |
| FND-003 | medium | Checker independence is tested only on the happy path, where search and checker agree. No fault-injection test shows `certificate::check` rejecting either of two forgeries: a recorded `accumulated` that disagrees with the re-derived prefix total, or a potential `initial` value that omits or doubles the offset. The existing `PlanFault` seams (`ForgeEntryState`, `DropSuccessor`, `ParentCycle`) do not reach the new `step_total`, offset or `initial` logic. | fsl-analyze/src/plan_search/certificate.rs:488 |
| FND-004 | low | `FSpec-FR-802-AC-6` (`maximize Reach.final.value` is general with closure; no objective minimizes `length`) is tagged on `a_step_sum_adds_its_offset_once_and_certifies_empty_plans`, which asserts a `potential` certificate for a step sum, and on two other tests that exercise neither clause. | tests/it/plan_export_surface.rs:359 |
| FND-005 | low | FR-1012-AC-3 states the typed decimal form with the example `coefficient "30"` for `0.30`. Its only bound test, `a_decimal_step_cost_is_exact_and_certified`, now asserts `coefficient "25"` (FR-802-AC-1's 0.25). The spec example and the bound test's oracle disagree. The 0.30 case lives in `a_converted_decimal_step_sum_preserves_cost_and_adds_one_typed_offset`, which is not tagged FR-1012-AC-3. | spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md:32 |

## Coverage

- Local criteria touched by the delta: 1 (FR-1012-AC-3), tagged; 0 untagged.
- FSpec FR-802 ACs claimed by delta tests: AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8.
  Each was audited by hand (FND-001 and FND-004 cover the mis-tags).
- Runtime evidence: none. 32 focused tests UNRUN, and ARCH results were not available.
- Semantic review: not run as a separate pass.
- Plan completion: not assessed

## Initial review completion

The original finding rows remain byte-for-byte unchanged. This is completion of the same initial review after the original worker stopped on its weekly limit; no fix/disposition round has occurred. Native Codex continued the review with explicit architecture approval. Exact native model identifier is not exposed to this reviewer: metadata-unavailable. The obsolete Opus backend instruction is superseded; no Opus model identity is claimed for this completion.

The supplied runtime evidence supersedes historical draft statements below that all 32 tests were unrun. Original draft prose is retained for provenance; this completion verdict and evidence are authoritative.

### Completion verdict

**CONDITIONAL — not mergeable.** Five original gap findings remain open, and runtime execution is incomplete/failing.

Runtime evidence supplied by the dispatching lead for the exact frozen head, not executed by this reviewer: CST 3 selected = 2 passed / 1 failed (near-miss diagnostic via time_declaration.rs:87, expected identifier versus actual expected `Integer` or identifier); forms 7 selected = 7 passed; semantics 11 selected = 9 passed / 2 failed (a_step_sum_types_its_binder_as_the_step_record at 457 and a_decimal_step_sum_retains_its_complete_declared_summand_type at 483, objective NodePreimage(EmptyBindingName)); root 11 expected / 2 selected = 0 passed / 2 failed (legacy decimal and request-surface step cost, invalid_package at check). Nine new root functions matched none and remain UNEXECUTED. Thus actual execution is 23 selected, 18 passed, 5 failed, plus 9 unrun; it is not 32 passes. The root discovery discrepancy requires the service's exact command, raw log and test list; the nine functions exist with top-level #[test], unconditional module registration and the Cargo it target in the frozen git blob. No missing-attribute explanation is supported. Root invalid_package is observed, but its full cause chain is not independently available here. The diagnostic mismatch is a runtime blocker; this review does not assign an unchanged pre-existing domain grammar oracle to the lambda delta without baseline comparison.

No production-code/spec/branch edits, commits, pushes, builds, tests, full gates or new tickets were performed. Six production libraries passed the supplied check at 2138cbfce5252515332158c595f81518ee26a982; static git comparison confirms all production src/Cargo files are unchanged through the frozen head. The initial exit-143 load-shed attempt is not counted as a pass.

Current tools resolved to quire 0.52.0 (engine 0.62.2), quoin 0.36.8 and spec-artifacts-process@v0.36.0. These are the actual installed versions, rather than the older release-notice values. No applicable AssuranceProfile was found. No spec/plan or CI workflow file changes exist in this stack diff; therefore an initial spec-review method and Jev criterion-strength method are inapplicable, not claimed executed. Optional standalone semantic gap review was not requested. The scope is explicitly a lambda-delta audit, not a repository-wide assurance PASS or a parent wiring re-audit.

Code FND-001 remains a resource-cost concern supported by prefix reevaluation in search and nested prefix recomputation in certificate checking, not a measured benchmark or proof of a specific budget threshold. Any linear implementation must first preserve the proven step-local classification, complete numeric policy, source evaluation order, shared meter, authored-domain failures and independent checker derivation; blindly adding Rational projections is not a safe remedy. Code FND-002's distinct Refused handling is statically present; reachability of Refused from this exact addition API has not been demonstrated, so confidence is medium. Code FND-003 is the explicit absence/sentinel style rule, not evidence that the placeholder reaches a final checked type. Gap FND-005 preserves the original exact-example trace concern; its 0.30 text is an example of typed serialization, so it does not establish that 0.25 serialization is mathematically wrong. Bind the existing 0.30 converted-cost oracle to the local criterion to close the trace mismatch.

Float/Quantity runtime AC-3/AC-5 and native numeric result enumeration remain required open work on QSL-1978. Typed unsupported behavior is not an acceptance pass and is no waiver. Negative/maximized offsets are present as source tests but remain among unexecuted root tests. Cancellation and forged accumulated/initial certificate rejection remain the explicit evidence gaps in the original findings; a positive certification assertion does not prove rejection of a forged proof.

### Computed matrix reconciliation

Reconciliation: retained quire matrix (quire 0.52.0, engine 0.62.2), original exit 0; no run-evidence store was read. No massive matrix was rerun. The retained FR-1012-AC-3 row is tagged to a_decimal_step_cost_is_exact_and_certified, with an additional unmarked-trace-id diagnostic at the documentation line; the real trace attribute binds the test. Only that local criterion is in this audit scope. External FSpec AC tags were read against FR-802 rather than counted as local matrix bindings. No repository-wide coverage claim is made.

Reverse-gap inventory covered the new root grammar/forms, derived numeric total type, plan-node evaluator, source objective adapter, and search/replay/certificate arithmetic. All six behavior groups have FR-802 ownership; no new owning-requirement gap or live stub was found in the scoped delta.

Plan completion: not assessed

## Examined scope and finding data

```yaml
clean: false
scope:
- id: fsl-analyze/src/plan_search.rs
  path: fsl-analyze/src/plan_search.rs
  role: examined
  excerpt: "    /// Add a step cost to its prefix total under the objective's numeric\n\
    \    /// semantics and meter. `plan` includes the new step. An authored\n    ///\
    \ domain violation returns `None`; a resource denial returns a stop.\n    fn accumulate_step_cost(\n\
    \        &mut self,\n        _plan: &PlanView<'_>,\n        total: &Rational,\n\
    \        cost: &Rational,\n    ) -> Result<Option<Rational>, RuleStop> {\n   \
    \     self.add_objective_values(total, cost)\n    }\n\n    /// Add exact objective\
    \ projections while constructing or checking a\n    /// certificate. Evaluating\
    \ implementations charge their numeric meter.\n  "
- id: fsl-analyze/src/plan_search/certificate.rs
  path: fsl-analyze/src/plan_search/certificate.rs
  role: examined
  excerpt: "    search: &mut Search<'_, '_, R>,\n        CertificateKind::Potential\
    \ => Some(potentials(search)?),\nfn potentials<R: PlanRules>(\n    search: &mut\
    \ Search<'_, '_, R>,\n) -> Result<Vec<Option<Rational>>, PlanStopCause> {\n  \
    \                  if let Some(through) = search\n                        .rules\n\
    \                        .add_objective_values(cost, rest)\n                 \
    \       .map_err(super::stop_cause)?\n                    {\n                \
    \        if best.as_ref().is_none_or(|kept| through < *kept) {\n             \
    \               best = Some(through);\n                        }\n    Ok(pi)\n\
    \   "
- id: fsl-analyze/src/plan_search/replay.rs
  path: fsl-analyze/src/plan_search/replay.rs
  role: examined
  excerpt: "                sum = match (sum, cost) {\n                    (Some(total),\
    \ Some(cost)) => rules\n                        .accumulate_step_cost(&view, &total,\
    \ &cost)\n                        .map_err(PlanSearchError::RuleStopped)?,\n \
    \                   _ => None,\n                };\n            }\n          \
    \  match sum {\n                Some(total) => rules\n                    .step_sum_objective(&plan.view(),\
    \ &total)\n                    .map_err(PlanSearchError::RuleStopped)?,\n    \
    \            None => None,"
- id: fsl-analyze/src/plan_source.rs
  path: fsl-analyze/src/plan_source.rs
  role: examined
  excerpt: "use filament_checked::node::{Arithmetic, NodeKind, Visit};\n    CheckedBody,\
    \ CheckedNode, CheckedPlan, CheckedSynthesis, PlanOperation, TemporalClauseNode,\n\
    \        self.evaluate_node(body.root(), plan)\n    }\n\n    fn evaluate_node(&mut\
    \ self, node: CheckedNode<'_>, plan: Value) -> Result<Evaluated, RuleStop> {\n\
    \        let evaluation = fsl_eval::value::evaluate_plan_node(\n            node,\n\
    \    let root = step_sum_node(body.root());\n    if !matches!(\n        root.value_type(),\n\
    \        ValueType::Integer | ValueType::Int(_) | ValueType::Rational(_) | ValueType::Decimal(_)\n\
    \    ) {\n        return fals"
- id: fsl-cst/src/cst.rs
  path: fsl-cst/src/cst.rs
  role: examined
  excerpt: "    #[doc = \"`<identifier> . steps . sum ( <identifier> => <expression>\
    \ ) [+ [-] <numeric-literal>]`: the per-step sum of a plan synthesis objective,\
    \ the only position that admits `=>`.\"]\n    StepSumObjective,"
- id: fsl-cst/src/grammar.rs
  path: fsl-cst/src/grammar.rs
  role: examined
  excerpt: "                s(vec![\n                    contextual(\"minimize\"),\n\
    \                    c(vec![r(P::StepSumObjective), r(P::Expression)]),\n    \
    \            ]),\n                s(vec![\n                    contextual(\"maximize\"\
    ),\n                    c(vec![r(P::StepSumObjective), r(P::Expression)]),\n \
    \               ]),\n    g.insert(\n        P::StepSumObjective,\n        s(vec![\n\
    \            ident(),\n            x(\".\"),\n            literal(\"steps\"),\n\
    \            x(\".\"),\n            x(\"sum\"),\n            x(\"(\"),\n     \
    \       ident(),\n            x(\"=>\"),\n            r(P::Expression),\n    \
    \        x(\")\")"
- id: fsl-cst/src/token.rs
  path: fsl-cst/src/token.rs
  role: examined
  excerpt: '    Hash => "#", Arrow => "->", FatArrow => "=>", DotDot => "..", Caret
    => "^", Question => "?", Apostrophe => "''", AtSign => "@", Tilde => "~"'
- id: fsl-cst/tests/it/complete_grammar.rs
  path: fsl-cst/tests/it/complete_grammar.rs
  role: examined
  excerpt: "    Production::StepSumObjective,\n                    \"synthesis Cheapest\
    \ using Complete grammar M::Grammar domain M::Domain minimize Find.steps.sum(s\
    \ => if s.operation = M::Op::charge then 0.10 else 0.05) + 1 satisfies { true\
    \ };\\n\","
- id: fsl-cst/tests/it/synthesis_verify.rs
  path: fsl-cst/tests/it/synthesis_verify.rs
  role: examined
  excerpt: "use super::time_declaration::{accepts, refuses, PREFIX};\nuse fsl_cst::{CompleteCause,\
    \ CompleteCode, Limits, Production};\nuse fsl_foundation::{SourceIdentity, Span};\n\
    \n/// FSpec FR-802 \"Classification\": `Reach.steps.sum(s => c)`, optionally plus\n\
    /// a constant, is a plan objective, and the objective root is the one place\n\
    /// `=>` is read.\n#[test]\n#[trace(\"FSpec-FR-802-AC-1\")]\nfn step_sum_objective_reads_only_at_the_objective_root()\
    \ {\n    for objective in [\n        \"minimize Reach.steps.sum(s => 2)\",\n \
    \       \"minimize Reach.steps.sum(s => if s.operation = M::Op::charge then 0.10\
    \ else 0.05)\",\n    "
- id: fsl-cst/tests/it/time_declaration.rs
  path: fsl-cst/tests/it/time_declaration.rs
  role: examined
  excerpt: 'pub(super) const PREFIX: &str = "language \"ix:native\" edition \"1-draft\";\nprofile
    p = \"filament.value.complete/v1\";\n// durée\n";'
- id: fsl-eval/src/value/expression/mod.rs
  path: fsl-eval/src/value/expression/mod.rs
  role: examined
  excerpt: "    frame: Vec<Option<Value>>,\n    meter: &mut Meter,\n) -> Result<Evaluation,\
    \ InternalFault> {\n    evaluate_plan_node(package, scope, body.root(), frame,\
    \ meter)\n}\n\n/// Evaluate a checked subexpression of a plan body with the same\
    \ scope,\n/// local frame and meter as [`evaluate_plan_body`]. This lets a per-step\n\
    /// objective evaluate its sum separately from its constant offset.\n/// Unlike\
    \ `evaluate_bare_expression`, the node already belongs to the\n/// checked source\
    \ graph and needs its synthetic plan scope; it does not\n/// construct a bytes-only\
    \ expression projection or a second evaluator.\npub f"
- id: fsl-eval/src/value/mod.rs
  path: fsl-eval/src/value/mod.rs
  role: examined
  excerpt: "    evaluate_plan_body, evaluate_plan_node, evaluate_protocol_slot, evaluate_refinement_row,\n\
    \    evaluate_reward,"
- id: fsl-forms/src/syntax.rs
  path: fsl-forms/src/syntax.rs
  role: examined
  excerpt: "    /// `sum<N>(x in c: e)`, or a plan objective's `Reach.steps.sum(s\
    \ => e)`\n    /// (`result_type` is `None`: the total takes the summand's own\
    \ family).\n        result_type: Option<String>,\n            result_type: Some(result_type.name),"
- id: fsl-forms/src/synthesis_verify.rs
  path: fsl-forms/src/synthesis_verify.rs
  role: examined
  excerpt: "    let mut expressions = nodes_of(children, Production::Expression);\n\
    \    expressions.extend(nodes_of(children, Production::StepSumObjective));"
- id: fsl-forms/src/value.rs
  path: fsl-forms/src/value.rs
  role: examined
  excerpt: "    /// `Reach.steps.sum(s => e)`: the binder of the objective's step\
    \ sum.\n    StepSum(String),\n                    result_type: Some(result_type.name),\n\
    \            Self::StepSum(binder) => {\n                arity(2)?;\n        \
    \        ExprNode::Sum {\n                    result_type: None,\n           \
    \         result_type_span: Span { start: 0, end: 0 },\n                    binder,\n\
    \                    source: operands.next()?,\n                    summand: operands.next()?,\n\
    \                }\n            }\n            Production::Primary\n         \
    \   | Production::ScrutineePrimary\n            | Pro"
- id: fsl-forms/tests/it/synthesis_verify_forms.rs
  path: fsl-forms/tests/it/synthesis_verify_forms.rs
  role: examined
  excerpt: "use filament_exact::Integer;\nuse fsl_forms::{\n    build_unit, BinaryOperator,\
    \ DeclarationForm, ExprNode, ExpressionForm, SynthesisDirectionForm,\n    SynthesisForm,\n\
    };\n\nconst STEP_SUM: &str = \"synthesis Reach using p grammar M::G domain M::D\
    \ \\\n     minimize Reach.steps.sum(s => s.index) + 3 satisfies { true };\";\n\
    \n/// FSpec FR-802 \"Classification\": `Reach.steps.sum(s => c) + k` builds one\n\
    /// sum over the plan's `steps` field with no written result type, plus the\n\
    /// constant, each node spanned in source order.\n#[trace(\"FSpec-TC-1952\", \"\
    FSpec-FR-802-AC-1\")]\n#[test]\nfn a_step_sum_objective_builds"
- id: fsl-forms/tests/it/value_forms.rs
  path: fsl-forms/tests/it/value_forms.rs
  role: examined
  excerpt: '        } => format!("Sum[{},{binder}]", result_type.as_deref().unwrap_or("")),'
- id: fsl-semantics/src/check/assemble.rs
  path: fsl-semantics/src/check/assemble.rs
  role: examined
  excerpt: "                    result_type: Some(name),\n                | ExprNode::Sum\
    \ { result_type: None, .. }"
- id: fsl-semantics/src/check/assemble/clock_forms.rs
  path: fsl-semantics/src/check/assemble/clock_forms.rs
  role: examined
  excerpt: '                    result_type: Some(name),'
- id: fsl-semantics/src/check/check/typing.rs
  path: fsl-semantics/src/check/check/typing.rs
  role: examined
  excerpt: "/// The type of the total of a plan objective's `Reach.steps.sum(s =>\
    \ e)`,\n/// given the complete summand type. Integer ranges total as `Integer`;\n\
    /// decimal and rational domains remain the domains the summand declares.\n///\
    \ Resource limits belong to the evaluation meter, never invented domains.\nfn\
    \ step_sum_total(summand: &ValueType) -> Option<ValueType> {\n    match summand\
    \ {\n        ValueType::Integer | ValueType::Int(_) => Some(ValueType::Integer),\n\
    \        ValueType::BoundedQuantity(bounded) => Some(ValueType::Quantity(bounded.unit().clone())),\n\
    \        ValueType::Rational(_)\n        | ValueT"
- id: fsl-semantics/src/check/family.rs
  path: fsl-semantics/src/check/family.rs
  role: examined
  excerpt: '                out.write_str(result_type.as_deref().unwrap_or(""));'
- id: fsl-semantics/tests/it/synthesis_plan_value.rs
  path: fsl-semantics/tests/it/synthesis_plan_value.rs
  role: examined
  excerpt: "use filament_exact::ValueType;\n) -> Result<CheckedSynthesis, String>\
    \ {\n    check_model_with_types(model_text, lengths, objective, goal, \"\")\n\
    }\n\nfn check_step_sum(lengths: &str, objective: &str, goal: &str) -> Result<CheckedSynthesis,\
    \ String> {\n    check_step_sum_with_types(lengths, objective, goal, \"\")\n}\n\
    \nfn check_step_sum_with_types(\n    lengths: &str,\n    objective: &str,\n  \
    \  goal: &str,\n    types: &str,\n) -> Result<CheckedSynthesis, String> {\n  \
    \  let model = SHOP\n        .replace(\n            \"object Order identity (id)\"\
    ,\n            \"type OrderId = Int[0,999];\\ntype OrderTotal = Int[0,100];"
- id: tests/it/plan_export_surface.rs
  path: tests/it/plan_export_surface.rs
  role: examined
  excerpt: "            \"minimize Reach.steps.sum(s => let cost: Decimal[0,10000;2,2;exact]\
    \ = if s.index < 2 then 0.10 else 0.05 in cost)\",\n        json!({\"type\": \"\
    decimal\", \"coefficient\": \"25\", \"scale\": \"2\"}),\n/// The offset is charged\
    \ once at every length, including the empty plan;\n/// replay and the independent\
    \ potential checker must agree with search.\n#[test]\n#[trace(\"FSpec-FR-802-AC-1\"\
    , \"FSpec-FR-802-AC-6\")]\nfn a_step_sum_adds_its_offset_once_and_certifies_empty_plans()\
    \ {\n    for (minimum, expected) in [(0, \"7\"), (1, \"9\"), (3, \"13\")] {\n\
    \        let (exit, value) = plan(\n            &plan_unit(\n       "
- id: tests/it/request_surface_s1_16.rs
  path: tests/it/request_surface_s1_16.rs
  role: examined
  excerpt: ''
- id: fsl-semantics/src/check/synthesis.rs::objective
  path: fsl-semantics/src/check/synthesis.rs
  role: examined
  excerpt: "        let objective = if let Some(objective) = &form.objective {\n \
    \           let (body, _) = check_expression(\n                scope,\n      \
    \          signatures,\n                &objective.expression.expression,\n  \
    \              &parameters,\n                None,\n                &objective_location,\n\
    \            )\n            .map_err(|refusal| vec![refusal])?;"
- id: fsl-semantics/src/check/lowering/synthesis.rs::objective
  path: fsl-semantics/src/check/lowering/synthesis.rs
  role: examined
  excerpt: "        let objective = match (&form.objective, &checked.objective) {\n\
    \            (None, None) => BindingValue::Group(GroupTerm::new(Vec::new())),\n\
    \            (Some(objective), Some(body)) => {\n                let mut binders\
    \ = Binders::new(\n                    &checked.slot_names,\n                \
    \    &[Binder {\n                        slot: 0,\n                        parameter:\
    \ candidate.clone().into(),\n                        semantic_type: candidate_type.into(),\n\
    \                    }],\n                );\n                let root = self.expression(body.root(),\
    \ &mut binders)?;"
- id: fsl-semantics/src/check/lowering.rs::bind
  path: fsl-semantics/src/check/lowering.rs
  role: context_only
  excerpt: "        let semantic_type = semantic_type.into();\n        let name =\
    \ binders.slot_names.get(slot).cloned().ok_or_else(|| {\n            refuse(\n\
    \                location,\n                CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),\n\
    \            )\n        })?;\n        let level = binders.scope.len();"
- id: FSpec-FR-802-AC-1
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: '`minimize Reach.steps.sum(s => let cost: Decimal[0,10000;2,2;exact] =
    if s.operation = M::Op::charge then 0.10 else 0.05 in cost)` over three steps,
    two `M::Op::charge` and one other, evaluates to exactly `0.25` of type `Decimal[0,10000;2,2;exact]`,
    with no binary rounding, and the certificate is `potential`.'
- id: FSpec-FR-802-AC-2
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: An objective summing `1/3` over three steps is exactly `1` as a `Rational`;
    an objective whose intermediate value exceeds `accounting.integer_bits` stops
    the search `incomplete` naming that setting.
- id: FSpec-FR-802-AC-3
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: An objective adding `30 s` and `1 min` per step compares in seconds, so
    a plan of one `1 min` step and one of two `30 s` steps tie at `60 s`.
- id: FSpec-FR-802-AC-4
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: A step cost `s.after.value - s.before.value` that is negative on some step
    reclassifies the objective as general, the certificate is `closure`, and `objective.negative_step`
    names that step.
- id: FSpec-FR-802-AC-5
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: A `Float64` objective that is NaN on one plan excludes that plan, counts
    it in `explored.nan_objectives`, and ranks the remaining plans by `totalOrder`;
    an objective undefined on a plan excludes it and counts it in `explored.undefined_objectives`.
- id: FSpec-FR-802-AC-6
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: '`maximize Reach.final.value` is general and settles with a `closure` certificate;
    with no objective the plan minimizes `length`.'
- id: FSpec-FR-802-AC-7
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: With `type Cost = Decimal[0,10000;2,2;exact];`, `minimize Reach.steps.sum(s
    => convert<Cost>(0.10))` over three steps evaluates to exactly `0.30`; the literal
    and total have the complete `Cost` type, including coefficient bounds `[0,10000]`,
    scale bounds `[2,2]` and mode `exact`, and the non-negative per-step sum takes
    a `potential` certificate.
- id: FSpec-FR-802-AC-8
  path: spec/functional/FR-802-compute-plan-objectives-with-exact-arithmetic.md
  role: examined
  excerpt: '`minimize Reach.steps.sum(s => if s.operation = M::Op::charge then 0.10
    else 0.05)` and the same objective with `decimal(10,2)` and `decimal(5,2)` in
    place of its decimal primaries both refuse at type checking with `refused { code:
    ill_typed, cause: ambiguous-literal }`: neither summand has an FR-140 source of
    a unique expected decimal type, and neither the objective nor the literal spellings
    supplies one.'
- id: FR-1012-AC-3
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: examined
  excerpt: 'A decimal per-step cost objective is written exactly in FSpec FR-181''s
    typed `decimal` form (`{"type": "decimal", "coefficient": "30", "scale": "2"}`
    for `0.30`) and certified with a `potential` certificate.'
- id: fsl-forms/tests/it/synthesis_verify_forms.rs::a_step_sum_retains_float_and_quantity_constant_forms
  path: fsl-forms/tests/it/synthesis_verify_forms.rs
  role: examined
  excerpt: "\n/// Numeric families remain representable even when a planner must return\n\
    /// an explicit unsupported result for their execution.\n#[trace(\"FSpec-FR-802-AC-3\"\
    , \"FSpec-FR-802-AC-5\")]\n#[test]\nfn a_step_sum_retains_float_and_quantity_constant_forms()\
    \ {\n    for (term, constant) in [\n        (\n            \"float64(bits: 0x3ff0000000000000)\"\
    ,\n            \"float64(bits: 0x3ff0000000000000)\",\n        ),"
- id: fsl-analyze/src/plan_source.rs::add_objective_values
  path: fsl-analyze/src/plan_source.rs
  role: examined
  excerpt: "    fn add_objective_values(\n        &mut self,\n        left: &Rational,\n\
    \        right: &Rational,\n    ) -> Result<Option<Rational>, RuleStop> {\n  \
    \      match filament_exact::evaluate_rational_arithmetic(\n            filament_exact::RationalArithmetic::Add(left,\
    \ right),\n            None,\n            &mut self.system.evaluation_meter(),\n\
    \        ) {\n            Outcome::Completed(value) => Ok(Some(value)),\n    \
    \        Outcome::Incomplete(incomplete) => {\n                Err(self.evaluation_stop(incomplete.limit_kind.as_str(),\
    \ incomplete.limit))\n            }\n            Outcome::Undefined(_) | O"
- id: fsl-analyze/src/plan_search/certificate.rs::step_total
  path: fsl-analyze/src/plan_search/certificate.rs
  role: examined
  excerpt: "    states: &[ModelSimState],\n    steps: &[super::PlanStep],\n) -> Result<Option<Rational>,\
    \ PlanStopCause> {\n    let mut sum = zero();\n    for index in 0..steps.len()\
    \ {\n        let view = PlanView {\n            states: &states[..=index + 1],\n\
    \            steps: &steps[..=index],\n        };\n        let Some(cost) = rules.step_cost(&view,\
    \ index).map_err(super::stop_cause)? else {\n            return Ok(None);\n  \
    \      };\n        let Some(total) = rules\n            .accumulate_step_cost(&view,\
    \ &sum, &cost)\n            .map_err(super::stop_cause)?\n        else {\n   \
    \         return Ok(None);\n        };"
- id: tests/it/plan_export_surface.rs::a_step_sum_adds_its_offset_once_and_certifies_empty_plans
  path: tests/it/plan_export_surface.rs
  role: examined
  excerpt: "/// The offset is charged once at every length, including the empty plan;\n\
    /// replay and the independent potential checker must agree with search.\n#[test]\n\
    #[trace(\"FSpec-FR-802-AC-1\", \"FSpec-FR-802-AC-6\")]\nfn a_step_sum_adds_its_offset_once_and_certifies_empty_plans()\
    \ {\n    for (minimum, expected) in [(0, \"7\"), (1, \"9\"), (3, \"13\")] {\n\
    \        let (exit, value) = plan(\n            &plan_unit(\n                \"\
    Int[0, 3]\",\n                \"minimize Reach.steps.sum(s => 2) + 7\",\n    \
    \            &format!(\"Reach.length >= {minimum}\"),\n            ),\n      \
    \      json!({}),\n        );"
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: fsl-forms/tests/it/synthesis_verify_forms.rs::a_step_sum_retains_float_and_quantity_constant_forms
  related_ids:
  - FSpec-FR-802-AC-3
  - FSpec-FR-802-AC-5
  path: fsl-forms/tests/it/synthesis_verify_forms.rs
  lines: 243-253
  excerpt: "\n/// Numeric families remain representable even when a planner must return\n\
    /// an explicit unsupported result for their execution.\n#[trace(\"FSpec-FR-802-AC-3\"\
    , \"FSpec-FR-802-AC-5\")]\n#[test]\nfn a_step_sum_retains_float_and_quantity_constant_forms()\
    \ {\n    for (term, constant) in [\n        (\n            \"float64(bits: 0x3ff0000000000000)\"\
    ,\n            \"float64(bits: 0x3ff0000000000000)\",\n        ),"
  finding: '`a_step_sum_retains_float_and_quantity_constant_forms` checks only that
    S2 keeps Float and Quantity constant syntax. It is tagged `FSpec-FR-802-AC-3`
    (seconds-unit tie at 60 s) and `FSpec-FR-802-AC-5` (NaN exclusion and `totalOrder`
    ranking). Both are runtime behaviours that `SourcePlan::new` still refuses as
    unsupported, so the tags inflate coverage of two open ACs.'
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: fsl-analyze/src/plan_source.rs::add_objective_values
  related_ids: []
  path: fsl-analyze/src/plan_source.rs
  lines: 1005-1020
  excerpt: "    fn add_objective_values(\n        &mut self,\n        left: &Rational,\n\
    \        right: &Rational,\n    ) -> Result<Option<Rational>, RuleStop> {\n  \
    \      match filament_exact::evaluate_rational_arithmetic(\n            filament_exact::RationalArithmetic::Add(left,\
    \ right),\n            None,\n            &mut self.system.evaluation_meter(),\n\
    \        ) {\n            Outcome::Completed(value) => Ok(Some(value)),\n    \
    \        Outcome::Incomplete(incomplete) => {\n                Err(self.evaluation_stop(incomplete.limit_kind.as_str(),\
    \ incomplete.limit))\n            }\n            Outcome::Undefined(_) | O"
  finding: 'No test in the delta drives a cancellation through the new objective arithmetic:
    `add_objective_values`, `accumulate_step_cost`, `objective_offset` and the potential
    build. Nothing shows that it reports `Cancelled` rather than an `accounting.*`
    incomplete. The brief requires cancellation to be distinguished from named resource
    stops, and the tests cover only the two `accounting` settings.'
- fnd: FND-003
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: fsl-analyze/src/plan_search/certificate.rs::step_total
  related_ids: []
  path: fsl-analyze/src/plan_search/certificate.rs
  lines: 663-683
  excerpt: "    states: &[ModelSimState],\n    steps: &[super::PlanStep],\n) -> Result<Option<Rational>,\
    \ PlanStopCause> {\n    let mut sum = zero();\n    for index in 0..steps.len()\
    \ {\n        let view = PlanView {\n            states: &states[..=index + 1],\n\
    \            steps: &steps[..=index],\n        };\n        let Some(cost) = rules.step_cost(&view,\
    \ index).map_err(super::stop_cause)? else {\n            return Ok(None);\n  \
    \      };\n        let Some(total) = rules\n            .accumulate_step_cost(&view,\
    \ &sum, &cost)\n            .map_err(super::stop_cause)?\n        else {\n   \
    \         return Ok(None);\n        };"
  finding: 'Checker independence is tested only on the happy path, where search and
    checker agree. No fault-injection test shows `certificate::check` rejecting either
    of two forgeries: a recorded `accumulated` that disagrees with the re-derived
    prefix total, or a potential `initial` value that omits or doubles the offset.
    The existing `PlanFault` seams (`ForgeEntryState`, `DropSuccessor`, `ParentCycle`)
    do not reach the new `step_total`, offset or `initial` logic.'
- fnd: FND-004
  severity: low
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: tests/it/plan_export_surface.rs::a_step_sum_adds_its_offset_once_and_certifies_empty_plans
  related_ids: []
  path: tests/it/plan_export_surface.rs
  lines: 356-369
  excerpt: "/// The offset is charged once at every length, including the empty plan;\n\
    /// replay and the independent potential checker must agree with search.\n#[test]\n\
    #[trace(\"FSpec-FR-802-AC-1\", \"FSpec-FR-802-AC-6\")]\nfn a_step_sum_adds_its_offset_once_and_certifies_empty_plans()\
    \ {\n    for (minimum, expected) in [(0, \"7\"), (1, \"9\"), (3, \"13\")] {\n\
    \        let (exit, value) = plan(\n            &plan_unit(\n                \"\
    Int[0, 3]\",\n                \"minimize Reach.steps.sum(s => 2) + 7\",\n    \
    \            &format!(\"Reach.length >= {minimum}\"),\n            ),\n      \
    \      json!({}),\n        );"
  finding: '`FSpec-FR-802-AC-6` (`maximize Reach.final.value` is general with closure;
    no objective minimizes `length`) is tagged on `a_step_sum_adds_its_offset_once_and_certifies_empty_plans`,
    which asserts a `potential` certificate for a step sum, and on two other tests
    that exercise neither clause.'
- fnd: FND-005
  severity: low
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: FR-1012-AC-3
  related_ids:
  - FSpec-FR-802-AC-1
  - FSpec-FR-802-AC-7
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  lines: 32-32
  excerpt: 'A decimal per-step cost objective is written exactly in FSpec FR-181''s
    typed `decimal` form (`{"type": "decimal", "coefficient": "30", "scale": "2"}`
    for `0.30`) and certified with a `potential` certificate.'
  finding: FR-1012-AC-3 states the typed decimal form with the example `coefficient
    "30"` for `0.30`. Its only bound test, `a_decimal_step_cost_is_exact_and_certified`,
    now asserts `coefficient "25"` (FR-802-AC-1's 0.25). The spec example and the
    bound test's oracle disagree. The 0.30 case lives in `a_converted_decimal_step_sum_preserves_cost_and_adds_one_typed_offset`,
    which is not tagged FR-1012-AC-3.
bindings: []
```
