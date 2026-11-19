---
id: SR-20261119000000000001
title: "Code and Rust review of QSL-2125 PR #3026"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@8983ead8d161765c8b5fddbd2221022162ad5460; PR #3026 diff: fsl-analyze/src/analyze.rs, fsl-analyze/tests/it/analyze_reach_rulings.rs, fsl-analyze/tests/it/analyze_request_wire.rs, fsl-command/src/command.rs, fsl-command/src/command/wire.rs, fsl-semantics/tests/it/state_graph_claims.rs, tests/it/compile_package_output.rs, xtask/src/refinement/layering.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-489
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2215
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2291
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2297
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1545
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1272
    type: reviews
---

## Summary

Ticket: QSL-2125. Reviewed PR #3026 at `8983ead8d161765c8b5fddbd2221022162ad5460`, including the QSL-2269, QSL-2293, and QSL-2309 changes and the language slice of QSL-2125. The Rust lane reviewed the changed request parsing, path handling, package handoff fixture, and linked-engine test updates. One edge case remains in the output/handoff containment check: lexical `Path::starts_with` does not detect an output path that traverses through a sibling directory and resolves inside the handoff directory.

## Verdict

**CONDITIONAL** — the focused state-graph, normalization-limit, registry, profile, and output/handoff tests cover the intended changed paths, but an output path such as `existing/../handoff/pkg.json` bypasses the containment refusal while resolving inside the newly written handoff directory.

## Assurance Context

No applicable AssuranceProfile for the changed code or behaviors was found under `spec/`.

## Scope

- `FR-620-AC-11`: examined the refusal and admission cases for `hashed_bytes` in `fsl-analyze/tests/it/analyze_request_wire.rs`.
- `TC-2215`: examined the test mapping and trace tag for the normalization-limit refusal.
- `FR-628-AC-4`: examined the package-selected Kernel-checked fixture, typed instance key, undefined arm, and locus assertions in `fsl-analyze/tests/it/analyze_reach_rulings.rs`.
- `FR-628-AC-5`: examined the package-selected state-graph budget-stop assertions in the same test.
- `TC-2291`: examined the test mapping and both trace tags.
- `FR-810-AC-5`: examined independent output/handoff writes and request-stage refusal/no-write assertions in `tests/it/compile_package_output.rs` and the implementation in `fsl-command/src/command.rs`.
- `TC-2297`: examined the integration-test mapping and trace tag.
- `FR-165-AC-7`: examined the newly added temporal-complete profile acceptance test in `fsl-semantics/tests/it/state_graph_claims.rs`.
- `TC-1545`: examined the test mapping and trace tag.
- `FR-489-AC-5`: examined the updated registry assertion and layering documentation in `fsl-analyze/src/analyze.rs` and `xtask/src/refinement/layering.rs`.
- `TC-1272`: examined the registry test's trace binding.
- `fsl-command/src/command/wire.rs`: examined the removed `hashed_bytes` deserialization field and closed-record behavior.
- `fsl-analyze/tests/it/analyze_reach_rulings.rs`: examined the package compilation and analyze-request handoff helper and unignored evidence test.
- `fsl-analyze/src/analyze.rs`: examined the linked-engine registry test text and assertion.
- `xtask/src/refinement/layering.rs`: examined the registry snapshot test text and trace bindings.

## Checks

- Reviewed the full `origin/main...HEAD` diff at the frozen PR head; worktree remained clean.
- Confirmed `quoin --version` is `0.36.7`, `quire --version` is `0.52.0 (engine 0.62.2)`, and `spec-artifacts-process@v0.36.0` is installed.
- Read-only `quoin matrix --repo . --json` completed; the relevant changed criteria are statically tagged to the expected tests.
- Focused evidence supplied by the code lead for this head before the trailing spec-only rebase: `fsl-semantics` profile test 1/1; `fsl-analyze` state-graph evidence, hashed-bytes refusal, linked descriptors, and unregistered backend tests each 1/1 under 16 GiB with the QSL-2396 stress test explicitly excluded; output/handoff test 1/1.
- No Cargo command was started in this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The containment check uses lexical `Path::starts_with`, so an output such as `existing/../handoff/pkg.json` does not start with `handoff_directory` even when `existing` exists and the path resolves inside the handoff directory. The handoff writer creates that directory before the output write, allowing the output file to be written there despite FR-810-AC-5's required refusal. Normalize/resolve the candidate against the request directory before the containment check, or reject traversal components and cover this case. | fsl-command/src/command.rs:2371 |

## Rust lane

The changed Rust follows the surrounding test and wire-decoding patterns; no additional Rust-specific findings were identified. The containment finding is the behavior defect reported above.
