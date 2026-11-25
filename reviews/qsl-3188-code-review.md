---
id: SR-846
title: "Code review of FSL QSL-3188 PR #3845"
type: SpecReview
analysis: code-review
scope: "QSL-3188; FSL PR #3845 at 3f55e1e42dc8500683cb45c4408da2579fd7d015 against origin/main 1df75ab47fc1fc64d6e87df8172903e449a78766"
review_set: subset
---

## Summary

Static review of the eight changed Rust files, including the new mutual-recursion emitter tests and binder-shadowing change. The exact candidate is textually mergeable with current main, but the merged emitter walk conflicts semantically with #3835, and the new FSpec trace tags omit the repository-required namespace.

## Verdict

FAIL. #3835's function-node pruning bypasses the syntax-owner handling added by this PR, so recursive function bodies are not walked transitively and the new source-map/ordinal expectations cannot hold on current main. The new tests also use bare FSpec ids where this repository requires `FSpec-` prefixes. No cargo, tests, or build gates were run in this static review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Current-main #3835 skips function nodes before this change can enter their bodies, breaking the transitive mutual-recursion visits required by the new tests and FR-825-AC-11 source-map ordinals. | fsl-package/src/emit/value_validity_requirements/frames.rs:239-250; tests.rs:720-739 |
| FND-002 | medium | Both new tests tag FSpec criteria with bare `FR-*` strings instead of the required `FSpec-FR-*` namespace, so the intended cross-repository trace bindings are untracked. | fsl-package/src/emit/value_validity_requirements/tests.rs:643,757; AGENTS.md: Trace tag ids |
