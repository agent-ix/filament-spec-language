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

## Supplemental Dispositions — Round 1

PASS, no findings. This confined supplement covers only the delta from 38edb9318c468841a58285a62f9f6acfd2291b1a to 128108371d85b0b500aa170f6bae366c8c3b2c8c for public PR filament-spec-language#3189. The original pre-PR metadata and finding placeholder remain historical; no original finding was rewritten. This supplement supersedes the earlier exclusion of the now-added ignored-reason guard.

Examined xtask/src/ignored_reason_tests.rs, its test-only registration in xtask/src/lib.rs, and the reason/oracle changes in fsl-package/src/emit/tests/protocol_graph.rs. The syn visitor recurses through nested cfg_attr attributes, rejects missing or unowned ignore values, and accepts a syntactically valid IR/QSL/STD ticket or a make target present in CI_CHECKS with a nonempty recipe. Both fixtures exercise the actual checker. The repository guard scans the intended package, seam, and handoff test sources through existing source discovery/parsing helpers and is registered in the ordinary xtask library suite. No duplicate parser or source vendoring is introduced.

The FIFO-key cases now assert current positive behavior using lower_with/lower_package, including successful strict IR admission and the existing independent Read::Verified result. Their QSL-1142 ignores explicitly retain pending checker/IR parity; they are not counted as executed evidence. The redundant obsolete quantity-refusal test and its unused refusal helper were removed; unstable-key negative coverage is preserved. Relevant examined contracts are FR-881-AC-2 (S3 admits the channel exactly when the FSpec FR-726 reader admits it) and FR-880-AC-10 (the listed bounded, quantity, composite, and model-owned keys compile and every package is admitted).

Accepted author receipts without rerun: two checker fixtures passed; the repository guard first exposed exactly four orphaned reasons, then passed after the reason/oracle corrections. No ignored reader-gap tests were run and no coverage completion is claimed for them. No tests, builds, lints, gates, matrix, upstream audit, or repeat full review ran in this supplemental pass. Plan completion: not assessed. The final aggregate gate remains pending with the coder. No existing defect dispositions were needed because the original review had no findings.

## Supplemental Dispositions — Round 2 Final

PASS for the confined final delta, with QSL-2109 acceptance still blocked by QSL-2729. Reviewed only changes since 128108371d85b0b500aa170f6bae366c8c3b2c8c at final head 07416e6b76451fc9415b7c5040262ee621b51d96 (source head e14179fc42296e240d77cf0fc020c765e760bf3f), public PR filament-spec-language#3189. No new review findings; the original no-finding placeholder remains historical.

Examined fsl-package/src/emit/tests/admission_corpus.rs, tests/it/checked_handoff.rs, xtask/src/ci_feature_lane_tests.rs, and the author gate receipt reviews/qsl-2109-final-gate.md. The classifier labels three currently unsupported IR kinds as not emitted with reasons, rather than claiming them as admitted evidence. Population-row behavior now has an active positive test requiring the emitted field mapping, no omitted refinement, and strict reader admission. The separate sequence regression keeps that same strict positive oracle, remains ignored under real owner QSL-2729, and carries no trace tag; its actual InvalidDerivedShape failure is neither accepted as success nor counted as passing coverage.

The handoff repairs preserve exact source-substitution code/cause/locus assertions, positive admission before sidecar name mutation, and typed ClauseName refusal. The missing-profile node locus is derived from the actual await/compensate node rather than a fixed ordinal; the wildcard table entry is followed by an exact dynamic-path assertion. FR-517-AC-18 promises emitted model-named unit/dimension closure and reader admission, so removing an unrelated absence claim leaves the required positive closure assertions intact. The CI process tests retain exact full command ordering, an absolute generated corpus path reused by the layering check, and the existing resolved-feature equivalence invariants around the current fsl-source Skip decision.

Accepted the author's original component receipts and failed-only correction receipts without rerunning them. The original component run was not all green: admission 32 PASS/2 FAIL/1 ignored, handoff 42 PASS/4 FAIL, CI lanes 7 PASS/3 FAIL; ignored-reason fixtures/guard 3 PASS and real seam compiler 1 PASS. Subsequent failed-only repairs passed except the genuine strict sequence-row defect, now owned by QSL-2729. The final ownership guard passed. Full acceptance remains blocked; this review does not replace those facts with a green aggregate-gate claim. No tests, builds, lints, gates, matrix, requirement audit, passing-segment repeat, or review sweep ran in this final supplement. Plan completion: not assessed.
