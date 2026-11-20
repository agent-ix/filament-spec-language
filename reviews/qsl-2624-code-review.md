---
id: SR-20261120095317
title: "code review of PR3148 terminal reads and canonical aggregate printing"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3148 6f50eedd8b263f0ddf73e0926beb74454356d4f6; fsl-eval terminal.rs, fsl-command tl.rs and tl/canonical.rs, fsl-replay clause.rs, fsl-semantics check, fsl-package terminal ordering, fsl-analyze terminal_two_models.rs"
review_set: subset
---

## Summary

Ticket: QSL-2624 (with QSL-2630, QSL-2627, QSL-2628). Code review with the Rust lane, by reading only; no build was run. The FR-124 filter and the node-key ordinals are correct. Two highs block: the new Filter-source arm in the aggregate printer drops an authored collection count's body, and the unwired QSL-2627 functions are dead code that fails the clippy -D warnings gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Filter-source arm treats any Filter source as a population where; an authored collection count over filter(...) prints the filter condition and drops the count body | fsl-command/src/command/tl/canonical.rs:709-724 |
| FND-002 | high | package_label, CanonicalRows::from_projection, CheckedExpression::from_projection and lower_with_package_labels have no caller, so dead_code fails cargo clippy --all-targets -D warnings | fsl-command/src/command/tl/canonical.rs:35; fsl-command/src/command/tl/canonical.rs:1136; fsl-command/src/command/tl.rs:1236 |
| FND-003 | medium | Population count always prints where body, so count(x in P) prints where true although FR-931 says an omitted where prints none | fsl-command/src/command/tl/canonical.rs:750-753 |
| FND-004 | medium | aggregate_type_argument falls back to the structural spelling when a named type argument does not resolve through source_owner by bare name, losing the alias identity for a qualified or imported type | fsl-command/src/command/tl/canonical.rs:292-298; fsl-replay/src/spine/clause.rs:582-587 |
| FND-005 | medium | QSL-2628 temporal-clause IEEE literal scan changes profile selection with no active test, because the test is restored to main | fsl-semantics/src/check/mod.rs:3061-3120 |
| FND-006 | low | guarded_summand checks only for an Integer literal 0 else branch, not that the If or sum is integer typed as the checker's fold requires | fsl-command/src/command/tl/canonical.rs:566-578 |
| FND-007 | low | Unrelated rustfmt churn remains: check.rs import order and slot_name wrap, typing.rs Exists/All arms, state_model.rs use swap | fsl-semantics/src/check/check.rs:102; fsl-semantics/src/check/check.rs:3210; fsl-semantics/src/check/check/typing.rs:4382; fsl-semantics/src/check/state_model.rs:617 |
| FND-008 | low | Rewritten two_aliases test asserts only that a mismatch exists, not which key it names or how | fsl-analyze/tests/it/terminal_two_models.rs:560-561 |

## Verdict

Blocking: FND-001 and FND-002 must be fixed. terminal_class filters to the identities the subject's universes cover and checks every alias of each. It never consults another identity, including that identity's terminal any. The classification rule itself is unchanged. Ordinals use terminal node key order on the source route (terminal_keys, resolve_terminals) and the bare route (BarePackage). git merge-tree against origin/main is clean. tests/it/request_surface_s1_16.rs is identical to origin/main. No test is ignored or deleted. No new string-literal comparison or string match was found outside string_edge functions, by reading only (xtask string-edge not run).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | rustfmt drift on new lines: a doubled blank line in the canonical.rs tests and an over-long method chain in is_declared_type_name | fsl-command/src/command/tl/canonical.rs:1146; fsl-replay/src/spine/clause.rs:612 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | high | ModelSystem::new now builds over the covered identities for every caller, so an authored item over a claim of an uncovered identity settles proved vacuously instead of the previous requires-bound refusal: package-wide invariants find no conforming rows and the identity's operations are dropped | fsl-eval/src/simulation/model.rs:800-824; fsl-eval/src/simulation/model/constraint.rs:44-60 |
| FND-011 | medium | Under partial coverage, a snapshot that lists an uncovered identity's population is now refused wrong-role-mapping for every caller, where before it was admitted | fsl-semantics/src/model/observation/document.rs:2987-2993 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | population_filter requires Only or Sum, a population source, and the outer slot and name; a collection-filter test is present |
| FND-002 | still-open | canonical_identity at canonical.rs:962 is left over and has no caller, so dead_code still fails clippy -D warnings |
| FND-003 | fixed | a population count over a literal true body prints no where; test added |
| FND-004 | still-open | an unresolved qualified or imported name now prints its authored text (Lib::Small), which depends on the alias; FR-931 Names requires an alias-free declaration key |
| FND-005 | fixed | the temporal IEEE scan has been removed from fsl-semantics |
| FND-006 | fixed | guarded_summand requires an Integer or Int value type |
| FND-007 | fixed | the churn hunks have been reverted |
| FND-008 | fixed | the test now asserts the mismatch key and its requested and recompiled sides from node order |
| FND-009 | still-open | new in this round |
| FND-002 | fixed | canonical_identity has been deleted |
| FND-004 | still-open | unchanged in 0d431f1f |
| FND-009 | fixed | both spots are formatted |
| FND-010 | still-open | new in round 2 |
| FND-011 | still-open | new in round 2 |
