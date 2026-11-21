---
id: SR-2026112117504199740761
title: "code-review (with Rust lane) of QSL-1978 lambda step-sum objective"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@ae8dc866d3d4d5e91a73d71f78fe62476f1d5bf6 vs 5795c03d1c5a1cf0ea1645306fc1f8463018ef70; fsl-cst, fsl-forms, fsl-semantics, fsl-eval, fsl-analyze (plan_search.rs, plan_search/certificate.rs, plan_search/replay.rs, plan_source.rs), tests/it/plan_export_surface.rs, tests/it/request_surface_s1_16.rs"
review_set: subset
---

## Summary

Ticket: QSL-1978 (PR filament-spec-language#3353, draft, base `peter/qsl-1978-plan-wiring`).
I reviewed only the lambda delta, `git diff 5795c03d1c..ae8dc866d3`: 24 files, the six lambda
commits plus the merge of the wiring base and the fixture-only correction. The working tree
was clean at the frozen head. I did not review the wiring parent again.

What I examined, with no defect found in it:

- **Grammar.** `StepSumObjective` (`fsl-cst/src/grammar.rs`) is reachable only as the
  `minimize`/`maximize` root, as an ordered choice ahead of `Expression`. Its suffix is
  optional and admits only `+ [-] <literal>`. A nested form, a non-`sum` aggregate, a
  second argument, a non-constant suffix and `+ 2 * 3` all refuse at S1, at the offending
  token.
- **Forms.** `Mapping::step_sum` (`fsl-forms/src/value.rs`) builds
  `Add(Sum{result_type: None}, constant)`, with an optional `Negate` wrapper, and spans the
  nodes in source order. `ExactValue`, `FloatValue` and `QuantityLiteral` are now mapped
  instead of refused, so Float and Quantity constant syntax is kept.
- **Typing.** `Typer::tally` and `step_sum_total` (`fsl-semantics/src/check/check/typing.rs`)
  type the summand on its own, and the total is derived from the complete summand type:
  - `Int(_)` becomes `Integer`, with no manufactured bound.
  - `Decimal`/`Rational` keep their full domain, scale and rounding (FR-802-AC-7).
  - `BoundedQuantity` becomes `Quantity(unit)`.
  - `Float` is gated on the IEEE profile.
  - A literal with no FR-140 context refuses `AmbiguousLiteral`.
- **Evaluation.** `evaluate_plan_node` (`fsl-eval`) reuses the plan scope, frame and shared
  meter. It does not start a second evaluator or a fresh meter.
- **Classification.** `is_step_sum`, `step_sum_node` and `constant_offset` split off a
  literal-only constant and require an exact total type. A non-literal suffix (a name, an
  alias, `Reach.length`, a model read) stays `General`. An explicit `sum<N>` over a
  non-`steps` source plus a constant stays `General`.
- **Offset handling.** The offset is applied once, through `objective_offset` and
  `step_sum_objective`, in four places: the search goal value, the replay
  (`replay::objective_value`), the closure checker, and the potential checker's `initial`
  rule (offset at the empty plan). Potentials exclude the offset.
- **Checker independence.** The checker re-derives the prefix total from the source steps
  (`step_total`). It no longer adds onto the recorded `entry.key.accumulated`.
- **Typed running total.** Search dominance, the closure checker and replay all use the
  same typed running total. The evaluator checks the running total at every addition
  (`collections::sum_exact`), so pruning a prefix whose total is out of domain cannot drop
  a defined completion.
- **Stops.** Resource stops go through `evaluation_stop`, which keeps `Cancelled`
  separate from `Accounting`. The authored-domain overflow (`SumOutOfDomain`) stays
  `Undefined`.
- **Preserved behaviour.** Float/Quantity objectives and native Float/Rational/Decimal
  result enumeration are still refused in `SourcePlan::new` before search; this diff does
  not touch that. Begin gate, receiver and ordinal identity, `begin_plan_replay` and the
  shared `evaluation_meter` lifecycle are not changed by this diff.
- **Fixture correction (ae8dc866).** The SHOP `.replace` targets match the fixture text
  exactly. The objective expressions and oracles are unchanged.

Rust lane: rust-style baseline plus its implementation, API-design and safety references, and
the repository's AGENTS.md. No new panic path reachable from source input: the
`tokens.get(..)` uses in `step_sum` refuse rather than index. No `unsafe`, Tokio or wire
changes.

## Verdict

**CONDITIONAL: one medium and two low findings.**

The change implements the root form, independent summand typing and the once-only offset.
Search, replay and the independent checker all re-derive the offset from source.

The medium finding is resource behaviour: the production `PlanRules` impl re-evaluates the
whole prefix sum at every extension and inside every checker successor record, all on the
shared evaluation meter. That is quadratic metered work, which can produce `incomplete` stops
the linear per-step semantics would not.

This review is not mergeable on its own evidence. All 32 focused tests are UNRUN at
ae8dc866, and ARCH runtime results were not available when this review was written. The
strict merge-order hold also applies (wiring, then FSpec786, before this PR).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `SourcePlan::accumulate_step_cost` ignores its `total` and `cost` arguments and re-evaluates the step sum over the whole prefix plan on every call. The search calls it once per extension with the full prefix. The checker's `step_total` calls it once per prefix index, and does so for every derived successor of every entry. Metered evaluation work is therefore O(depth) per search extension and O(depth^2) per checker successor record, all charged to the one shared `evaluation_meter`. Under a fixed `ScalarLimits` budget, a long-horizon plan search, or its certificate check, stops `incomplete` where linear accumulation would complete. The trait doc ("Add a step cost to its prefix total") also does not describe what the only production impl does. `prefix_with` is computed twice per extension (`plan_search.rs:1091`, `:1110`), and the checker clones extended prefixes even for `General` objectives. | fsl-analyze/src/plan_source.rs:983 |
| FND-002 | low | `SourcePlan::add_objective_values` maps `Outcome::Refused` to `Ok(None)` (an undefined objective), whereas `evaluate_node` treats a `Refused` outcome as an adapter fault (`self.fault` plus a `Cancelled` stop). A refused exact addition while a potential is being built, or during the descent/initial checks, would silently drop an edge or reject the certificate, instead of reporting the contract violation the rest of the adapter reports. | fsl-analyze/src/plan_source.rs:1019 |
| FND-003 | low | A step sum is encoded as `ExprNode::Sum { result_type: None, result_type_span: Span { start: 0, end: 0 } }`, and `Typer::tally` seeds `ValueType::Integer` as a placeholder before replacing it once the summand is typed. A sentinel span and a placeholder type stand in for "absent" (rust-style API design: use `Option<T>` for true absence, not a sentinel). An `Option<TypeForm>`-shaped field, or a distinct step-sum node, would keep the meaningless span and placeholder type out of every match site. | fsl-forms/src/value.rs:1197 |

## Gates

The brief makes this review reviewer-only. I ran no cargo build, clippy, fmt, test, Kani,
replay or full gate.

Production evidence supplied by the coder, not re-run by this reviewer: six-package
`cargo check --lib` PASS at 2138cbfc. Every source and manifest diff between 2138cbfc and
ae8dc866 is empty; only `fsl-semantics/tests/it/synthesis_plan_value.rs` changed. The first
2138 attempt (exit 143) was measured load shedding and is not counted as a pass.

Focused tests: 3 CST, 7 forms, 11 semantics and 11 unique root tests, 32 in total, all
UNRUN. These are unverified risks that only the runtime results can settle:

- whether the `+ 0.05` and `+ rational(1,3)` offsets check into `Literal` nodes, so that
  `constant_offset` accepts them and the tests' `potential` assertions hold;
- whether the forms tests' `S1:` prefix and span oracles hold.

## Initial review completion

The original finding rows remain byte-for-byte unchanged. This is completion of the same initial review after the original worker stopped on its weekly limit; no fix/disposition round has occurred. Native Codex continued the review with explicit architecture approval. Exact native model identifier is not exposed to this reviewer: metadata-unavailable. The obsolete Opus backend instruction is superseded; no Opus model identity is claimed for this completion.

The supplied runtime evidence supersedes historical draft statements below that all 32 tests were unrun. Original draft prose is retained for provenance; this completion verdict and evidence are authoritative.

### Completion verdict

**FAIL — not mergeable.** Four code findings, including the high-severity source binder lowering defect.

Runtime evidence supplied by the dispatching lead for the exact frozen head, not executed by this reviewer: CST 3 selected = 2 passed / 1 failed (near-miss diagnostic via time_declaration.rs:87, expected identifier versus actual expected `Integer` or identifier); forms 7 selected = 7 passed; semantics 11 selected = 9 passed / 2 failed (a_step_sum_types_its_binder_as_the_step_record at 457 and a_decimal_step_sum_retains_its_complete_declared_summand_type at 483, objective NodePreimage(EmptyBindingName)); root 11 expected / 2 selected = 0 passed / 2 failed (legacy decimal and request-surface step cost, invalid_package at check). Nine new root functions matched none and remain UNEXECUTED. Thus actual execution is 23 selected, 18 passed, 5 failed, plus 9 unrun; it is not 32 passes. The root discovery discrepancy requires the service's exact command, raw log and test list; the nine functions exist with top-level #[test], unconditional module registration and the Cargo it target in the frozen git blob. No missing-attribute explanation is supported. Root invalid_package is observed, but its full cause chain is not independently available here. The diagnostic mismatch is a runtime blocker; this review does not assign an unchanged pre-existing domain grammar oracle to the lambda delta without baseline comparison.

No production-code/spec/branch edits, commits, pushes, builds, tests, full gates or new tickets were performed. Six production libraries passed the supplied check at 2138cbfce5252515332158c595f81518ee26a982; static git comparison confirms all production src/Cargo files are unchanged through the frozen head. The initial exit-143 load-shed attempt is not counted as a pass.

Current tools resolved to quire 0.52.0 (engine 0.62.2), quoin 0.36.8 and spec-artifacts-process@v0.36.0. These are the actual installed versions, rather than the older release-notice values. No applicable AssuranceProfile was found. No spec/plan or CI workflow file changes exist in this stack diff; therefore an initial spec-review method and Jev criterion-strength method are inapplicable, not claimed executed. Optional standalone semantic gap review was not requested. The scope is explicitly a lambda-delta audit, not a repository-wide assurance PASS or a parent wiring re-audit.

Code FND-001 remains a resource-cost concern supported by prefix reevaluation in search and nested prefix recomputation in certificate checking, not a measured benchmark or proof of a specific budget threshold. Any linear implementation must first preserve the proven step-local classification, complete numeric policy, source evaluation order, shared meter, authored-domain failures and independent checker derivation; blindly adding Rational projections is not a safe remedy. Code FND-002's distinct Refused handling is statically present; reachability of Refused from this exact addition API has not been demonstrated, so confidence is medium. Code FND-003 is the explicit absence/sentinel style rule, not evidence that the placeholder reaches a final checked type. Gap FND-005 preserves the original exact-example trace concern; its 0.30 text is an example of typed serialization, so it does not establish that 0.25 serialization is mathematically wrong. Bind the existing 0.30 converted-cost oracle to the local criterion to close the trace mismatch.

Float/Quantity runtime AC-3/AC-5 and native numeric result enumeration remain required open work on QSL-1978. Typed unsupported behavior is not an acceptance pass and is no waiver. Negative/maximized offsets are present as source tests but remain among unexecuted root tests. Cancellation and forged accumulated/initial certificate rejection remain the explicit evidence gaps in the original findings; a positive certification assertion does not prove rejection of a forged proof.

## New findings (initial review completion)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The new lambda objective introduces step/let binder slots, but synthesis checking discards the objective Typer slot names (`let (body, _)`) and retains only the independently checked satisfies slot names. Objective lowering then uses that satisfies table for the objective body; binding an objective-only slot refuses NodePreimage(EmptyBindingName). The two new positive semantics tests fail before their numeric/type oracles. Preserve separate source binder metadata for each checked expression and use the corresponding table at lowering; do not weaken the fixtures or infer names from the satisfies body. | fsl-semantics/src/check/synthesis.rs:279; fsl-semantics/src/check/lowering/synthesis.rs:74; fsl-semantics/tests/it/synthesis_plan_value.rs:457 |

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
- id: fsl-analyze/src/plan_source.rs::accumulate_step_cost
  path: fsl-analyze/src/plan_source.rs
  role: examined
  excerpt: "    fn accumulate_step_cost(\n        &mut self,\n        plan: &PlanView<'_>,\n\
    \        _total: &Rational,\n        _cost: &Rational,\n    ) -> Result<Option<Rational>,\
    \ RuleStop> {\n        let Some(body) = self.synthesis.objective() else {\n  \
    \          return self.objective(plan);\n        };\n        let Some(value) =\
    \ self.plan_value(plan, 0) else {\n            return Ok(None);\n        };\n\
    \        // Reuse the evaluator's ordered, typed sum rather than adding the\n\
    \        // Rational projection outside its decimal domain or meter.\n       \
    \ Ok(\n            match self.evaluate_node(step_sum_node(body.root"
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
- id: fsl-forms/src/value.rs::Shape::StepSum
  path: fsl-forms/src/value.rs
  role: examined
  excerpt: "            Self::StepSum(binder) => {\n                arity(2)?;\n \
    \               ExprNode::Sum {\n                    result_type: None,\n    \
    \                result_type_span: Span { start: 0, end: 0 },\n              \
    \      binder,\n                    source: operands.next()?,\n              \
    \      summand: operands.next()?,\n                }\n            }\n        \
    \    Self::Population {\n                form,\n                binder,"
findings:
- fnd: FND-001
  severity: medium
  confidence: medium
  method: code-review
  check: other
  artifact_id: fsl-analyze/src/plan_source.rs::accumulate_step_cost
  related_ids: []
  path: fsl-analyze/src/plan_source.rs
  lines: 983-1004
  excerpt: "    fn accumulate_step_cost(\n        &mut self,\n        plan: &PlanView<'_>,\n\
    \        _total: &Rational,\n        _cost: &Rational,\n    ) -> Result<Option<Rational>,\
    \ RuleStop> {\n        let Some(body) = self.synthesis.objective() else {\n  \
    \          return self.objective(plan);\n        };\n        let Some(value) =\
    \ self.plan_value(plan, 0) else {\n            return Ok(None);\n        };\n\
    \        // Reuse the evaluator's ordered, typed sum rather than adding the\n\
    \        // Rational projection outside its decimal domain or meter.\n       \
    \ Ok(\n            match self.evaluate_node(step_sum_node(body.root"
  finding: '`SourcePlan::accumulate_step_cost` ignores its `total` and `cost` arguments
    and re-evaluates the step sum over the whole prefix plan on every call. The search
    calls it once per extension with the full prefix. The checker''s `step_total`
    calls it once per prefix index, and does so for every derived successor of every
    entry. Metered evaluation work is therefore O(depth) per search extension and
    O(depth^2) per checker successor record, all charged to the one shared `evaluation_meter`.
    Under a fixed `ScalarLimits` budget, a long-horizon plan search, or its certificate
    check, stops `incomplete` where linear accumulation would complete. The trait
    doc ("Add a step cost to its prefix total") also does not describe what the only
    production impl does. `prefix_with` is computed twice per extension (`plan_search.rs:1091`,
    `:1110`), and the checker clones extended prefixes even for `General` objectives.'
- fnd: FND-002
  severity: low
  confidence: medium
  method: code-review
  check: other
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
  finding: '`SourcePlan::add_objective_values` maps `Outcome::Refused` to `Ok(None)`
    (an undefined objective), whereas `evaluate_node` treats a `Refused` outcome as
    an adapter fault (`self.fault` plus a `Cancelled` stop). A refused exact addition
    while a potential is being built, or during the descent/initial checks, would
    silently drop an edge or reject the certificate, instead of reporting the contract
    violation the rest of the adapter reports.'
- fnd: FND-003
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: fsl-forms/src/value.rs::Shape::StepSum
  related_ids: []
  path: fsl-forms/src/value.rs
  lines: 1193-1205
  excerpt: "            Self::StepSum(binder) => {\n                arity(2)?;\n \
    \               ExprNode::Sum {\n                    result_type: None,\n    \
    \                result_type_span: Span { start: 0, end: 0 },\n              \
    \      binder,\n                    source: operands.next()?,\n              \
    \      summand: operands.next()?,\n                }\n            }\n        \
    \    Self::Population {\n                form,\n                binder,"
  finding: 'A step sum is encoded as `ExprNode::Sum { result_type: None, result_type_span:
    Span { start: 0, end: 0 } }`, and `Typer::tally` seeds `ValueType::Integer` as
    a placeholder before replacing it once the summand is typed. A sentinel span and
    a placeholder type stand in for "absent" (rust-style API design: use `Option<T>`
    for true absence, not a sentinel). An `Option<TypeForm>`-shaped field, or a distinct
    step-sum node, would keep the meaningless span and placeholder type out of every
    match site.'
- fnd: FND-004
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: fsl-semantics/src/check/synthesis.rs::objective
  related_ids: []
  path: fsl-semantics/src/check/synthesis.rs
  lines: 279-288
  excerpt: "        let objective = if let Some(objective) = &form.objective {\n \
    \           let (body, _) = check_expression(\n                scope,\n      \
    \          signatures,\n                &objective.expression.expression,\n  \
    \              &parameters,\n                None,\n                &objective_location,\n\
    \            )\n            .map_err(|refusal| vec![refusal])?;"
  finding: The new lambda objective introduces step/let binder slots, but synthesis
    checking discards the objective Typer slot names (`let (body, _)`) and retains
    only the independently checked satisfies slot names. Objective lowering then uses
    that satisfies table for the objective body; binding an objective-only slot refuses
    NodePreimage(EmptyBindingName). The two new positive semantics tests fail before
    their numeric/type oracles. Preserve separate source binder metadata for each
    checked expression and use the corresponding table at lowering; do not weaken
    the fixtures or infer names from the satisfies body.
bindings: []
```

## Disposition round 1

**FAIL — not mergeable.** Original code findings001..004 are fixed; new high FND005 and medium FND006 are open.

Authoritative parent-forwarded ARCH results at exact 14a10a490ee81653bf4eb76faad8099f74e52dc2: CST3 passed; forms7 passed; semantics10 passed/1 failed; root7 passed/4 failed; zero ignored. Total primary32 = 27 passed/5 failed. Semantics failure is a_decimal_step_sum_retains_its_complete_declared_summand_type at its positive-context expect (line483), IllTyped(AmbiguousLiteral). Root failures are a_converted_decimal_step_sum_preserves_cost_and_adds_one_typed_offset, a_large_step_sum_coefficient_obeys_the_named_decimal_budget, a_nonconstant_suffix_on_an_explicit_sum_remains_general and a_rational_step_sum_and_its_offset_are_exact_and_certified. The first root failure reports exit20 versus0, check/ill_typed/ambiguous-literal. Remaining cause chains were checked statically where grounded, not presented as separately obtained raw runtime diagnostics. Analyze4 are UNRUN, held by parent-reported upstream-main relationship_extents/model_unions initializer repairs. Neither those external initializer failures nor historical root-discovery uncertainty is attributed as a new lambda defect. No reviewer runtime was performed.

This is disposition round1 of the same existing review, not another initial review. Original finding rows and earlier sections remain untouched. The intermediate actual-main integration 14a10 was checked against frozen fix candidate a8ae: token conflict retained FatArrow and BidirectionalArrow, and the approved helper uses legal number consistently in declarations/contracts/snapshots/final reads. Numeric, stop and certificate oracles are unchanged. No unrelated accepted-main re-audit was performed.

Supplied six-production-library compile PASS at exact14a10 is retained with its provenance; it does not establish test-target compilation. Required final current-origin/main integration and the mandatory locked all-targets check for every touched crate plus fsl-analyze and filament-spec-language, after latest review custody, remain lead/coder-owned premerge work. Merge must occur within10 minutes of that exact-head PASS and must repeat if main moves. This reviewer ran no cargo, test, build, matrix, baseline campaign, source edit, commit, push, merge or new-ticket operation. Native model identifier remains metadata-unavailable. Tools are quire0.52.0(engine0.62.2), quoin0.36.8, process modulev0.36.0.

Float/Quantity objective runtime and native Float/Rational/Decimal result enumeration remain required open QSL1978 work; typed unsupported is no waiver. No source type/domain/scale is inferred from Decimal spelling or objective expected results. FR140 expressly authorizes the Decimal convert target context; FR039 is considered independently for Rational. The new collection failure is an invalid success fixture, not evidence that nonconstant suffixes are safely classified StepSum.

All fixes/findings remain within the existing maximum TWO coder fix rounds. Round1 is now closed with defects/evidence gaps; route the complete remaining set together to the same coder's FINAL fix round2. No third fix cycle or fresh initial reviewer is authorized. Plan completion: not assessed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | Explicit Decimal conversion targets lose their authorized literal context: ExprNode::Convert resolves the complete target and then always infers its operand with no hint. FR-140 explicitly permits a convert target as the unique expected Decimal type, and FR-802-AC-7 requires convert<Cost>(0.10), so rejecting these literals as AmbiguousLiteral is an implementation defect. The positive complete-type semantics test and converted/large-coefficient root tests fail before their type, exact-value or accounting oracles. Propagate the authored target only where the literal/source typing contract permits it; retain independent source typing for actual conversions and the all-literal ambiguity refusals. The Rational root follows the same no-hint route; preserve an explicit FR-039-authored rational source context or justify propagation from its own contract, rather than applying the Decimal rule indiscriminately. | fsl-semantics/src/check/check/typing.rs:2182-2185 |
| FND-006 | medium | The new unrelated-source classification fixture expects success for sum<Integer>(s in [1,2]: s) + 7, but [1,2] has no unique expected collection type: tally infers its source without a hint and collection_type(None) refuses AmbiguousLiteral. The written Integer is the sum result type and does not declare the source collection kind/bound/type. This failing fixture never exercises the promised General classification and value 10. Give the collection an explicit authored collection context while preserving its non-steps source, once-only offset, exact value 10 and closure oracle; do not infer its collection type from the objective or sum result. | tests/it/plan_export_surface.rs:565-574 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | SourcePlan evaluates one typed step and extends its carried admitted payload with sum_exact. Search builds one extended prefix. Certificate checking derives an independent prefix total once per entry, outside successors, and General successors avoid those prefix clones. Actual ordinary typed decimal, integer offsets and negative/maximized root cases now pass; no benchmark or universal budget claim is made. |
| FND-002 | fixed | Refused exact Rational addition records self.fault and returns Cancelled, matching the adapter-fault policy rather than silently treating it as Undefined. Dynamic Refused reachability is not claimed demonstrated. |
| FND-003 | fixed | Inferred result-type spans and unresolved tally types are represented by Option, with no zero span or provisional Integer value. Forms runtime is 7 passed. |
| FND-004 | fixed | CheckedSynthesis retains objective_slot_names from its own Typer and objective lowering uses that table. The positive step-record binder test and legacy annotated decimal root now pass; the remaining complete-type semantics failure is distinct conversion-context finding FND-005. |
| FND-005 | still-open | New round1 finding: required explicit Decimal conversion context is still discarded; repair and run the affected valid-context cases without weakening the ambiguity-negative oracles. |
| FND-006 | still-open | New round1 finding: the unrelated collection success fixture has no explicit source collection context and fails before classification; repair the fixture while preserving its value/classification assertions. |

## Disposition round 1 data

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "        let added = collections::sum_exact(\n            &value_type,\n\
    \            total.carried.clone(),\n            summand,\n            &mut self.system.evaluation_meter(),\n\
    \        );\n        let retained = match added {\n            Some(Ok((value,\
    \ _loss))) => {"
  reason: SourcePlan evaluates one typed step and extends its carried admitted payload
    with sum_exact. Search builds one extended prefix. Certificate checking derives
    an independent prefix total once per entry, outside successors, and General successors
    avoid those prefix clones. Actual ordinary typed decimal, integer offsets and
    negative/maximized root cases now pass; no benchmark or universal budget claim
    is made.
- fnd: FND-002
  outcome: fixed
  after_excerpt: "            Outcome::Undefined(_) => Ok(None),\n            Outcome::Refused(_)\
    \ => {\n                self.fault.get_or_insert_with(|| {\n                 \
    \   \"a checked objective addition refused evaluation\".to_owned()\n         \
    \       });\n                Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested))\n\
    \            }\n        }"
  reason: Refused exact Rational addition records self.fault and returns Cancelled,
    matching the adapter-fault policy rather than silently treating it as Undefined.
    Dynamic Refused reachability is not claimed demonstrated.
- fnd: FND-003
  outcome: fixed
  after_excerpt: "            Self::StepSum(binder) => {\n                arity(2)?;\n\
    \                ExprNode::Sum {\n                    result_type: None,\n   \
    \                 result_type_span: None,\n                    binder,\n     \
    \               source: operands.next()?,\n                    summand: operands.next()?,\n\
    \                }\n            }\n            Self::Population {"
  reason: Inferred result-type spans and unresolved tally types are represented by
    Option, with no zero span or provisional Integer value. Forms runtime is 7 passed.
- fnd: FND-004
  outcome: fixed
  after_excerpt: "            (Some(objective), Some(body)) => {\n               \
    \ let mut binders = Binders::new(\n                    &checked.objective_slot_names,\n\
    \                    &[Binder {\n                        slot: 0,\n          \
    \              parameter: candidate.clone().into(),\n                        semantic_type:\
    \ candidate_type.into(),\n                    }],"
  reason: CheckedSynthesis retains objective_slot_names from its own Typer and objective
    lowering uses that table. The positive step-record binder test and legacy annotated
    decimal root now pass; the remaining complete-type semantics failure is distinct
    conversion-context finding FND-005.
- fnd: FND-005
  outcome: still-open
  reason: 'New round1 finding: required explicit Decimal conversion context is still
    discarded; repair and run the affected valid-context cases without weakening the
    ambiguity-negative oracles.'
- fnd: FND-006
  outcome: still-open
  reason: 'New round1 finding: the unrelated collection success fixture has no explicit
    source collection context and fails before classification; repair the fixture
    while preserving its value/classification assertions.'
scope:
- id: fsl-analyze/src/plan_source.rs::accumulate_step_cost
  path: fsl-analyze/src/plan_source.rs
  role: examined
  excerpt: "    fn accumulate_step_cost(\n        &mut self,\n        plan: &PlanView<'_>,\n\
    \        _total: &Rational,\n        _cost: &Rational,\n    ) -> Result<Option<Rational>,\
    \ RuleStop> {\n        let Some(body) = self.synthesis.objective() else {\n  \
    \          return self.objective(plan);\n        };\n        let Some(value) =\
    \ self.plan_value(plan, 0) else {\n            return Ok(None);\n        };\n\
    \        // Reuse the evaluator's ordered, typed sum rather than adding the\n\
    \        // Rational projection outside its decimal domain or meter.\n       \
    \ Ok(\n            match self.evaluate_node(step_sum_node(body.root"
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
- id: fsl-forms/src/value.rs::Shape::StepSum
  path: fsl-forms/src/value.rs
  role: examined
  excerpt: "            Self::StepSum(binder) => {\n                arity(2)?;\n \
    \               ExprNode::Sum {\n                    result_type: None,\n    \
    \                result_type_span: Span { start: 0, end: 0 },\n              \
    \      binder,\n                    source: operands.next()?,\n              \
    \      summand: operands.next()?,\n                }\n            }\n        \
    \    Self::Population {\n                form,\n                binder,"
- id: fsl-semantics/src/check/synthesis.rs::objective
  path: fsl-semantics/src/check/synthesis.rs
  role: examined
  excerpt: "        let objective = if let Some(objective) = &form.objective {\n \
    \           let (body, _) = check_expression(\n                scope,\n      \
    \          signatures,\n                &objective.expression.expression,\n  \
    \              &parameters,\n                None,\n                &objective_location,\n\
    \            )\n            .map_err(|refusal| vec![refusal])?;"
- id: fsl-semantics/src/check/check/typing.rs::ExprNode::Convert
  path: fsl-semantics/src/check/check/typing.rs
  role: examined
  excerpt: "            ExprNode::Convert { target, operand } => {\n             \
    \   let target = self.declared_target(target, &location)?;\n                let\
    \ goal = Goal::Infer(expression.at(*operand), None, location.child(0));\n    \
    \            return Ok(Self::unary(Unary::Convert(target), goal, location, frames));\n\
    \            }"
- id: tests/it/plan_export_surface.rs::a_nonconstant_suffix_on_an_explicit_sum_remains_general
  path: tests/it/plan_export_surface.rs
  role: examined
  excerpt: '        ("minimize sum<Integer>(s in [1,2]: s) + 7", "10"),'
- id: FSpec-FR-140::Decimal typing
  path: spec/functional/type-model/FR-140-evaluate-exact-decimals.md
  role: examined
  excerpt: 'A `decimal(c,s)` literal takes exactly its unique

    expected type: the other operand''s static type when that operand is not a

    literal, a declared field, parameter, result or annotated type of its position,

    a `convert` target, or the unique expected type of the enclosing decimal

    operation, which that operation passes to its operands.'
findings:
- fnd: FND-001
  severity: medium
  confidence: medium
  method: code-review
  check: other
  artifact_id: fsl-analyze/src/plan_source.rs::accumulate_step_cost
  related_ids: []
  path: fsl-analyze/src/plan_source.rs
  lines: 983-1004
  excerpt: "    fn accumulate_step_cost(\n        &mut self,\n        plan: &PlanView<'_>,\n\
    \        _total: &Rational,\n        _cost: &Rational,\n    ) -> Result<Option<Rational>,\
    \ RuleStop> {\n        let Some(body) = self.synthesis.objective() else {\n  \
    \          return self.objective(plan);\n        };\n        let Some(value) =\
    \ self.plan_value(plan, 0) else {\n            return Ok(None);\n        };\n\
    \        // Reuse the evaluator's ordered, typed sum rather than adding the\n\
    \        // Rational projection outside its decimal domain or meter.\n       \
    \ Ok(\n            match self.evaluate_node(step_sum_node(body.root"
  finding: '`SourcePlan::accumulate_step_cost` ignores its `total` and `cost` arguments
    and re-evaluates the step sum over the whole prefix plan on every call. The search
    calls it once per extension with the full prefix. The checker''s `step_total`
    calls it once per prefix index, and does so for every derived successor of every
    entry. Metered evaluation work is therefore O(depth) per search extension and
    O(depth^2) per checker successor record, all charged to the one shared `evaluation_meter`.
    Under a fixed `ScalarLimits` budget, a long-horizon plan search, or its certificate
    check, stops `incomplete` where linear accumulation would complete. The trait
    doc ("Add a step cost to its prefix total") also does not describe what the only
    production impl does. `prefix_with` is computed twice per extension (`plan_search.rs:1091`,
    `:1110`), and the checker clones extended prefixes even for `General` objectives.'
- fnd: FND-002
  severity: low
  confidence: medium
  method: code-review
  check: other
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
  finding: '`SourcePlan::add_objective_values` maps `Outcome::Refused` to `Ok(None)`
    (an undefined objective), whereas `evaluate_node` treats a `Refused` outcome as
    an adapter fault (`self.fault` plus a `Cancelled` stop). A refused exact addition
    while a potential is being built, or during the descent/initial checks, would
    silently drop an edge or reject the certificate, instead of reporting the contract
    violation the rest of the adapter reports.'
- fnd: FND-003
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: fsl-forms/src/value.rs::Shape::StepSum
  related_ids: []
  path: fsl-forms/src/value.rs
  lines: 1193-1205
  excerpt: "            Self::StepSum(binder) => {\n                arity(2)?;\n \
    \               ExprNode::Sum {\n                    result_type: None,\n    \
    \                result_type_span: Span { start: 0, end: 0 },\n              \
    \      binder,\n                    source: operands.next()?,\n              \
    \      summand: operands.next()?,\n                }\n            }\n        \
    \    Self::Population {\n                form,\n                binder,"
  finding: 'A step sum is encoded as `ExprNode::Sum { result_type: None, result_type_span:
    Span { start: 0, end: 0 } }`, and `Typer::tally` seeds `ValueType::Integer` as
    a placeholder before replacing it once the summand is typed. A sentinel span and
    a placeholder type stand in for "absent" (rust-style API design: use `Option<T>`
    for true absence, not a sentinel). An `Option<TypeForm>`-shaped field, or a distinct
    step-sum node, would keep the meaningless span and placeholder type out of every
    match site.'
- fnd: FND-004
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: fsl-semantics/src/check/synthesis.rs::objective
  related_ids: []
  path: fsl-semantics/src/check/synthesis.rs
  lines: 279-288
  excerpt: "        let objective = if let Some(objective) = &form.objective {\n \
    \           let (body, _) = check_expression(\n                scope,\n      \
    \          signatures,\n                &objective.expression.expression,\n  \
    \              &parameters,\n                None,\n                &objective_location,\n\
    \            )\n            .map_err(|refusal| vec![refusal])?;"
  finding: The new lambda objective introduces step/let binder slots, but synthesis
    checking discards the objective Typer slot names (`let (body, _)`) and retains
    only the independently checked satisfies slot names. Objective lowering then uses
    that satisfies table for the objective body; binding an objective-only slot refuses
    NodePreimage(EmptyBindingName). The two new positive semantics tests fail before
    their numeric/type oracles. Preserve separate source binder metadata for each
    checked expression and use the corresponding table at lowering; do not weaken
    the fixtures or infer names from the satisfies body.
- fnd: FND-005
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: fsl-semantics/src/check/check/typing.rs::ExprNode::Convert
  related_ids:
  - FSpec-FR-140
  - FSpec-FR-802-AC-7
  - FSpec-FR-039
  path: fsl-semantics/src/check/check/typing.rs
  lines: 2182-2185
  excerpt: "            ExprNode::Convert { target, operand } => {\n             \
    \   let target = self.declared_target(target, &location)?;\n                let\
    \ goal = Goal::Infer(expression.at(*operand), None, location.child(0));\n    \
    \            return Ok(Self::unary(Unary::Convert(target), goal, location, frames));\n\
    \            }"
  finding: 'Explicit Decimal conversion targets lose their authorized literal context:
    ExprNode::Convert resolves the complete target and then always infers its operand
    with no hint. FR-140 explicitly permits a convert target as the unique expected
    Decimal type, and FR-802-AC-7 requires convert<Cost>(0.10), so rejecting these
    literals as AmbiguousLiteral is an implementation defect. The positive complete-type
    semantics test and converted/large-coefficient root tests fail before their type,
    exact-value or accounting oracles. Propagate the authored target only where the
    literal/source typing contract permits it; retain independent source typing for
    actual conversions and the all-literal ambiguity refusals. The Rational root follows
    the same no-hint route; preserve an explicit FR-039-authored rational source context
    or justify propagation from its own contract, rather than applying the Decimal
    rule indiscriminately.'
- fnd: FND-006
  severity: medium
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: tests/it/plan_export_surface.rs::a_nonconstant_suffix_on_an_explicit_sum_remains_general
  related_ids:
  - FSpec-FR-145
  path: tests/it/plan_export_surface.rs
  lines: 565-574
  excerpt: '        ("minimize sum<Integer>(s in [1,2]: s) + 7", "10"),'
  finding: 'The new unrelated-source classification fixture expects success for sum<Integer>(s
    in [1,2]: s) + 7, but [1,2] has no unique expected collection type: tally infers
    its source without a hint and collection_type(None) refuses AmbiguousLiteral.
    The written Integer is the sum result type and does not declare the source collection
    kind/bound/type. This failing fixture never exercises the promised General classification
    and value 10. Give the collection an explicit authored collection context while
    preserving its non-steps source, once-only offset, exact value 10 and closure
    oracle; do not infer its collection type from the objective or sum result.'
bindings: []
```
