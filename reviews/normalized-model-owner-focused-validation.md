# Retained normalization-owner API: focused evidence

This PR isolates the five owner/provenance files introduced by `639a3945`
onto main `ad79956e5`. It changes `fsl-semantics` and `fsl-package`.
The accepted implementation evidence below is reused from the reviewed run
branch; these tests were not repeated for this extraction. The surrounding
run/reference consumers stay in their own PRs.

| Accepted test | Result | Existing receipt |
| --- | --- | --- |
| `cli_contract::run::canonical_run_package_reference_roundtrip_uses_retained_snapshot_owners` | PASS, command exit 0; genuine package calls retain same-admission model owners after source deletion | [package receipt](https://github.com/agent-ix/filament-spec-language/blob/cc2e8b67e7af38e9b091dbfb8063bfb51579b0e5/.agent/logs/canonical-run-package-reference.log) |
| `cli_contract::run::canonical_run_population_materialization_and_reference_roundtrip` | PASS, command exit 0; authentic source references and population expansion | [source receipt](https://github.com/agent-ix/filament-spec-language/blob/cc2e8b67e7af38e9b091dbfb8063bfb51579b0e5/.agent/logs/canonical-run-population-2.log) |
| `spine::run_value::accounting_tests::canonical_run_catalog_preserves_normalization_work_denial` | PASS; actual retained normalization owner yields its typed work denial | [catalog receipt](https://github.com/agent-ix/filament-spec-language/blob/cc2e8b67e7af38e9b091dbfb8063bfb51579b0e5/.agent/logs/canonical-run-accounting-3.log) |
| `cli_contract::run::review_fix_deep_completed_values_use_actual_depth_policy` | PASS, command exit 0; genuine reference-bearing depth-100 result charges the source owner | [deep-result receipt](https://github.com/agent-ix/filament-spec-language/blob/cc2e8b67e7af38e9b091dbfb8063bfb51579b0e5/.agent/logs/review-fix-round1-command.log) |

The catalog receipt's aggregate command exited 101 because its separately
named cumulative-conversion fixture failed; the catalog test itself passed.
That unrelated fixture's correction and later pass are recorded in the run
PR's original focused receipts. No aggregate success is claimed here.
These accepted tests cover the owning implementation with its real consumers;
they do not constitute a final current-head integration check for this PR.
