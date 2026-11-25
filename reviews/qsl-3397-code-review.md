---
id: SR-967175026302197768900474181941935084869
title: "Code review — QSL-3397 active parallel binder scopes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-3397; PR #3972; fsl-semantics/src/check/protocol_clause/binders.rs, fsl-semantics/src/check/protocol_clause/tests/graph.rs"
review_set: subset
---

## Summary

Reviewed PR #3972's binder-scope change and its regression coverage against FR-513-AC-12 and the FR-113 sibling-scope acceptance cases. No code defects were found in the changed scope; the lead's final aggregate gate remains pending after review.

## Verdict

**PASS** — the implementation refuses a duplicate at the later binder in concurrently active parallel branches, while retaining the tested choice and sequential reuse behavior. Focused test receipts are from the coder; the reviewer did not repeat them or run the aggregate gate before review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Changed files examined:

- `fsl-semantics/src/check/protocol_clause/binders.rs`
- `fsl-semantics/src/check/protocol_clause/tests/graph.rs`

The production rule uses the source-ordered binders and reports at the later binder. Scope overlap retains the existing enclosing-scope prefix behavior and adds only the case where the first differing control scopes are sibling `Branch` scopes owned by a `parallel` declaration. The test checks the later binder's source span, the `Shadow` cause naming the binder, and the absence of the prior spurious `Finish`/`AmbiguousName` refusal on literal `true`.

The coder reported these filtered checks passing on the candidate before the final merge with current `origin/main`: the exact QSL-3397 regression, FR-113-AC-13 choice reuse, and FR-113-AC-11 sequential reuse. `git diff --check` passed. Final aggregate gate is deferred to the lead after this review, per the review-before-gate sequence.

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
