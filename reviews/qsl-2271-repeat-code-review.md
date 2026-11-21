---
id: SR-1795266708245856368
title: Ordinary package Repeat code-review for PR 3304
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; QSL-2271; frozen f1c43209be6b2344addb1048c353120a70249369..3695a73b696a38da7a3df762e9e45b989e49806a;
  seven changed Rust files
review_set: subset
---

## Summary

Code review with Rust lane. Initial review of the single frozen commit. One blocking fixture defect found. No implementation edits or Cargo execution.

## Verdict

**FAIL**: FND-001 blocks the claimed test evidence. Production Repeat mapping was inspected without finding another concrete defect; its runtime correctness is not certified.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | All five new Repeat tests use an unreadable Boolean binder from the void touch operation, so source checking refuses before the intended Repeat behavior is exercised. repeat_attempt selects touch, fill::document gives touch no result, and fill::attempt declares a Boolean binder; constraints again/out/not again read it. Use predicates independent of that binder where possible and a genuine Boolean-returning operation with valid contracts for the post-test case; preserve the existing assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:59; fsl-analyze/tests/it/fill.rs:203; fsl-analyze/tests/it/fill.rs:573; fsl-semantics/src/check/protocol_slots.rs:1188 | correct-requirement-no-evidence |

## Examined scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-218-AC-44 | examined | Over the Cell fixture, whose `flag` returns `Boolean` and whose `touch` returns nothing: `attempt T by w on M::Cell::flag contracts [FlagPre, FlagPost] as (t: Boolean) { true }` followed by `check Gate using v { t }` checks and the check reads the result; the same attempt with `as (t: Integer)` and no read of `t`, and the `touch` attempt with `as (t: Boolean)` and no read, each check; with `as (t: Integer)` and `check Gate using v { t = 1 }`, and with the `touch` attempt and `check Gate using v { t }`, each unit refuses exactly once, `ill_typed`/`type-mismatch` at the read `t` inside the check |
| FR-206-AC-5 | examined | A `choice` whose guard reads an earlier binder, a `repeat` and a `check` add no step and no position. For `repeat R maximum 2 { attempt X }` with guard `true` and an `exhausted` control `attempt Z`, where `X` and `Z` are always enabled: the thread rests at `X`, takes `X`, rests at `X` again, takes `X` a second time, and then rests at `Z`; every behaviour takes `X` exactly twice, and `Z` is enabled in no state before the second `X`. With the `exhausted` control replaced by a following sibling `attempt W`, the thread rests at `W` after the second `X`. A `check` whose value is false leaves its th |
| FR-383-AC-6 | examined | In **W** and its variants, `sequence`, `choice`, `parallel` and `repeat` emit their layouts: a `sequence` its children in order and an empty `sequence` an empty list; a `choice` its owner, its set of visible expression-node references and at least two cases sorted by label; a `parallel` each of the join policies `all`, `any`, `quorum` (with `count`) and `predicate` (with the function node), the memory model and `outstanding` exactly when written (every non-`all` join is written with one, FR-218), and the joined labels in ascending order; a `repeat` with a maximum and an `exhausted` control, wi |
| FR-880-AC-8 | examined | `repeat Loop by Seller visible (got.ok) while { not got.ok }` over a body that binds `got` compiles with `test` `post`. A `repeat` that steps only through a nested `repeat` compiles. Both packages are admitted. A post-test `repeat` whose body has a cycle through no step is still refused at S3 by FR-218's progress rule. |
| FR-882-AC-6 | examined | A post-test `repeat` (FR-880-AC-8), a `terminal when` that reads `over` under `on each` (FR-880-AC-6) and a protocol over `Option<M::Order>` each explore to a settled deadlock-freedom result. The post-test `repeat`'s first body step is enabled at its entry with no guard read. |
| FR-940-AC-6 | examined | For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC- |
| FR-940-AC-22 | examined | AC-4's Counter request over source with the items `Bounded` and `AtMostTwo`, run with `replay.input_bytes` set below the length of the package's emission (the up-front read-back is bounded by `i2.input_bytes` and admits the package), settles `AtMostTwo` `inconclusive` with cause `replay_refused` carrying `stage_limit_exceeded`/`input-bytes-exceeded` naming `replay.input_bytes`, never `refuted`, never `failed` with `analyze`/`package-emission`, and settles `Bounded` `proved` as without the setting; the document's category and exit follow its items (category inconclusive, never `internal-failure |
| FR-218 | context_only | When S3 checks a protocol declaration, the `ProtocolClause` family SHALL check `parallel` with its branches and its join policy and `outstanding` rule, `choice`, `repeat`, `await`, `check`, `send`, `receive`, `effect`, `event` and `commit` nodes, channels with their premises, relationship declarations and `related by` clauses, `requires temporal` requirements, replicated roles, the owner and visible facts of each `choice` and `repeat`, and node bodies, and SHALL produce an in-process `CheckedProtocolClause` holding the control tree, the compiled causal edge relation and the scopes the protocol |
| FR-531 | context_only | A consumer that reads a handoff directory (FR-517) with Contract IR's `read_checked_package` holds a `filament_contract_model::CheckedPackageV2` and holds no FSL source `CheckedPackage`, no `Emission` and no `AdmittedPackageRead`. Until this requirement the FSL evaluation entries needed one of those: `EvidencePackage` is built by `from_emission` or `from_read`, both over an `AdmittedPackageRead`, which exists only for a package FSL itself emitted from source and which retains that source (`AdmittedPackageRead::source`); and `CheckedPackageEvaluation` is sealed and implemented for the in-proces |
| FR-390 | context_only | S6a SHALL evaluate one expression of one protocol node of a `CheckedPackage`: an activation guard, a capture initializer, a retry relation, a recovery predicate, a trigger guard, a control's constraint, guard, visible entry or `related` end, a finish or terminal condition, a role's lifetime or a channel's key. The expression is the term the package holds in the node's body (FSpec FR-464, FR-465, FR-466). The caller supplies the value of every binder and every captured value the expression reads, and an observation when a supplied value names a model object. Evaluation reads the package and the |
| FR-730 | context_only | When a protocol `repeat` carries a loop proof (`invariant { I } variant { V }`, FR-218), the protocol model checker SHALL discharge its **invariant obligation**, that `I` holds at every decision of the repeat over the explored protocol subject (FR-215). Where that repeat has no maximum, the checker SHALL also discharge its **termination obligation**, that `V` decreases in its well-founded order on every back edge. Each derived obligation settles `proved`, `refuted` or `incomplete` by the rules below. A repeat with no loop proof has neither obligation. A repeat with a maximum terminates by that |
| FR-850 | context_only | A `native-analyze/1` request MAY select an existing `filament.checked-package/v2` artifact through its `package` member instead of a `program` source (FSpec FR-520). The selection names the FR-517 handoff directory that holds the package. When it does, FSL SHALL admit the directory's `checked-package.json` with the strict checked-package reader, attach the identity rows of its `identities.json` and, when present, the equality rows of its `equality.json`, admit the dependency artifact closure of `dependencies.json` with each package's own witness files, and build from those files alone the anal |

All changed files:

- fsl-analyze/src/analyze/clause_item.rs
- fsl-analyze/tests/it/analyze_package_protocol.rs
- fsl-analyze/tests/it/protocol_roles.rs
- fsl-eval/src/simulation/protocol.rs
- fsl-eval/src/simulation/protocol/package_program.rs
- fsl-eval/src/simulation/protocol/program.rs
- fsl-eval/src/simulation/protocol/settle.rs

## Coverage

Plan completion: not assessed

Seven changed Rust files inspected; five new integration tests inspected. Diff-bounded reverse inventory: package Repeat layout/bounds/test-mode decoding; shared Repeat guard dispatch; guard binder readers/liveness and frame supply; authored-proof typed refusal and analyzer detail; parity/replay fixtures. Each behavior maps to the examined requirements. No source stubs, test stubs, copied implementation, mock bypass, new dependency, public wire change, or narrowed gate found. The test bodies have meaningful assertions, but invalid fixtures prevent execution of those assertions.

The dispatch explicitly forbids matrix/full gates/Cargo/runtime. No computed matrix was run and no repository-wide coverage or assurance claim is made. This is a bounded gap review, with manual inspection of the changed trace tags and assertions; it cannot establish matrix-backed PASS. Optional broad semantic audit not run. Scoped compile evidence supplied by lead: locked fsl-analyze integration-target check exit 0. Five focused runtime tests were queued independently by the lead; no outcomes assumed here.

## Behavior inspection

The new Repeat guard is an admitted package slot, included in the existing constraint reader inventory. frame supplies values by declared binder and thread environment; readers retain earlier Receive values across guard entry/back edges. Package evaluation uses the existing observation/supply/accounting seam, without a source CheckedBody or new equality dispatch. Shared settle retains guard-before-bound behavior, max=0 and max=2 exhaustion ordering, and post-test entry before a guard read. Source S3 infers post mode when the guard reads a body binder; emission writes test=post. Existing protocol view traverses Repeat body then exhausted in source layout order; no synthetic case/branch is added for ordinary Repeat. Attempt paths retain Main/Again/name.

Weak Parallel without Send was inspected explicitly: S3 bars model-state reads in Repeat guard and visible slots (FR-218 Constructs; protocol_slots.rs:1485-1487,602-616). Record/event binder field reads remain permitted and immutable; footprint.rs:1237 binders_read reads the same Program::readers populated from the package guard slot. The Source-only node constraint footprint loop in access.rs:690 does not omit a permitted direct model read by this new guard. No new weak-guard refusal is justified by this scope. The existing ANY Parallel plus ANY Send package refusal is retained; broader message occurrence/weak Send work remains outside this change.

Nonempty authored Repeat proof returns PackageProgramRefusal::RepeatProof before execution; runtime maps it to Unsupported::PackageRepeatProof and analyzer emits unsupported_construct/not-yet-implemented with fact=repeat-proof and the admitted node. This does not implement package loop proof evaluation. Source proof support is unchanged. Await source/runtime/clock, correlation provenance, peer literal projection, transport and handoff selection are untouched.

## Assurance Context

No applicable AssuranceProfile identified. Repository AGENTS.md, CLAUDE.md, CONTRIBUTING.md, installed code-review, rust-review, rust-style and gap-analysis instructions applied. Full runtime/gate/registry assurance is unavailable within this dispatch. No exception granted by reviewer.

## Tool evidence and limits

On terra, /home/peter/.npm-global/bin/quoin reports 0.36.8; /home/peter/.npm-global/bin/quire reports quire 0.52.0 (engine 0.62.2); process module is spec-artifacts-process@v0.36.0. git diff --check base..head exited 0. Schema inspection exited 0 with registry warnings: UnknownEdgeType ADR allowed_links realizes_requirement; UnknownRole ADR FR; UnknownRole ADR NFR; UnknownEdgeType TestMatrix covers; UnknownEdgeType TestMatrixIndex covers. No registry mutation or fallback occurred.

## FIX1 evidence

Reviewed 3695a73b696a38da7a3df762e9e45b989e49806a..e65f8c264bd4b291fc03ac2cc4f08aa8e7934323; test-only fixture correction, original assertions preserved. FND-001 is fixed: irrelevant touch predicates are true, while the post-test reads an actual flag Boolean result with the existing FlagPre/FlagPost contracts supplied through fill::contracts("false"). No production or source grammar change. git diff --check exit 0.

Actual original runtime receipt /tmp/protocol-3304-3695-full.log records 0 passed, 5 failed, S3 void-binder errors; its undefined case also records expression-form for the mod guard. Lead supplied FIX1 runtime outcome: 4 passed, 1 failed; only package_repeat_blocked_attempt_preserves_replay_refusal fails at the same guard expression-form span 581..588. This reviewer ran no Cargo/runtime. The five-test gate is not green.

Root cause for FND-002: protocol_obligation.rs:1429-1477 constructs Typer without complete_value, and check/typing.rs:2479-2486 prohibits IntegerDiv/Rem/Mod when complete_value=false. Both integer div and rem alternatives have the same admission barrier. A source-grounded, unexecuted candidate is a protocol capture zero: Rational[0, 0; 1, 1] = 0/1 and guard zero / zero = zero, with zero visible if needed: check.rs:4581 permits Rational/Rational arithmetic without the complete_value restriction, and protocol slots infer expressions without a definedness discharge. This candidate must pass the existing unchanged division-by-zero and parity oracles before it counts as evidence; no support claim is made yet.

## New findings Round 1

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | high | The undefined-guard scenario still uses 6 mod 0 = 0, which protocol guard typing prohibits because complete_value is false. Source checking returns unsupported_construct/expression-form at Again before exploration, so the test cannot assert undefined-evaluation/division-by-zero or replay parity. Replace it with an expression admitted by the actual guard domain that is undefined at execution, retaining the existing division-by-zero and parity assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:211; fsl-semantics/src/check/check/typing.rs:2479; fsl-semantics/src/check/protocol_obligation.rs:1450 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FIX1 removes void touch binder reads and uses flag Boolean result for the real post-test; runtime now passes four cases, with the remaining failure separately recorded as FND-002. |
| FND-002 | still-open | Actual FIX1 runtime fails the undefined guard at S3 expression-form; a valid undefined expression and all five passing tests remain required. |

## Guard candidate correction

The unexecuted Rational candidate above is withdrawn after inspecting the package projection: lowering.rs:4812 and :4816 emit the same rational.div identity for integer Divide and Rational arithmetic Divide, while reader_protocol_projection.rs:2404 always rebuilds integer-operand NodeKind::Divide. This pre-existing numeric projection issue is outside the Repeat diff and must not be worked around by changing Repeat production.

A better source-grounded candidate uses capture zero: Decimal[0, 0; 0, 0; nearest-even] = decimal(0, 0), with visible(zero) and guard zero / zero = zero. Decimal syntax appears in analyze_family_cli.rs:308; check.rs:4596-4621 permits Decimal arithmetic without complete_value; lowering.rs:4832 emits decimal.div; reader_protocol_projection.rs:2449-2473 includes decimal.div and rebuilds NodeKind::Decimal; evaluate.rs:3213-3225 consumes Decimal operands and invokes the existing decimal division kernel. This is still unexecuted and proves neither source admission nor parity. Preserve division-by-zero/replay/parity assertions for runtime verification.

## FIX2 static inspection

Reviewed e65f8c264bd4b291fc03ac2cc4f08aa8e7934323..23ed53f96ec04a0228a2ac3c23c71d5fb7371a97: only package_repeat_blocked_attempt_preserves_replay_refusal changes (+18/-4). Undefined guard is zero / zero = zero; a zero capture of Decimal[0, 0; 0, 0; exact], initialized by decimal(0, 0), is injected before run and included in visible(zero). A shared local request closure makes the same adjustment for ordinary and replay-byte-bounded requests. The blocked scenario emits the same source; the other four fixtures and production code are unchanged. All assertions retain their original expected values, including undefined-evaluation, division-by-zero, replay reproduction, byte-bound refusal and source/package parity.

Static admission/projection trace matches the Decimal path documented above; exact rounding is carried by the emitted decimal.div mode and target type. git diff --check exit 0. Supplied warm check log /tmp/qsl-2271-repeat-fix2-check.log records fsl-analyze integration target check success in 19.77s. Static readiness established; FND-002 stays still-open until the lead reports the exact failed test passing at FIX2. No new full review or repeated tests requested.

## FIX2 runtime result

Lead reports exact affected test FAIL at 23ed53f96ec04a0228a2ac3c23c71d5fb7371a97: source S1 invalid_syntax, unexpected token, expected run at span 516..523; no undefined runtime was reached. The Decimal capture is currently inserted after channel declarations. Actual ProtocolClause grammar (fsl-cst/src/grammar.rs:2995-3013) requires, immediately after the opening brace: Capture*, Role+, Relationship*, Channel*, requirements/compensation*, terminal/scheduling*, run Control Finish. Capture production at :1856-1866 is capture Parameter = Expression;. Therefore the new capture must precede the first role. The exact existing role Planning on M::Cell; anchor in channel_request is a grammar-grounded insertion point; keep the same capture declaration and prepend it there for the undefined scenario only.

This is the still-open FND-002 fixture admission defect, not a new production finding. Four unaffected FIX1 test passes remain retained. No finding is closed from static plausibility; the focused test must pass after the mechanical order correction, preserving every assertion.

## Capture visibility correction

The lead reports the placement-corrected bc52821c862aedd964db217089d0eb0013ca8b62 exact test FAIL at source S3 ill_typed/operator-ineligible, Visible of Again span 629..633. Parsing now succeeds. The earlier static readiness assessment missed the explicit bare-capture visibility rule; visible(zero) is invalid.

FR-218 Choice and repeat decisions, lines 486-492, rejects a bare literal or capture as a visible entry. protocol_slots.rs:733-756 enforces this via expression_entries: a Local whose binder origin is Capture is refused at SlotKind::Visible with OperatorIneligible. This applies independently of the captured scalar type. The existing visible_entries_are_expressions_for_choices_and_repeats test at protocol_slots/tests.rs:185-241 traces FR-218-AC-34 and exercises this exact rule.

FR-218 lines 542-546 and 561-563 explicitly admit guard reads of protocol captures without any visible listing, because every role holds the immutable capture. protocol_slots.rs:680 classifies the capture; facts guard loop at :789-800 admits Eligibility::Capture without consulting the listed decision-fact set. The grammar- and checker-grounded correction is therefore visible() with the same Decimal capture before roles and the unchanged zero / zero = zero guard. FND-002 remains still-open; this is the same fixture admission issue and no separate finding is introduced. Four unaffected FIX1 greens are retained; no undefined runtime result exists yet.

## D2 actual package divergence

Frozen d390e128bf70fc632797cc09c3e205dd7f16cd67 removes the conditional visible variable and restores visible(), leaving the capture before the first role, the Decimal zero-divisor guard and every assertion unchanged. Source fixture admission is now proven at runtime, so FND-002 is fixed. The lead's exact one-test runtime log /tmp/protocol-3304-d390-full.log records 0 passed / 1 failed: source yields refuted undefined-evaluation/division-by-zero with replay reproduced=true; package without source yields declined invalid_runtime_input/invalid-input at repeat_parity's full-item equality assertion, line 86. The four unaffected e65f8c2 test passes are retained. This is not five passing tests.

Root cause, read-only: lowering.rs:3043-3060 emits Decimal as bounded_domain/decimal_range. PackageProgram's capture typing at package_program.rs:479-482 calls evaluator input.rs:2427-2429 -> bare_slot.rs:439-472 -> reader_protocol_projection.rs:3545 node_value_type. Cargo.lock:610 pins IR rev20a4dfbf29e979da4326ac81f85e8266e464519f. Its exact_projection.rs:3923-4395 has no Decimal/decimal_range projection arm and returns UnsupportedExactType in the final arm. FSL node_value_type adds rational_range, floats, collections and selected structural type fallbacks, but no decimal_range; it ultimately returns Unsupported. input.rs .ok() hides that unsupported type, Program reports MissingIdentity, and analyzer clause_item.rs:1156-1164 converts it to declined invalid input. This explains the observed refusal before decimal guard evaluation. The missing shared native projection is not permission to duplicate numeric type parsing locally or alter the test oracle. Owning-lane architecture disposition is required before a production fix.

## New findings Round 2

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | high | The admitted Decimal-capture Repeat refutes on source with undefined-evaluation/division-by-zero and replay reproduced=true, but the source-absent package request declines invalid_runtime_input/invalid-input before executing the guard. Package capture binder typing cannot project bounded_domain/decimal_range: pinned IR exact projection returns UnsupportedExactType and FSL node_value_type has no Decimal fallback; the Option adapter erases the cause and Program preflight maps it to invalid input. Complete the owning native type projection path or preserve a truthful typed unsupported boundary; do not weaken source/parity/replay assertions. | fsl-eval/src/simulation/protocol/package_program.rs:479; fsl-package/src/reader_protocol_projection.rs:3545; fsl-eval/src/simulation/model/input.rs:2427; fsl-analyze/src/analyze/clause_item.rs:1156 | implementation-bug-despite-evidence |

## Dispositions Round 2

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | At d390e128 source now admits the correctly ordered Decimal capture and visible() guard, reaches undefined division-by-zero and reproduces replay. The package parity failure is independently recorded as FND-003. |
| FND-003 | still-open | Source/package divergence is reproduced; native Decimal capture typing and truthful preflight failure handling remain unresolved. |

## Current verdict after D2

FAIL: FND-001 and FND-002 are fixed; FND-003 is high and still-open. No full suite, Cargo/runtime, matrix or repository assurance run by reviewer. No specification delta or new spec review. Artifacts remain scoped to this PR, with relevant dependency tracing only.

## D3 bounded static review

Reviewed only d390e128bf70fc632797cc09c3e205dd7f16cd67..a1f45ef99cc1bfe555ca9925a400416c76d668e5: 11 files, 271 additions/37 deletions. Architecture explicitly authorized the public sealed ModelEvaluator parameter-type method to return the original BareSelectionFailure Result, directly updating four internal call sites, the sealed default and BarePackage implementation; no compatibility adapter is required. This overrides earlier provisional cross-lane constraints for this bounded fix.

Source fixtures checked first: capture helper inserts before exact first role, matching ProtocolClause Capture* then Role+. Option<Decimal> is permitted by fsl-cst grammar.rs:909, absent by :1578; check.rs:3800-3817 types absent using the declared Option payload. Empty run is a valid empty sequence followed by finish closing. These fixtures introduce no unreadable attempt binder or bare capture in visible(). The previous four green fixtures remain unchanged.

Shared native representation: checked_v2/hybrid.rs:399 extracts decimal_range_type from the existing decoder, and the old Hybrid Decimal arm delegates to it. Bounds and scales are parsed exactly, scales checked via u64 parsing, rounding selected by RoundingMode::from_code and domain constructed by DecimalType::new. reader_protocol_projection.rs:3617 admits the Decimal fallback only for UnsupportedExactType/decimal_range at the requested node or its own semantic type; an Option/record/collection containing Decimal remains unsupported. Native Decimal literal projection reads the admitted coefficient, checks scale narrowing to u32, constructs Decimal and checks membership in its exact declared domain. No rational.div or peer literal projection arm changed. No duplicated Decimal decoder or fabricated CheckedBody was added.

Typed error path: ModelEvaluator's Result preserves BareSelectionFailure through capture, control binder, over and finish binder construction. PackageProgramRefusal::ParameterType and runtime ProtocolRefusal::PackageParameterType retain package, admitted binder node and original typed failure. Analyzer's shared helper calls existing into_view/catalog/reason/locus methods for unknown_required_feature/unsupported-feature, emits package/node/reason/locus, and treats Slot selection failure as FailedWithDetail rather than invalid input or successful projection. The two failure enums now derive Clone/Eq as required by containing existing enums. No public wire, IR pin, source grammar or new dependency changed.

Code inventory: shared Decimal range decoder and literal projection; typed four-site parameter projection propagation; analyzer/runtime refusal mapping; empty-run source/package capture parity regression; nested Option<Decimal> unsupported projection regression. Each belongs to FR-390, FR-531 and FR-940-AC-6/22. New tests retain source deletion, exact records/parity, expected catalog/reason and node/locus evidence, and absence of exploration/counterexample for unsupported nested type. This is a bounded review, not a new matrix/repository audit. No new static findings.

Validation: git diff --check exit 0; supplied final integration compile receipt /tmp/qsl-2271-repeat-decimal-locus-check.log records success in 33.31s. Reviewer ran no Cargo/runtime/matrix/full gate. Root will run exactly the empty-run, nested-refusal and previously failing Repeat tests, retaining four unaffected FIX1 greens. FND-003 remains still-open pending actual results; no green inference from static checks.

## D3 runtime evidence

Actual lead-confirmed Draco receipts at a1f45ef99cc1bfe555ca9925a400416c76d668e5: the package_decimal_capture_ filter reports 2 passed, 0 failed, 0 ignored, 2588 filtered out, exit 0; the exact package_repeat_blocked_attempt_preserves_replay_refusal reports 1 passed, 0 failed, 0 ignored, 2589 filtered out, exit 0. Raw receipts are retained by the lead at /private/tmp/qsl-3304-decimal.log and /private/tmp/qsl-3304-repeat.log. This reviewer relies on the lead's actual result report and did not run or independently read those remote runtime logs.

These results establish empty-run Decimal capture source/package parity; a genuine admitted Option<Decimal> capture retains typed unknown_required_feature/unsupported-feature with missing-type-representation, node/locus and no exploration; and the original failed Repeat now preserves source/package undefined-evaluation/division-by-zero, reproduced replay and replay byte-bound behavior. The four unaffected e65f8c264bd4b291fc03ac2cc4f08aa8e7934323 passes remain retained because their fixtures and production paths were unchanged by the bounded repair. Total scoped evidence: seven tests green, no ignored tests. No full parent completion or repository-wide assurance claim.

## Dispositions Round 3

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | Shared admitted Decimal range projection and native literal decoding now preserve exact type/value semantics, and typed parameter failures retain catalog, reason, package and locus. Actual lead-confirmed Draco tests pass: two Decimal capture tests and the exact Repeat undefined/replay test; four unchanged FIX1 greens retained. All seven scoped tests pass without ignores. |

## Final bounded verdict

PASS for the reviewed PR scope at a1f45ef99cc1bfe555ca9925a400416c76d668e5. FND-001, FND-002 and FND-003 are fixed; original findings and prior dispositions remain unchanged. Code-review includes the Rust lane; gap-analysis remains the explicitly authorized diff-bounded manual trace/reverse-gap/stub review, without computed matrix or whole-repository assurance. Specification methods were skipped because no specification changed. The parent QSL-2271 remains In Progress; this verdict supports this PR's merge readiness only.
