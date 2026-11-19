---
id: SR-2026111612172400000015
title: "Code review — QSL-2336 checked-node admission corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2800; fsl-package/src/emit/tests/admission_corpus.rs; FR-093-AC-19; TC-416 step 11"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-416
    type: references
---

## Summary

Reviewed PR #2800 at `cac6ae7ed41fb0c41b2767cdaed3adbe035973cd`. The one-file test change classifies the four IR probabilistic model node forms as intentionally not emitted, matching the emitter's existing `ProbabilisticModelNotEmitted` refusal; no defect was found.

## Verdict

PASS — the focused tests and formatter checks pass, and the new classifications preserve the closed, exactly-once node-kind audit required by FR-093-AC-19.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined Scope

- `FR-093-AC-19` — role: examined; `spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md`; excerpt: “Every IR checked-package/v2 node kind (`node_tag`, `semantic_form`) is classified exactly once: as a family FSL's lowering writes, or as a kind FSL's lowering never writes, with the reason. For each family FSL writes, a package holding a node of it is admitted by IR's checked-package/v2 reader, through FSL's I2 read, as emitted.”
- `TC-416` step 11 — role: examined; `spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md`; excerpt: “Classify every kind of IR's `CheckedNodeKind::all()` as a family FSL writes (admitted, or one of the two cases of FR-093-AC-19) or as a kind FSL never writes, with its reason.”

## Coverage

The changed file adds exactly four `ProbabilisticForm` kinds to the existing not-emitted table. The corpus combines emitted fixture rows, explicit not-emitted rows, extension kinds and typed timed-model kinds, then asserts there are no unclassified, multiply classified or unknown kinds. The corresponding emitter path rejects probabilistic model declarations before node emission; probabilistic claims remain separately classified. This aligns the edit with the ticket's stated IR-1549 root cause and with FR-093-AC-19.

The computed `quire matrix --scope . --format tsv` marks FR-093-AC-19 `tagged` by this test. The three ticket acceptance filters each passed 1/1 on the reviewed head; the additional package regression passed 1/1. `cargo fmt --all -- --check` and `git diff --check` passed. No aggregate gate was run. No applicable `AssuranceProfile` was found under `spec/`.

No production code changed, so a separate implementation-gap analysis was not applicable. The installed `quoin` CLI has no `gap-analysis` command (`quoin gap-analysis --help` returned “command gap-analysis not found”); the changed acceptance criterion was checked directly against the implementation and its traced test. Full repository reverse-gap analysis is not claimed. Plan completion: not assessed.
