---
id: SR-2026111612172400000009
title: "code review (with rust-review) of PR #2794 (QSL-2323): enforce S3 refusal for hyper divisors"
type: SpecReview
analysis: code-review
date: 2026-11-16
scope: "agent-ix/filament-spec-language; QSL-2323; PR #2794 at a01bac80b3ef039afa72d1abe9a314dfa7c0b9a4; fsl-semantics/src/check/hyper_clause.rs and changed hyper integration tests"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-487, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-495, type: references }
---
# SR-2026111612172400000009: code review of PR #2794

## Summary

Ticket: QSL-2323. Frozen head `a01bac80b3ef039afa72d1abe9a314dfa7c0b9a4`, compared with `origin/main...HEAD`. Rust-review lane folded in. No Cargo commands were run as directed. The seven-file diff is limited to one S3 source mapper change plus hyper fixture updates. The PR reports targeted tests passing; those results are attributed to the PR, not reviewer-rerun.

The source span correction selects the first divisor found under a root. For match conjunct lowering, the refusal location is normalized to the hyper site, so it does not identify the failed conjunct's expression path. If the match contains multiple divisions, the first divisor in the full match expression may be selected even when a later divisor caused the refusal. Also, a test changed from checking FR-487-AC-3's runtime `Undefined` behavior to S3 refusal but retains the FR-487-AC-3 trace tag.

The remaining five `hyper_replay` candidates, projected replay case, and witness replay case remain parked for QSL-2330. Kernel function-body cases remain QSL-2329. No parked case is treated as passing. The hand-modelled empty-set case has no clause divisor and is outside this change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | For match clauses with multiple division nodes, the refusal span can point to the first divisor in the full clause rather than the divisor whose nonzero proof failed. | fsl-semantics/src/check/hyper_clause.rs:1905-1908,2006-2029 |
| FND-002 | medium | The rewritten S3-refusal test still carries FR-487-AC-3, whose acceptance criterion requires runtime atom evaluation and an `Undefined` result. | fsl-analyze/tests/it/hyper_lowering.rs:155-164 |

## Verdict

**CONDITIONAL.** Both findings are bounded to the new diagnostic-span selection and test trace binding. The existing refusal category is preserved, declared clauses remain Linked, and the scope boundaries in the PR body match the requested routing. The focused test inventory and reported outcomes were reviewed as claims only; no Cargo suite was run.

## Examined units

| ID | Path | Role |
| --- | --- | --- |
| FR-487-AC-3 | spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md | examined |
| FR-487-AC-5 | spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md | context_only |
| FR-495-AC-3 | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md | context_only |

## Review details

FND-001: `lower_conjunct` wraps the underlying refusal with the hyper `location`; `body_refusal_at` then attempts `node_at(expression, location.path())` and falls back to the expression root. `unproved_nonzero_divisor_span` returns the first division operator's right-hand expression encountered. Existing changed cases have one relevant divisor, so their assertions do not exercise the multiple-divisor match case. The finding concerns exact location only; the refusal remains `undefined_expression/unproved-nonzero`.

FND-002: FR-487-AC-3 specifies both completed true/false atom readings and an `Undefined` result for an unproved divisor. The edited test now only asserts an S3 refusal. The other unchanged `an_atom_is_evaluated_over_its_components_and_is_deterministic` test covers the completed true/false readings, but not the AC's undefined-result clause. The trace should remain attached only if another test still verifies that clause.

No CI workflow, dependency, lint, or gate files changed. `git diff --check` was not run because the dispatch forbade tests but did not prohibit read-only checks; review was based on diff and source inspection. No applicable AssuranceProfile was identified in the scoped files.

### Finding excerpts

| ID | Excerpt |
| --- | --- |
| FND-001 | `if matches!(&refusal.cause, CheckCause::Unproved(Obligation::Nonzero)) { let root = lowering::node_at(expression, &path).unwrap_or_else(|| expression.root_id()); if let Some(divisor_span) = unproved_nonzero_divisor_span(expression, spans, root) { span = divisor_span; } }` |
| FND-002 | `#[trace("TC-1264", "FR-487-AC-3")]` above `an_unproved_hyper_atom_divisor_is_refused_at_s3`, which now calls `assert_hyper_division_refused` instead of evaluating an atom. |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The occurrence span is now passed into `unproved_nonzero_divisor_span`: `if let Some(divisor_span) = unproved_nonzero_divisor_span(expression, spans, span) { span = divisor_span; }`; the added multi-divisor regression asserts the later unproved divisor's span. |
| FND-002 | fixed | The refusal-only test now has the corrected trace tag `#[trace("FR-495")]`. |

