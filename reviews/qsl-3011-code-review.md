---
id: SR-967175026302197768900474181941935084855
title: "Code review — QSL-3011 present Option history lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3895 final head 9bc2d7e0a67dd9d4a50104af40b491577aaff6e4; initial review head 4420fe4e5c746dd0731e98a7c90a972c3d7d4ba2 against origin/main a2677a0e0aaf8763be74c6b10be5f7b8e384f5ac; fsl-semantics/src/check/lowering/refinement_rows.rs; fsl-semantics/src/check/mod.rs; fsl-semantics/src/check/refinement.rs; fsl-semantics/tests/it/refinement_declarations.rs; FR-138-AC-2/9; TC-543"
review_set: subset
---

## Summary

Ticket: QSL-3011. Re-reviewed the final PR head after merging refreshed `origin/main`. The nested `option_value` lowering, refinement admission, test coverage, and FR-138 resolution are preserved. No correctness findings were found; the new test block has one low formatting finding.

## Verdict

**CONDITIONAL** — the behavior and conflict resolutions are sound, but the new assertions in `refinement_declarations.rs` do not match the repository's Rust formatter output.

## Rust review

`history_initial` unwraps each present Option/value-type pair, retains the corresponding option types, lowers the final payload using its own type, then wraps the payload in reverse order. This preserves nested `option_value` structure and keeps enum payloads inline while using typed literal/reference lowering for other payloads. The `history_initials_have_literals` emission gate and its `v2_emittable` restriction are removed, so a present Option history is no longer refused before row lowering. The tests cover enum, integer and decimal payloads, the out-of-set refusal at its literal, and a present Option history coexisting with a terminal. FR-138 now records `HistoryRow.initial: Value` and removes the obsolete current-gap note.

The final `origin/main...HEAD` diff contains only the six QSL-3011 files. `HEAD` is a merge commit whose second parent is the refreshed `origin/main`; unrelated main changes in the merge parent do not appear in the PR comparison. No public API, dependency, wire-format, unsafe-code, or async behavior changes were introduced.

## Scope examined

- `fsl-semantics/src/check/lowering/refinement_rows.rs`
- `fsl-semantics/src/check/mod.rs`
- `fsl-semantics/src/check/refinement.rs`
- `fsl-semantics/tests/it/refinement_declarations.rs`
- `spec/functional/FR-138-check-and-compute-history-fields.md` — FR-138-AC-2 and FR-138-AC-9.
- `spec/test-cases/TC-543-history-fields-check-and-compute-without-changing-the-concrete-model.md` — added package-route step 8 and expected result.
- Previous clean code and base spec reviews at `a6a0f0dc7bc0365b83ff795c2b991cf557c7a3ab`; no prior findings require dispositions.

## Verification

- `git diff --check origin/main...HEAD` — exit 0.
- `quire validate --scope . spec/functional/FR-138-check-and-compute-history-fields.md spec/test-cases/TC-543-history-fields-check-and-compute-without-changing-the-concrete-model.md` — exit 0.
- Changed-file `rustfmt --check --edition 2021 --config skip_children=true` reports three formatting differences in the added test block at `refinement_declarations.rs:2331-2358`. Its other reported differences are in unchanged main code. No files were modified.
- Cargo tests and aggregate gates were not run; the code lead owns those gates and QSL-3311 owns the locked-build queue.
- The PR-scope static matrix fallback reports FR-138-AC-2 and FR-138-AC-9 as tagged, including both new tests. `quoin matrix --repo . --json` could not provide evidence-backed output: quoin 0.36.8 refused the request as too large. The static matrix was produced with quire 0.52.0 / engine 0.62.2.
- No applicable AssuranceProfile was found for this behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The added Option history test assertions are not rustfmt-formatted | fsl-semantics/tests/it/refinement_declarations.rs:2331, 2345, 2357 |

## Disposition review round 1

Checked fix commit `9bc2d7e0a67dd9d4a50104af40b491577aaff6e4` at final PR head `9bc2d7e0a67dd9d4a50104af40b491577aaff6e4`. Its only change from the reviewed head reformats the reported test assertions and reference lookup. The changed-file rustfmt check and fix-commit diff check pass. The earlier findings and conflict-resolution conclusions remain unchanged.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Applied the formatter's layout to the reference lookup and Option payload assertions; the changed test file now passes rustfmt check. |

### FND-001 after excerpt

```rust
let literal = graph
    .semantic_graph()
    .node(
        target
            .admitted()
            .expect("literal reference is admitted")
            .clone(),
    )
    .expect("literal node exists");
```
