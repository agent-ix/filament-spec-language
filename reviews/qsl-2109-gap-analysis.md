---
id: SR-1526
title: "QSL-2109 confined restored seam gap review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2109; Makefile; fsl-command/src/checked_handoff/selection.rs; fsl-package/src/emit/tests/admission_corpus.rs; tests/it/checked_handoff.rs; xtask/src/ci_feature_lane_tests.rs; xtask/src/seam_probe.rs; FR-518-AC-11; FR-507-AC-6"
review_set: subset
pr: null
branch: codex/qsl-2109-reenabled-ir-seam-tests
head: 38edb9318c468841a58285a62f9f6acfd2291b1a
model: unknown
quoin: 0.29.4
module: spec-artifacts-process@0.2.0
clean: true
reviewed_units:
  - id: FR-518-AC-11
    path: spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md
    role: examined
    excerpt: "A package whose `temporal_clause_names` row for a clause holds the name `Omega` where the sidecar declares `Alpha` is admitted by `read_checked_package` and refused by `Selection::check` with `clause.name`; the same package with the row's name `Alpha` is admitted by both."
  - id: FR-507-AC-6
    path: spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md
    role: examined
    excerpt: "A `Capability` variant with no applicable-family entry does not compile under `--cfg seam_probe`."
bindings:
  - {test_id: a_differing_declared_clause_name_is_refused_on_clause_name, ac_id: FR-518-AC-11, trace: correct}
  - {test_id: a_capability_variant_with_no_applicable_family_entry_does_not_compile, ac_id: FR-507-AC-6, trace: correct}
  - {test_id: seam_probe_executes_only_the_capability_applicability_test, ac_id: FR-507-AC-6, trace: correct}
findings_data: []
---

## Summary

Confined gap review of the six-file QSL-2109 diff and its restored operation read-back, temporal-name selection, and capability compiler seam. No gaps found within those obligations.

## Verdict

PASS for the confined diff review only. This is not an engine-backed repository-wide gap-analysis PASS: the dispatch explicitly excludes matrix computation and whole-repository auditing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed. No plan was supplied. Matrix verification and repository-wide reverse-gap inventory were not performed by explicit dispatch constraint. The gap-analysis skill was available; this is a deliberately bounded application, not a claim that the engine or skill was unavailable.

FR-518-AC-11 is exercised through a real emitted handoff, real strict reader, and real Selection::check. The positive control catches an always-refuse implementation; strict admission prevents a reader rejection from masquerading as selector enforcement; the typed axis assertion catches refusal on the wrong dimension. Node and name are matched together, permitting authored names sharing a canonical node without assuming global name uniqueness.

FR-507-AC-6 is bound to the real compiler test now selected by make seam-probe. Its positive build and E0004 function-specific negative assertion establish the capability applicability obligation, beyond command recording. The newly added routing test catches removal of the exact ignored-test invocation. The supplied real compiler execution and other focused passing receipts were accepted without rerun.

The restored operation test exercises emission and strict read-back and is active in normal runs; the changed trace prefix names the external FSpec obligation correctly. No changed path adds a stub, replaces its behavioral subject with a double, vendors source, or narrows a gate. Population-field coverage, Done upstream work, IR-1321, and the auxiliary ignored-reason scanner were excluded; no whole-ticket completion claim is made.

## Publication

Pre-PR branch and SHA are recorded above. Posting awaits a real PR number for the reviewer's required marker format. Exact inherited Codex model variant is unknown.
