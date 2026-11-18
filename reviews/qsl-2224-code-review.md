---
id: SR-2026111612172400000035
title: "Code review — QSL-2224 protocol analyze routing"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/src/protocol_analysis/conformance.rs, fsl-analyze/tests/it/analyze_cli.rs; FR-636-AC-1..4, FR-896-AC-1..4"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-636", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-896", type: references}
---

## Summary

Reviewed the Rust changes at frozen head `d8e9afda87403f0549701af61b062f436a3e7c0f`, including the CLI tests and protocol deadlock/liveness acceptance criteria. The new protocol request routing works on the tested cases, but the protocol `max_messages` limit still produces a refuted result where the criterion requires an incomplete result.

## Verdict

**CONDITIONAL** — the focused checks passed, but the protocol message-budget behavior is incorrect on the FR-636-AC-4 case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Protocol `max_messages: 1` still settles the parallel-send case as `refuted` instead of `incomplete` | fsl-analyze/src/protocol_analysis/conformance.rs:764 |

## Review scope

- Examined all changed source and test files in the diff.
- Examined FR-636-AC-1 through FR-636-AC-4 and FR-896-AC-1 through FR-896-AC-4.
- Rust API, error, lifecycle, wire-boundary, test quality, duplication, and changed-code-to-spec checks applied. No applicable AssuranceProfile was found for this scope.
- The enabled deadlock, liveness, missing-protocol, and finite-trace focused tests passed. The ignored FR-636-AC-4 test was run explicitly and failed: with `max_messages: 1`, it returned `refuted`, not `incomplete`.

## Dispositions

Not assessed; this is the initial review pass.
