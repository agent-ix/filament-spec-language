---
id: SR-2886002
title: "Gap analysis of FSL PR #3587: FR-338-AC-31 and FR-128 bindings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@b9628ab7aecc2d490eb1087672456988030215b0; FR-338-AC-31, FR-127-AC-1, FR-128; fsl-analyze/tests/it/protocol_bounded_lasso.rs, analyze_tp3_package.rs"
review_set: subset
---
# Gap analysis of FSL PR #3587

## Summary

Ticket: QSL-2886. The two un-ignored tests carry `#[trace("TC-525", "FR-338-AC-31")]`, and the lasso test also carries `FR-127-AC-1`. Both bindings are correct. The package test covers "from a source run and from the S4 checked package, with the same value, certification and checker verdict" for the Echo proved and refuted cases. The lasso test covers the Echo default `proved`/`Certified` result, the adversarial `Violated` with prefix `send` and loop `duplicate`, `lose`, the `Refuted` label, `WitnessFails` not at the origin, and the `max_states` one below the closure stopping `LimitReached{MaxStates}`.

## Verdict

PASS. The ticket's acceptance criterion ("the listed tests pass un-ignored") is met by the focused run. This is a scoped, manual check of the AC-to-test bindings. `quoin matrix` was not run, and no repository-wide matrix status is claimed. Plan completion: not assessed. The FR-128 `on each` later-activation rule now has executable evidence through the adversarial Echo lasso replay. Its position reporting is raised in SR-2886001 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
