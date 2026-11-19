---
id: SR-2026111612172400000054
title: "Code and Rust review of QSL-1901 Compile refusal region (filament-spec-language PR 2698, post-merge)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@d1c76b6be3e37fb556afbabfd8ed662cf51eeaf6; git diff origin/main...HEAD (PR 2698, branch qsl-1901-equality-postcondition, 5 files); fsl-replay/src/call_site.rs, fsl-replay/src/proof_result.rs, fsl-replay/src/spine/clause/tests/postcondition_range.rs, reviews/qsl-1901-code-review.md, reviews/qsl-1901-gap-analysis.md"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-1901. Second review of PR 2698 after the coder merged main, which had
landed its own FR-577 checker (#2817, #2931). The diff that remains is small.
`CallSiteRefusal::Compile` gains `region: Option<SourceRegion>`, and both
production construction sites in `From<CompileRefusal>` fill it from
`CompileRefusal::region()` / `UnitListRefusal::region()`. One unit test adds
`region: None`. A new test, traced to TC-1416 and FR-121-AC-37, checks the span.
The `refusal()` helper gets a `..` rest pattern. The PR also carries the stale
prior-round SR files. The rust-review lane is folded in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Adding a named field to the public `fsl_replay::CallSiteRefusal::Compile` variant breaks source compatibility for consumers. filament-contract-codegen (CF-CG-01 consumer of `call_site`, main 7f24256e) matches `CallSiteRefusal::Compile { code, message }` with no rest pattern at src/kani/terminal/constrained/tests.rs:200 and builds `CallSiteRefusal::Compile { code, message }` at tests/it/terminal_map.rs:634. The next CG pin bump onto an FSL main that contains this PR will fail to compile, and the PR body does not mention it. The change is spec-mandated (FR-121-AC-37), so it should not be reverted. File the CG follow-up (add `..` and `region: None`) with the merge, or say in the PR body that the CG bump carries it | fsl-replay/src/call_site.rs:318-327 |
| FND-002 | low | The PR's title ("discharge a postcondition's range obligations from its operation's preconditions (FR-577)") no longer describes the diff, because main landed FR-577. The two committed `reviews/qsl-1901-*.md` files are prior-round SRs. Their scope reads `agent-ix/quire-spec-language`, they cite `qsl-*` paths and commits b1fa7352c..ade95781e, and their ids are SR-2698001/002. They review code this PR no longer changes. Drop both files from the PR, or replace them with this round's SRs. Then retitle the PR to the region change | reviews/qsl-1901-code-review.md:1-8; reviews/qsl-1901-gap-analysis.md:1-8 |

## Scope and evidence

- Every construction site was checked. A grep over the workspace for `(Self|CallSiteRefusal)::Compile` finds two production constructors, call_site.rs:661 (`UnitList`) and call_site.rs:688 (every other source/check/emit arm). Both set `region: refusal.region().cloned()`, so no production site defaults to `None`. The only `region: None` is the `TerminalValue::from_call_site_refusal` unit test at proof_result.rs:3506, where the region is irrelevant. Every other match (call_site.rs:509, proof_result.rs:1463, execute/tests.rs:1421, both postcondition_range.rs uses) uses `..`.
- `CompileRefusal::region()` (fsl-replay/src/spine.rs:688-714) is pre-existing on main. It returns the `Check` arm's region and the first `CheckDiagnostics` region, so the field's doc ("the region the spine's CompileRefusal names for its first refusal") is accurate. The `Intake`, `Import` and `Dependency` arms map to other `CallSiteRefusal` variants that carry no region. That is unchanged and outside FR-121-AC-37.
- Rust lane: no `unsafe`, no new panics in library code, no `#[allow]`. The new test's `region.start() as usize` is a u64-to-usize cast in test code only. The test is tagged `#[trace("TC-1416", "FR-121-AC-37")]`, and the bare ids are FSL's own, which is correct under main's NFR-140.
- The head merges cleanly with origin/main c85753e0a (two commits ahead, no overlapping files; `git merge-tree` is clean, GitHub reports MERGEABLE).
- Focused tests and the ticket-acceptance check are in the gap-analysis SR (SR-2026111612172400000055).
- Inherited on main, not PR findings: the earlier O(posts x records) finding no longer applies. Main builds `redefinition_family_keys` once and keys precondition facts by (context, operation) in a `BTreeMap` (fsl-semantics/src/check/mod.rs:1693-1738, 3428-3456). One cosmetic inherited nit: a stray doc paragraph ("A family `check`'s stage limit as a checking refusal's cause ...") sits above `clause_preconditions`' own doc (fsl-semantics/src/check/mod.rs:1687-1692).

- FR-121-AC-37 (spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md): call_site refuses CallSiteRefusal::Compile whose region is Some, names the unit's source and spans exactly the bytes of `pre(self.balance) - amount`, code undefined_expression. (examined)
- FR-121 (same file): locate a function call site through the replay facade; consumer fact CF-CG-01. (context_only)

## Verdict

The region is wired through every Compile construction site, and it matches
main's FR-121-AC-37. No correctness defect. FND-001 is a downstream coordination
item, not a defect in this repository. The PR can merge once the stale review
files are dropped or replaced (FND-002) and the CG follow-up is recorded (FND-001).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 5d190a286: the PR body now has a "Downstream note" naming filament-contract-codegen main 7f24256e, `src/kani/terminal/constrained/tests.rs:200` and `tests/it/terminal_map.rs:634`, and stating that the field is spec-mandated (FR-121-AC-37) and the codegen follow-up is routed to its owner. No code change in this repository, as intended. |
| FND-002 | fixed | 5d190a286: the PR is retitled "QSL-1901: carry the unproved node's region on call-site compile refusals (FR-121-AC-37)". The stale `reviews/qsl-1901-code-review.md` and `reviews/qsl-1901-gap-analysis.md` are removed, and `reviews/qsl-1901-pr2698-*.md` are committed byte-identical to the review-pass SRs. |
