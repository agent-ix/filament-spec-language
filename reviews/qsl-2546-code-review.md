---
id: SR-2026111923073200000001
title: "Code review of QSL-2546 PR #3121 probabilistic claim node body and path"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@3d593a9fd6c39de2d6ba3228e0008cb0b422a4d8; git diff origin/main...HEAD (PR 3121, merge-base adb00d353); fsl-semantics/src/check/probabilistic/node_key.rs, check/lowering/probabilistic.rs, check/probabilistic/claim.rs, check/mod.rs; fsl-package/src/emit.rs, emit/probabilistic.rs, emit/tests/probabilistic.rs, lib.rs; Cargo.toml contract-model pin; QSL-2546"
review_set: subset
date: 2026-11-19
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-498
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: reviews
---

## Summary

Ticket: QSL-2546. This reviews PR #3121 at `3d593a9fd`. The PR writes the nine-binding body of the `claim`/`probabilistic_claim` node (FSpec FR-640 "Probabilistic claim node") and spells the node_id path from the same `Statement` tree. It adds `node_key.rs` for the keys of the probabilistic nodes and what a claim reads. It removes `ClaimWorkload` and `Emission::claim_workloads()`, adds `EmitRefusal::UnresolvedClaimReward`, and bumps the contract-model pin to contract-ir `3febe56`. The Rust lane (rust-review) is folded into this file.

Evidence:
- `cargo-slot cargo test -p fsl-package --lib -- emit::tests::probabilistic` (`--test-threads=1` with the two house skips) ran the 45 PR tests green.
- Throwaway probe tests were added in the review worktree and never pushed:
  - `interval_alone`: claim `A` alone, `always[0,2] holds(true)` against `always[0,3] holds(true)`. Resolved node_id is **equal**.
  - `interval_order`: `A` (`always[0,2]`) and `B` (`always[0,3]`), declared A,B and then B,A. Both resolved node_ids **change**.
  - `raw_rows_permutation`: the three-claim fixture, forward against reversed. Raw rows and raw nodes are byte-equal.
  - `each_when_true`: `on each (m)` against `on each (m) when (true)`. Same id.
  - `rational_spellings`: `0.5`, `rational(1, 2)`, `rational(2, 4)` and `decimal(50, 2)`. Same id.
- `hybrid_claim_hyper_expression_refuses_at_s3` was run on the merge-base worktree and on the head (results in the Verdict).

## Verdict

REQUEST CHANGES.

What holds up:
- One `Statement` tree drives both the S3 path spelling and the S4 body, so path and body cannot drift. The binding order, the `value_kind`/unit-operand/canonical-text spelling and the omission of integer/rational types all match FR-640.
- Dimensionless spellings collapse correctly (probe). `on each` with no `when` gives the same id as `when (true)` (probe).
- Confidence order is fixed alpha/beta/indifference. Fairness members are sorted and deduplicated in (declaring, operation, kind, granularity) order, and `whole` is the default.
- Weight-line order is canonicalized in the workload key.
- Reward, distribution and delay reads are filtered to the subject's model and sorted.
- Ordinals rank by (name, full path) then written content, which equals FR-498's "claims of that package and path by written content" because the path starts with the name.
- `ClaimWorkload` and `claim_workloads()` have no consumers in this repo or in sibling repos under ~/dev, so their removal is safe.
- `UnresolvedClaimReward` is `runtime_invariant`. No new `#[allow]`. The FSpec- trace prefix matches the repo convention.
- The un-ignored FR-640-AC-3 test is a real assertion.
- Raw rows are byte-identical under permutation for the existing fixture (probe), so the switch to resolved-id comparison masks no current defect.

What blocks:
- A claim's identity does not cover its event formula's interval (FND-001).
- When two claims' formulas differ only in an interval, each claim's identity depends on declaration order (FND-002). This breaks FR-498's reordering rule and FR-640-AC-3.
- The PR body describes FND-002 as a cross-package case only (FND-009).

Other checks:
- The claim that `hybrid_claim_hyper_expression_refuses_at_s3` is red on base is confirmed. It fails on merge-base `adb00d353` and on the head at the same site (hybrid/tests.rs:71), and the PR touches no hybrid code.
- `RUSTFLAGS='--cap-lints warn' cargo clippy -p fsl-package -p fsl-command` shows only the inherited `result_large_err`/`large_enum_variant` classes on the touched files. No new lint classes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The claim path names the event's `temporal`/`formula` node by its key, but `walked_operands` never walks the `TemporalInterval` operation member. So `always[0,2] holds(true)` and `always[0,3] holds(true)` derive one base path, and a claim alone in its package keeps the same node_id when its interval changes (probe `interval_alone`: equal). FR-498-AC-7 requires "its event to `always[0,3] holds(true)`" to change `A`'s node_id. The PR's test uses `eventually[0,2]`, which changes the operator identity, so it passes without exercising this case. | fsl-semantics/src/check/lowering.rs:7789-7843; fsl-semantics/src/check/lowering/probabilistic.rs (formula_statement); fsl-package/src/emit/tests/probabilistic.rs:2556-2603 |
| FND-002 | high | Formulas that differ only in an interval share a base path and take ordinals in minting order, and claims are lowered in source order. A package with `A` (`always[0,2]`) and `B` (`always[0,3]`) therefore gives both `A` and `B` a different node_id when the two declarations are swapped (probe `interval_order`: both differ). This breaks FR-498 ("reordering the unit's declarations SHALL leave every claim's {declaration, occurrence} unchanged") and FR-640-AC-3 ("Writing the four claims ... in another order changes no byte"). The PR's reorder test only permutes claims with identical formulas. | fsl-semantics/src/check/lowering.rs:2705-2729,7789-7843; fsl-semantics/src/check/lowering/probabilistic.rs:96-118 |
| FND-003 | medium | `window_statement` writes an `integer` `within`/`window` whenever no timed horizon was written. S3 `steps()` admits a step horizon (`within 10`) over a timed subject without refusing. FR-640 rule 7.3 requires a time `quantity` over a timed subject, so the producer would emit a package the strict reader refuses (or emit a body that disagrees with FR-640) for a step-horizon claim on a timed model. Not reproduced: no timed fixture in this lane. | fsl-semantics/src/check/lowering/probabilistic.rs:603-616; fsl-semantics/src/check/probabilistic/claim.rs:1846-1852 |
| FND-004 | medium | `weighted by delay` reaches S3 as the reward name `delay` (`WeightingForm` doc: "`delay` stays the spelled name"). Without a reward named `delay`, S3 refuses `UnresolvedReward`, which is honest. If the unit declares a reward named `delay`, the body writes `weighting kind reward` with that reward's nodes, where FR-407/FR-640 read delay weighting (`kind delay`, empty `reward`). That is a silent wrong identity and body. The case is pre-existing S3 semantics, but this PR writes it onto the wire; it belongs in QSL-2551's scope. | fsl-semantics/src/check/lowering/probabilistic.rs:618-623; fsl-semantics/src/check/probabilistic/claim.rs:1893-1904; fsl-semantics/src/check/probabilistic/node_key.rs (reward_read) |
| FND-005 | medium | `the_reader_refuses_a_claim_statement_that_breaks_rule_7` asserts only `Read::Refused(_)`, with no code, rule or locus. Its "a probability of another kind" edit rewrites `value_kind` to `integer` while keeping the value `1/2`, which FR-322 "Scalar literal values" refuses before rule 7, so it never exercises rule 7.3. It also carries a dead `let wire_text = ...; let _ = wire_text;`. `the_reader_refuses_a_malformed_claim_body` likewise asserts only `Refused(_)`. | fsl-package/src/emit/tests/probabilistic.rs:2661-2704 |
| FND-006 | low | `emitted_claims` now resolves row and node ids before comparison, so `assert_eq!(forward_rows, reversed_rows, "the rows are byte-identical")` no longer checks bytes (FR-498-AC-1). Raw rows are byte-equal today (probe), so nothing is masked, but the assertion message overclaims and a regression in the raw order would pass. | fsl-package/src/emit/tests/probabilistic.rs:137-165,267 |
| FND-007 | low | `ClaimReadFault::Subject` (no model_root key, or a fairness operation with no model node) is reported as `EmitRefusal::SourceMapInvariant`, which names the wrong invariant in the diagnostic. | fsl-package/src/emit/probabilistic.rs:622 |
| FND-008 | low | The public API is widened with no consumer outside the crate: `CheckedProbabilisticClaim::declaration_location` went from pub(crate) to pub, `CheckedPredicate::slot_names` and `CheckedTemporalFormula::conditions` became pub, and `ClaimActivation::Each` gains a pub field (a breaking change for external struct-variant constructors). `Statement`, `ClaimReads` and `SchedulerRead` are exported with pub fields and are not `#[non_exhaustive]`. | fsl-semantics/src/check/probabilistic/claim.rs:466,528; fsl-semantics/src/check/mod.rs:202-210 |
| FND-009 | medium | The PR body says "two claims differing only in `always[0,2]` vs `always[0,3]` share a path in separate packages", and QSL-2641 adds "within a package the ordinal separates them". It omits that within one package that ordinal follows source order (FND-002) and that FR-498-AC-7's interval case fails (FND-001). The partial framing understates a conformance failure of ACs this PR tags as covered. | PR #3121 body "Not in this PR"; QSL-2641 |

