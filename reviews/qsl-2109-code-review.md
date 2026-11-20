---
id: SR-1525
title: "QSL-2109 restored seam code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2109; Makefile; fsl-command/src/checked_handoff/selection.rs; fsl-package/src/emit/tests/admission_corpus.rs; tests/it/checked_handoff.rs; xtask/src/ci_feature_lane_tests.rs; xtask/src/seam_probe.rs"
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
  - {id: Makefile, path: Makefile, role: examined, excerpt: "$(MAKE) seam-probe-capability"}
  - {id: check_clause, path: fsl-command/src/checked_handoff/selection.rs, role: examined, excerpt: "name.declaration == node.node_id && name.name.as_ref() == row.name.as_str()"}
  - {id: operation_rows_read_back, path: fsl-package/src/emit/tests/admission_corpus.rs, role: examined, excerpt: 'assert!(admitted.is_ok(), "the rows read back: {admitted:?}");'}
  - {id: a_differing_declared_clause_name_is_refused_on_clause_name, path: tests/it/checked_handoff.rs, role: examined, excerpt: 'name["name"] = serde_json::json!("Omega");'}
  - {id: seam_probe_executes_only_the_capability_applicability_test, path: xtask/src/ci_feature_lane_tests.rs, role: examined, excerpt: 'let calls = recipe.run(&["seam-probe"], Some("seam target"), &[]);'}
  - {id: a_capability_variant_with_no_applicable_family_entry_does_not_compile, path: xtask/src/seam_probe.rs, role: examined, excerpt: 'build_and_collect_e0004(workspace_root, "fsl-semantics", "--cfg seam_probe")'}
bindings:
  - {test_id: a_differing_declared_clause_name_is_refused_on_clause_name, ac_id: FR-518-AC-11, trace: correct}
  - {test_id: a_capability_variant_with_no_applicable_family_entry_does_not_compile, ac_id: FR-507-AC-6, trace: correct}
  - {test_id: seam_probe_executes_only_the_capability_applicability_test, ac_id: FR-507-AC-6, trace: correct}
findings_data: []
---

## Summary

Reviewed the six-file frozen diff for QSL-2109, including Rust behavior, test assertions, Make routing, trace namespace, duplication, and gate changes. No defects found in this scope.

## Verdict

PASS for this diff. The clause check joins the current name table by declaration node and name, rather than relying on the retired node declaration representation. It does not assume globally unique names. The test separately establishes original selection acceptance, strict reader admission after mutation, and a typed ClauseName refusal.

The compiler test executes a successful normal build and a failing probe build, requiring E0004 at Capability::applies_to_declaration. A generic syntax or dependency failure cannot satisfy that diagnostic assertion. The Make change selects the actual ignored test; its process-seam routing test complements the supplied real compiler execution receipt.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Applied code-review with the rust-review lane and rust-style implementation/testing rules, with repository conventions taking precedence. Reviewed all six changed paths; relevant unchanged reader helper, compiler diagnostic collector, capability match, and selection callers provided context. The operation test now runs normally and uses the repository-required FSpec trace prefix.

Accepted the author's in-lane receipts: operation read-back, clause-name behavior, real make seam-probe-capability compiler execution, and Make routing each passed one test with zero failures or ignores and exit 0. No tests, builds, lints, gates, matrix, or upstream audits were repeated. The final operation edit changes trace metadata only. The final aggregate gate remains the coder's responsibility after review.

Population-field coverage, Done upstream work, and the ignored IR-1321 protocol performance case were excluded. The auxiliary ignored-reason scanner is unclaimed. This review does not declare all QSL-2109 work complete. No assurance-profile discovery or whole-repository audit was performed under the dispatch's explicit scope.

## Publication

Pre-PR review at the branch and SHA above. Linear posting is deferred until a real PR number is supplied because the reviewer marker contract requires pr=<repo>#<n>; no placeholder PR number was invented. Exact inherited Codex model variant is unavailable and recorded as unknown.
