---
id: SR-2026112005262801
title: "code-review of PR 3145 (QSL-2664, QSL-2559, QSL-2596)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3145 head 86d922cf71c1a7fb476fd269c963e5bb3ef9943b; fsl-analyze/src/{analyze.rs,analyze/request.rs,analyze/hybrid_item.rs,provider.rs}, fsl-package/src/package_admission.rs, fsl-eval/src/simulation/model/terminal.rs, fsl-semantics/src/check/check/typing.rs, tests"
review_set: subset
---

## Summary

Ticket: QSL-2664 (with QSL-2559, QSL-2596). Code review with the Rust lane, read-only over `git diff origin/main...HEAD`; no builds (the coder's focused runs were reported green).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | provider::run reports every package refusal as cause `invalid-value` and ProofRefusalCause::InvalidInput, whatever the reader's cause (none, missing-member, conflicting-definition) or code (missing_import, resource_exhausted) | fsl-analyze/src/provider.rs:88-91 |
| FND-002 | medium | The up-front strict read adds one full emit + strict read to every analyze run; its result is not reused, so checked_v2_package (per protocol item), admitted_bytes (per backend group) and wire_graph (per hybrid item) each emit and read again | fsl-analyze/src/analyze/request.rs:410-419 |
| FND-003 | low | AnalyzeRefusal::Package bypasses the clause route's package_refusal: no FR-940 ambiguous-name to ambiguous_declaration remap, and of_import_view never fills identity for missing_import | fsl-analyze/src/analyze.rs:822-841 |
| FND-004 | medium | terminal_class reads every package terminal; FR-124 reads only the declarations of the subject's model identity, and the two-identity test now asserts a test/vault state is intended by test/vault-other's `any` | fsl-eval/src/simulation/model/terminal.rs:166 |
| FND-005 | low | `mod refused_package;` is out of alphabetical order; rustfmt (reorder_modules) moves it, new fmt drift | fsl-package/src/emit/tests.rs:58 |
| FND-006 | low | Replay paths still map any package emit/read failure to fault("package-emission") (runtime_invariant); reachable only for a package the strict reader admits but ReplayPackage::read rejects (different byte limit) | fsl-analyze/src/analyze/clause_item.rs:451 |

## Verdict

No high findings; mergeable with the mediums tracked. QSL-2596: `ExprNode::Reaches` has exactly two expression children (source, target; the edge is a name, fsl-forms/src/syntax.rs:925), so `location.child(1)` is the target for every form; ExpressionLocation is a child-index path used for refusals and outcome loci, not node ids, so only diagnostics inside a reaches target move (to the right place). QSL-2559 classifier matches FR-124 :153-158 line by line (intended > undecided > undefined > deadlocked; first refused/undefined in row order is named). QSL-2664 maps reader refusals to the typed refusal before any item runs on every analyze_bound caller (analyze, analyze document settle, provider::run); the package route was already admitted in fsl-command. No Python, no new ignores, no pin changes; clean merge-tree against origin/main and against PR 3142.

## Scope and evidence

- QSL-2664 (fsl-analyze/src/analyze/request.rs): analyze_bound reads the compiled package through the strict reader before any item runs and returns AnalyzeRefusal::Package with the reader's code, cause and pointer.
- FR-124 (spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md): The classifier SHALL classify the terminal state as intended when at least one declaration intends it; otherwise as undecided when at least one is refused; otherwise as neither intended nor deadlocked when at least one is Undefined; otherwise, and with no declaration, as deadlocked. The class SHALL NOT depend on the order in which the declarations are read or on the aliases.
- FR-104-AC-9 (spec/functional/FR-104-check-state-clauses.md): reaches(self, target, parent) on a ConfigVersion subtype with its inherited parent: Option<Reference<ConfigVersion>> field checks.

## Recommendations

- FND-001: keep the reader's typed cause in PackageAdmissionRefusal (the contract-model cause enum) and give it a `&'static str` spelling, or match the reader's closed cause set to static strings; use the catalog's default cause when there is none, and map resource_exhausted to its own cause rather than InvalidInput.
- FND-002: read once in analyze_bound and pass the AdmittedPackageRead (or its admitted CheckedPackageV2 and EmittedPackage) to work, settle_items and wire_graph.
- FND-003: move the FR-940 remap and the missing_import identity lookup into fsl-package so every route shares them, or route AnalyzeRefusal::Package through fsl-command's package_refusal.
- FND-004: filter self.terminals() to the subject's selection identity (or confirm the joint two-identity system is outside FR-124 and drop the cross-identity cases from the test).
- FND-005: move the line between refinement_rows and retained_rows.
- FND-006: no change needed now; note it if replay limits ever diverge from V2ReadLimits.
