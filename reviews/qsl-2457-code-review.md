---
id: SR-2026111612172400000046
title: "Code review — QSL-2457 red-main fixes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@213caccd08fda3cd2cd723354c3fbd8413e5f5b5; fsl-eval/tests/it/model_reference_queries.rs; fsl-replay/src/lib.rs, spine.rs, spine_eval_api.rs, spine/**/*.rs; fsl-semantics/src/check/state_model.rs; FR-513 row FR-015-AC-5, FR-100-AC-8, FR-122-AC-1/4, FR-266-AC-2, FR-268-AC-1 context"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-015
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-513
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-122
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-266
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-268
    type: references
---

## Summary

Reviewed the frozen QSL-2457 PR head across its fixture updates, state-check behavior, and replay facade aliases. The named fixes match their governing requirements, but the active FR-266-AC-2 test does not assert the required comparison-record contents.

## Verdict

**CONDITIONAL** — add a focused assertion for the comparison-record fields required by FR-266-AC-2.

## Scope and evidence

- Examined FR-015-AC-5: the dereference-only missing-field branch returns the required `ill_typed/type-mismatch`; the existing untyped-field path remains ahead of it.
- Examined FR-100-AC-8: the public spine types are named through replay-owned aliases, and the named TC-452 surface test passes.
- Examined FR-122-AC-1 and AC-4: active TC-517 tests assert replay outcome, identities, and document reads; the comparison-record cases use a producer-created record.
- Examined FR-266-AC-2: TC-741 checks basis and witness presence for decisive corpus cases, but not the specified record fields (FND-001).
- Read FR-268-AC-1 for context: its AllBelow, NestedAll, and FilteredAll scenarios are covered by TC-743, which the matrix marks as ignored under QSL-2173/QSL-2391; this is outside the ten named regressions.
- Read the repository instructions and relevant Rust API, implementation, safety, testing, and wire-boundary guidance. No applicable AssuranceProfile was found. No aggregate tests or builds were run by the reviewer; the coder reported all ten named filters passing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The active FR-266-AC-2 test checks only witness presence, not the mandated comparison-record contents | fsl-replay/src/spine/clause/tests/witness.rs:1620-1657; fsl-replay/src/spine/clause/tests/state_clause_replay.rs:263-283 |

## Finding detail

### FND-001 — medium

The active FR-266-AC-2 test reduces each decisive report's witness to `is_some()`, so it cannot detect a record with the wrong comparison operands or decision path. The replay tests build the expected record by calling `run_clause` on the same fixture, which is not an independent oracle for FR-266-AC-2's specified record content.


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The fix commit asserts the decisive ParentOrder record's comparison occurrence, ordered operands, and decision path. |

### FND-001 after excerpt

```rust
assert_eq!(record.deciding_comparison, Some(comparison.clone()));
let operands = record.operands.as_ref().expect("comparison operands");
assert!(crate::result::same_element(
    &operands[0],
    &integer(parent_version)
));
assert!(crate::result::same_element(
    &operands[1],
    &integer(child_version)
));
assert_eq!(
    record.decision_path,
    Some(vec![implication.clone(), comparison.clone()])
);
```
