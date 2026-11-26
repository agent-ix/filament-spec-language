---
id: SR-96741
title: "code-review of FR-1051 certificate checker trusted base"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md; PR4035 47b83d02c1717277360e3b7794ea6851d200e3d1"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1051
    type: references
---

## Summary

Ticket: QSL-3504. Independent review at head 47b83d02c1717277360e3b7794ea6851d200e3d1 (merge base 9842eb939a). Read-only; no cargo was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Operation-contract arm: only ContractTarget::evaluate is recorded; the producer also takes claim selection, case enumeration and wire writing from checker code, and ContractTarget::replay holds the producer ModelSystem | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:249 |
| FND-002 | high | Producer objects missing from X-35..X-40: replay_hyper_trace_tuple, check_match_witness, replay_model_relation_tuple, and the producer closure the finite hyper replay decides with | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:255 |
| FND-003 | high | Two producer calls into checker code have no row: ProbabilityProductKey::for_model_state_with_budget and RefinementQueryRequest::with_certificate_request | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:233 |
| FND-004 | high | Names counts a method call only when T::m is already in a table (and the file names T), so an unlisted shared method is never a finding | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:110 |
| FND-005 | medium | Checker code names items that are in no base row and no exception: ModelLasso, CounterexampleWireError, and TemporalCounterexample and StepSite methods | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:140 |
| FND-006 | medium | The base trusts the whole fsl_eval::temporal formula evaluator, wider than the approved core (stepping, predicate reading, position automaton, codecs) | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:136 |
| FND-007 | medium | Producer code is a closed path list, so code in no class (hybrid/**, model_check/item.rs, planner/**, task_transition.rs, fsl-command, fsl-inspect, fsl-conformance) may call checker internals unreported | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:85 |
| FND-008 | low | Two out-of-scope paths settle proved: protocol_analysis and execute bounded shadow; the exclusion text does not say so | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:94 |

### FND-001 detail

contract_check.rs is checker code. fsl-analyze calls select_contract (analyze/request.rs:1551,1574; tie_rule.rs:48,69), ContractTarget::new (contract_check/mod.rs:144,246), declared_cases (:156), initial_cases (:158), wire and read; none is a check entry, codec function or X row. The arm settles proved (basis exhaustive, uncertified) from that enumeration with no certificate, so a case that declared_cases or initial_cases omits is a false proof that nothing records. ContractTarget::replay, a check entry, reads the producer-opened ModelSystem through &self (contract_check.rs:646-650) and is in none of X-35..X-40. The path calls make AC-5 fail on main.

### FND-002 detail

HyperReplay carries systems: Vec<&ModelSystem>, letters: &dyn HyperLetters and matcher: &dyn StepMatch; ModelRelationReplay carries systems. fsl-analyze passes its own into replay_hyper_trace_tuple (analyze/hyper_route.rs:209, analyze/package_hyper.rs:205). check_match_witness takes the same request and its Accepted answer decides whether an SMT hyper proof settles proved (model_check/hyper_settle.rs:651-662). Only replay_model_relation_input is in X-40, so AC-5 fails on main. replay_finite_hyper_refutation and reproduce_finite_hyper_refutation take the population and a body closure from fsl-analyze (analyze/finite_hyper_item.rs:313): the producer closure is the whole evaluation. The banned-type list has no closure or population type, so AC-4 never reports it.

### FND-003 detail

fsl-analyze/src/analyze/exact/candidate.rs:56 derives product keys with for_model_state_with_budget, the function the EN-5 checker uses (reward.rs:196, long_run.rs:972, refutation.rs:944, cyclic.rs:189, fairness.rs:624); X-18 names for_model_state_with_limits and the formula forms only, and the Codec table has neither. with_certificate_request is listed as checker code (Single functions) and the producer encode_refinement_query calls it (refinement_query.rs:311); it builds the certificate request both sides encode from. Neither is a check entry, codec function or exception, so the requirement is false on main and AC-5 fails.

### FND-004 detail

A new or existing sharing made by a method call on an unlisted method is invisible to AC-1, AC-2 and AC-3, and AC-5 passes without it. Live cases: fsl-analyze target.declared_cases() and target.initial_cases(); fsl-hybrid-reference/src/refute.rs calls reader.guard_reading, guard_defined, resets_defined, reset_box, reset_point, guard_instants, satisfies_initial and polynomial_solution of the checker ModelReader in a file that never names the type; model_check/replay.rs calls payload.trace_position() and closure_loop() defined in producer code. The rule must resolve every method call by receiver type, or the lint must fail closed on a call it cannot resolve; being listed in a table cannot be a precondition for being named.

### FND-005 detail

model_check/graph_hyper.rs:35 imports state_graph::ModelLasso (the row lists five state_graph types, not it). contract_check.rs:726-769 uses CounterexampleWireError from outcome::counterexample (not listed). model_check/replay.rs calls TemporalCounterexample::closure_loop and trace_position and StepSite::pointer, functions of producer code that the row excludes (type items only); trace_position derives a member value both sides use. The first two are AC-1 findings with no row, so AC-5 fails on main.

### FND-006 detail

fsl_eval::temporal is the SM-1 formula evaluator. The producer hyper search decides with evaluate_tuple and evaluate_projected_prefix (fsl-analyze model_check/hyper.rs:2541, hyper/projected.rs:125, hyper/single.rs:586) and the checker check_match_witness and hyper replay decide with the same functions, so a defect in temporal evaluation passes both. The approved design names predicate reading, not formula evaluation. Record it as an exception, or get an arch ruling and say why in the row; fsl_eval::value (model-relation evaluation) is in the same row.

### FND-007 detail

Moving a producer helper to a new fsl-replay file or another crate removes it from AC-2 and AC-3 without removing the sharing. Existing unclassified files include model_check/hyper_smt_outcome.rs, hyper_counterexample.rs, hybrid/provider.rs and hybrid/process_request.rs (settling code lists hybrid.rs only). Define producer code as everything that is not checker, settling, test or a named out-of-scope path.

### FND-008 detail

protocol_analysis/settle.rs:37-47 maps AnalysisOutcome::Proved to TerminalValue::Proved, Uncertified, label proved (fsl-analyze protocol_analysis/project.rs:294 produces it). execute/composite_parity.rs verified_rows settles VerifiedShadowResult::Proved. Neither has an FR-282 checker, so there is no independence to enforce, but a reader of this text would believe neither settles a proof. statistical/** settles measured values only. State that these settle Uncertified proofs with no checker.

## Verdict

Changes requested. Producer and checker share code that is in neither table (FND-001 to FND-003, FND-005), and the Names rule hides method-call sharing (FND-004). Checked and clean: the probability writer rows X-17 and X-19 to X-21 (probability_certificate_document has no producer caller); check_protocol_initial, check_protocol_step and open_protocol_systems are called only under cfg(test); certificate/smt.rs calls the fsl_eval::smt encoders covered by X-9; every fsl_eval path named by checker files is in the base or an exception row.

## Scope and evidence

- FR-1051: examined
- fsl-replay checker modules: examined
- producer call sites: examined

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | high | Constructors and builders of checker code that both sides call break the shared-semantics rule and no exception row covers them, so AC-7 cannot pass on main | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:171 |
| FND-010 | medium | The stated blind spots omit implicit calls and macro arguments, and permit skipping feature-gated code the class rules call non-test | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:116 |

### FND-009 detail

Zone::zero is called by fsl-analyze zone_check/task_certificate.rs:253 and by the checker certificate/zone_certificate/product.rs:309; Zone::from_matrix by zone_check/certificate.rs:305 and by zone_certificate/wire.rs:159 and divergence.rs:342; ProbabilityDocumentLimits::for_exact by analyze/exact.rs:663 and probability_certificate/document/timed.rs:4823. Codec classes only excuse producer-to-checker calls, not the both-sides rule. No S, P or O row concerns certificate/zone, so no entry may excuse the zone calls. Where a P row does concern the file, the both-sides finding breaks "Shared semantics" while the row's rule is "Producer call into checker", and AC-7 requires the entry to match row and rule. The one-type rule keeps these calls permanently, so a temporary row is the wrong answer: let C-1 to C-4 functions that derive nothing be named by both sides, or put the zone value type in the base. This was already true at 47b83d02c1; the first pass missed it.

### FND-010 detail

Only paths and x.m(..) are said to name an item. Operator, Deref, From via ?, and formatting calls into checker or non-base trait impls (hybrid_replay and certificate define such impls) are neither named nor listed as unseen. Nothing says calls written inside macro arguments (assert!, matches!, format!, local macro_rules as in hybrid_replay/eval.rs) are found; a source walk that does not expand macros never sees them and so never reports them unresolved. The list also allows missing code behind a non-default feature, while "Classes of code" makes test-support code non-test (over 100 cfg sites). Require analysis with all features and expanded macro calls, and list implicit calls as covered or unseen.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checked at 2483104d1e |
| FND-002 | fixed | Checked at 2483104d1e |
| FND-003 | fixed | Checked at 2483104d1e |
| FND-004 | fixed | Checked at 2483104d1e |
| FND-005 | fixed | Checked at 2483104d1e |
| FND-006 | fixed | Checked at 2483104d1e |
| FND-007 | fixed | Checked at 2483104d1e |
| FND-008 | fixed | Checked at 2483104d1e |
