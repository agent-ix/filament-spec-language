---
id: SR-2026111612172400000050
title: "Code review — FSL function claim outcome serialization"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-command/src/command/output.rs, fsl-replay/src/clause_evaluation.rs, fsl-replay/src/execute/state_clause.rs, fsl-replay/src/outcome.rs, fsl-replay/src/spine/call.rs, fsl-replay/src/spine/call/tests.rs, fsl-replay/src/spine/native_result.rs, fsl-replay/src/spine/refinement.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-867", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-286", type: references}
---

## Summary

Reviewed PR #2982 at `77c01dbf5cf9c5410e65b57d827faa3294629684`, including the Rust implementation and its focused serializer tests against FR-867 and FR-286. The PR introduces a third local expression-location wire representation, leaves two existing QSL-2201 tests asserting the old undefined-cause shape, and does not fully exercise the refusal item contract in its new tests.

## Verdict

**FAIL** — the duplicate wire codec is a high-severity P0 finding; the stale tests and incomplete acceptance assertions also need resolution.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new expression-location serializer duplicates the existing `LocationWire` codec and its exhaustive owner mapping, creating another source of truth for the same declaration/path wire shape. | fsl-replay/src/outcome.rs:386, fsl-replay/src/result/wire.rs:1655 |
| FND-002 | medium | Two QSL-2201 unit tests still require `undefined-evaluation.code` and absence of `undefined-evaluation.cause`; this change now writes the evaluator reason under `cause`, so those tests will fail when run. | fsl-replay/src/spine/clause/tests/item_cause.rs:16-17, fsl-replay/src/spine/clause/tests/item_cause.rs:37-39 |
| FND-003 | medium | The integer, bounded, and unknown claim tests assert `diagnostic` only; they do not assert the item `cause` or exercise the specified `OutcomeDocument::from_clause_runs` path, so the FR-867-AC-4 cause/document contract can regress while these tests pass. | fsl-replay/src/outcome.rs:3285-3332 |

## Review Scope

- FR-867-AC-1 through AC-4 and TC-2951.
- FR-286-AC-4 and AC-11, plus the existing QSL-2201 item-cause tests.
- All eight changed Rust files listed in the frontmatter scope.
- The expression-location codecs in `fsl-replay/src/result/wire.rs` and `fsl-replay/src/spine/native_result.rs` were searched for duplicate representations.

## Rust Review

The focused gate reported for this head passed all four `claim_item` tests. No broad or package-wide gate was run under the targeted-test-only instruction. The four tests initially exercised real serialization but left the refusal cause and document-construction assertions incomplete; FND-003 records that review finding.

## Dispositions

Disposition pass 1 rechecked PR #2982 at `36a0eeb9447c92d9e91ef6428d939672a7372134`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The item serializer now reuses the existing `LocationWire` serialization through `expression_location_json`. |
| FND-002 | fixed | The stale shared-cause assertions now check `cause` and its absence of `code`; the related clause-path assertions were also updated. |
| FND-003 | fixed | Claim tests now construct `OutcomeDocument::from_clause_runs` and assert each refusal item's cause as well as diagnostic details. |

- FND-001 after_excerpt: `pub(super) fn expression_location_json(location: &Location) -> serde_json::Value { serde_json::to_value(LocationWire::of(location)).expect("expression location wire form is serializable") }`
- FND-002 after_excerpt: `assert_eq!(value["cause"]["cause"], "division-by-zero"); assert!(value["cause"].get("code").is_none());`
- FND-003 after_excerpt: `let document = OutcomeDocument::from_clause_runs(Some(OutcomeStage::S6c), [("claim", &report)]).expect("refusal and undefined dispositions settle as items"); json_of(&document)`
