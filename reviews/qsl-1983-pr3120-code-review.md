---
id: SR-2026111919340478491681409
title: "Code review — optional operation parameters"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/model/intake.rs, fsl-semantics/src/model/normalize.rs, fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/check/typing.rs, fsl-semantics/src/check/state_model.rs, fsl-semantics/src/model/observation/document.rs, fsl-replay/src/spine/clause/tests/invocation_parameters.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-056", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-103", type: references }
---

## Summary

Reviewed PR #3120 at frozen head `e1d35d0aa8b977b5b6de3c672fd5f65f5507e3d8`, including the Rust implementation, focused tests, and unrelated replay adjustments. The required clauses of the optional-parameter implementation are not all exercised by tests; no build or test gates were run per the dispatch brief.

## Verdict

**CONDITIONAL** — the optional parameter’s clause typing and presence obligations required by FR-103-AC-12 have no tests in this change or the current tree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Optional operation parameters lack tests for the required Option presence and dereference semantics | fsl-semantics/src/check/assemble.rs:2988-2992 |

## Scope

- `FR-056-AC-65` — examined: intake and type-environment behavior for optional operation parameters.
- `FR-103-AC-12` — examined: operation clause typing, omitted-call lowering, and arity refusals.
- `TC-3040` — context only: prescribed end-to-end checks for optional operation parameters.
- All 27 changed files were read for implementation, tests, and replay-forward adjustments. No Cargo manifests or CI workflows changed. `git diff --check` was clean.

## Findings detail

The new assembler wraps an optional parameter's value type in `Option<X>`, and the new test only checks dispatch arity and invocation admission. There is no test for `present(p)`, guarded `value(p)`, unguarded `value(p)` refusing `unproved-presence`, or `deref(p)` refusing `type-mismatch`, although these are explicit FR-103-AC-12 behaviors and are the core semantic consequence of this changed type construction. The checked-call test also does not assert the required omitted-versus-explicit-`absent` checked-node equality. Add focused tests for these cases before treating the clause semantics as reviewed evidence.
