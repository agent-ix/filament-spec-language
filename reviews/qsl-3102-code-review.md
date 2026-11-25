---
id: SR-967175026302197768900474181941935084846
title: "Code review — QSL-3102 EN-5 guards"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/exact_probabilistic.rs, fsl-analyze/tests/it/statistical_manifest.rs, fsl-analyze/tests/it/weighted_long_run_certificate.rs, fsl-replay/src/statistical/settle.rs; FR-195-AC-5, FR-199-AC-4, FR-199-AC-5"
review_set: subset
relationships:
  - {target: ix://agent-ix/filament-spec-language/FR-195, type: references}
  - {target: ix://agent-ix/filament-spec-language/FR-199, type: references}
---

## Summary

Reviewed the four changed Rust test/source files and their owning acceptance criteria. The new settlement and manifest assertions are trace-bound; the weighted long-run test weakens the claim threshold below the requirement's stated scenario.

## Verdict

**CONDITIONAL** — one medium test-intent finding remains; no production-code defect was identified by static review. Cargo gates were not run because the shared linker guard is active, and the lead owns final controls.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Zero-weight certificate test lowers FR-199-AC-4's claim threshold from the specified 0.999 scenario to 1/2 | fsl-analyze/tests/it/weighted_long_run_certificate.rs:706 |

## Coverage

- Rust lane: API, implementation, safety, boundary, and deterministic test review of changed Rust files; no production implementation changed.
- Diff/CI: no workflow files changed; no gate was weakened.
- Gates: `cargo fmt --check`, clippy, touched-crate tests, and `cargo deny check` were not run because the requested linker guard remains active. The parent owns final controls; no Cargo command was started.

## New findings (disposition pass 1)

Reviewed head a36a2293324f9f06ca46c92f9c31c696868a65c6 (diff `git diff origin/main...HEAD`, merge-base 1df75ab47f). FND-002 records the finding posted as "FND-001" in Linear comment 32a70dc5 under this same SR id, renumbered here so each finding has one unambiguous id.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | (Comment 32a70dc5 FND-001.) New EN-5 manifest test en5_advertises_only_exact_evidence_for_unbounded_probabilistic_satisfaction duplicated exact_probability_manifest_and_statistical_refusal_settlement_are_exact | fsl-analyze/tests/it/statistical_manifest.rs:103 |
| FND-003 | medium | New settle test exact_only_statistical_refusals_settle_with_their_named_causes duplicates each_refusal_row_settles_as_the_map_states (same file) and exact_probability_manifest_and_statistical_refusal_settlement_are_exact; its only extra assertion, category() == Unsupported, follows from the matched TerminalValue::Unsupported. Fix: delete the test | fsl-replay/src/statistical/settle.rs:357 |
| FND-004 | medium | FR-199-AC-4 requires the certificate to pass tools/fsl-probability-check, but the only tests tagged FR-199-AC-4 call the fsl_replay library function check_probability_certificate_document inside the producer's test process; no fsl-probability-check test runs a zero-weight weighted long-run document. Pre-existing on origin/main, restated by this PR. Fix: add a tools/fsl-probability-check/tests/it/roundtrip.rs case through proof_source_request for the zero-duration Avail claim at >= 999/1000 and tag it FR-199-AC-4 | fsl-analyze/tests/it/weighted_long_run_certificate.rs:700 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 3eb7337f70: zero_claim uses `>= rational(999,1000)` (weighted_long_run_certificate.rs:707) |
| FND-002 | fixed | Commit 3eb7337f70 deletes the test; statistical_manifest.rs is unchanged against origin/main at head |
| FND-003 | still-open | New in this pass; no fix at head |
| FND-004 | still-open | New in this pass; no fix at head |
