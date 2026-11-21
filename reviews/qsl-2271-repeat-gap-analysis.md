---
id: SR-1795266708254148051
title: Ordinary package Repeat gap-analysis for PR 3304
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2271; frozen f1c43209be6b2344addb1048c353120a70249369..3695a73b696a38da7a3df762e9e45b989e49806a;
  seven changed Rust files
review_set: subset
---

## Summary

Bounded, planless gap review. Initial review of the single frozen commit. One blocking fixture defect found. No implementation edits or Cargo execution.

## Verdict

**FAIL**: FND-001 blocks the claimed test evidence. Production Repeat mapping was inspected without finding another concrete defect; its runtime correctness is not certified.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The five new trace-tagged Repeat tests do not provide executable evidence for package Repeat parity or proof refusal: their shared void-touch fixture reads its unreadable Boolean binder and is rejected at source S3. The earlier-Receive, bounded-exhaustion, post-test, undefined-guard and proof-refusal scenarios therefore cannot reach their intended assertions. Repair the fixtures and rerun all five tests without weakening the expected records, ordering, replay or refusal assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:59; fsl-analyze/tests/it/fill.rs:203; fsl-analyze/tests/it/fill.rs:573; fsl-semantics/src/check/protocol_slots.rs:1188 | correct-requirement-no-evidence |

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
