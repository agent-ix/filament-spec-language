---
id: SR-967175026302197768900474181941935084870
title: "Gap analysis — QSL-3397 active parallel binder scopes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-3397; PR #3972; diff-scoped implementation, tests and owning criteria"
review_set: subset
---

## Summary

The changed behavior is owned by FR-513-AC-12 and has a new correctly tagged regression test in the changed graph test. No gap was found between that criterion, the tested refusal, and the implementation. The repository-wide Quoin reverse-gap/stub audit could not complete, so this is a diff-scoped result rather than a repository-wide assurance claim.

## Verdict

**CONDITIONAL** — no changed-scope gap was found; a full reverse-gap and stub sweep remains incomplete because the installed Quoin matrix command failed. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No changed-scope gap found (placeholder) | - |

## Coverage

Plan completion: not assessed.

`quire matrix --scope /home/peter/dev/worktrees/qsl-3397-fsl --format json` completed with exit 0. In the computed matrix, FR-513-AC-12 is `tagged`; `fsl-semantics/src/check/protocol_clause/tests/graph.rs` is listed as a binder for the criterion after the PR adds its trace tag. The focused code-to-spec check found that the new behavior is the simultaneous-active-binder branch already stated in FR-513-AC-12; no new requirement was introduced. No new stubs, weakened tests, or behavior without an owning criterion were found in the two changed files.

The required full-repository reverse-gap and stub review was not completed. `quoin matrix --repo /home/peter/dev/worktrees/qsl-3397-fsl --json` failed with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size` using Quoin 0.36.8. The complete full-repository audit therefore remains unverified. The matrix emitted existing unbacked-claim warnings outside the changed test; no unrelated baseline claims are attributed to this PR.

## Examined units and bindings

```yaml
clean: true
scope:
  - id: FR-513-AC-12
    path: spec/functional/FR-513-name-the-successor-or-keep-of-each-retained-criterion-that-rides-a-retired-path.md
    role: examined
    excerpt: "A same-spelled name of another owner is not a visible-entry case: if its binder is simultaneously active, the later binder refuses `ambiguous_declaration`/`ambiguous-name` at that binder, naming the earlier active binder (FR-113); disjoint or ended binder scopes may reuse a name."
  - id: FR-113-AC-11
    path: spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md
    role: examined
    excerpt: "Because the sequence passes Inner's output to the later step, After refuses ambiguous_declaration/ambiguous-name at its binder declaration naming Nested's still-active binder."
  - id: FR-113-AC-13
    path: spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md
    role: examined
    excerpt: "The protocol checks without ambiguous_declaration: neither sibling case's binder is active at the other's declaration."
findings: []
bindings: []
```
