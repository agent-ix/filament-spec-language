---
id: SR-967175026302197768900474181941935084848
title: "Code and Rust review — QSL-3222 PR #3890"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/model_check/liveness.rs; tests/it/temporal_package_route/remaining_routes.rs; spec/test-cases/TC-1581-analyze-outcome-document-carries-verdict-certificate-counterexample-and-accounting.md; FR-126-AC-26; FR-621-AC-19; FR-621-AC-20; FR-621-AC-21; FR-621-AC-22"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-126", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-621", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/TC-1581", type: reviews }
---

## Summary

Ticket QSL-3222, PR #3890 at head `17865da498b637679928ee0a9ddd4fe434651302`. The liveness path now retains roots admitted before an initial-state limit, accounts their graph states, and marks the result incomplete. Static review found that the touched fairness test still does not bind or assert FR-621-AC-19 through AC-22, and the new one-state assertion does not verify the exact MaxStates cause required by FR-126-AC-26.

## Verdict

**CONDITIONAL** — two medium test coverage findings remain. The implementation's stop precedence and retained-root accounting match the reviewed path; terra check passed, while the exact filtered integration test is pending.

## Scope examined

- `fsl-analyze/src/model_check/liveness.rs`: initial-state iterator handling, retained product roots, state limit accounting, first-stop precedence, and mapping to the MaxStates incomplete result.
- `tests/it/temporal_package_route/remaining_routes.rs`: changed trace attribute and fairness assertions.
- `FR-126-AC-26`: lazy admission, exact state-limit result, and stop precedence.
- `FR-621-AC-19`: native analyze request and first outcome identity.
- `FR-621-AC-20`: second outcome identity.
- `FR-621-AC-21`: third outcome identity and certificate.
- `FR-621-AC-22`: outcome identity ordering and aggregate exit.
- `TC-1581`: new bounded fairness step and existing ordered-results procedure.

## Rust review

`start_states` preserves already inserted product roots on `OverBudget`, records `Halt::InitialStates`, and stops pulling the model iterator. `Run::explore` retains those roots, adds their graph state count, then selects the recorded initial-state limit as the ending. `record` preserves the first stop, so a later retention stop does not replace the limit reached while generating the initial set. No new panic, unsafe operation, wire conversion, or unrelated code was found. No Cargo command was run by this reviewer because the focused terra test is active.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The touched fairness test neither asserts outcome request indices and claim names nor carries FR-621-AC-19 through AC-22 trace tags, so swapping the first two refuted items still passes. | tests/it/temporal_package_route/remaining_routes.rs:368-406; FR-621-AC-19..22 |
| FND-002 | medium | The new one-state check asserts `incomplete` and one explored state but not that the result's cause is `max_states` with value 1, so it does not establish FR-126-AC-26's exact limit outcome. | tests/it/temporal_package_route/remaining_routes.rs:439-443; FR-126-AC-26 |

## Verification

- `quire matrix --scope . --format json` — completed; 6,726 criteria rows, with FR-126-AC-26 tagged and FR-621-AC-19 through AC-22 untagged. Existing repository diagnostics were emitted.
- Terra `cargo check --locked -p fsl-analyze --all-targets` — passed per coder report.
- Terra exact filtered integration test — still running at review time; not run by reviewer.
- Strict Clippy on the full dependency graph reports two pre-existing `fsl-package` warnings outside this diff; a local no-deps Clippy attempt also surfaced existing workspace warnings. No changes requested for these per arch instruction.
