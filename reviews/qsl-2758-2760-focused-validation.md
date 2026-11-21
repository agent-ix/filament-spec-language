# QSL-2758 / QSL-2760 focused validation

Reviewed source head: `cc2e8b67e7af38e9b091dbfb8063bfb51579b0e5`.
SR-1547 and SR-1548 pass after the same reviewer's first-round dispositions;
SR-1549 and SR-1550 remain their original passing specification reviews.

The 27 original accepted named tests comprise:

| Evidence | Named passes | Receipt under `.agent/logs/` |
| --- | ---: | --- |
| Canonical exact/normalized values, strict legacy refusal, independent item identity/order | 3 | `canonical-run-values-integrated-2.log` |
| Catalog normalization-owner denial | 1 | `canonical-run-accounting-3.log` (this test passed in an aggregate exit 101) |
| Cumulative supplied admission plus execution | 1 | `canonical-run-cumulative-4.log` |
| Source population expansion and genuine reference reuse | 1 | `canonical-run-population-2.log` |
| Authoritative values: 16 ready subcases | 1 | `canonical-run-authoritative.log` (values passed in an aggregate exit 101) |
| Explicit backend plan: success 0, refutation 10, blocked 22, null-as-absent 0, unknown backend 20 | 1 | `canonical-run-plans-selected-2.log` |
| Affected source/package/budget/law/handoff/provider migrations | 17 | `migration-*.log`, excluding the separately owned reused-claim failure |
| Genuine package canonical reference roundtrip | 1 | `canonical-run-package-reference.log` |
| Value-only call at zero model work | 1 | `canonical-run-zero-model-work.log` |
| **Total original accepted** | **27** | |

Four new review regressions pass, with both commands exiting 0 and using
`--test-threads=1`:

- `cargo test --locked -p fsl-replay --lib review_fix_ -- --test-threads=1`
  passed `spine::run_value::accounting_tests::review_fix_legacy_certificate_reference_roundtrip`,
  `spine::run_value::accounting_tests::review_fix_canonical_staging_admits_conversion_before_allocation`,
  and `spine::run_value::accounting_tests::review_fix_codec_depth_policy_has_no_implicit_ceiling`.
  Receipt: `.agent/logs/review-fix-round1-replay-2.log`.
- `cargo test --locked -p fsl-command --test it cli_contract::run::review_fix_deep_completed_values_use_actual_depth_policy -- --exact --test-threads=1`
  passed one public-command test with real value-only and reference-bearing
  results nested 100 levels deep. Receipt: `.agent/logs/review-fix-round1-command.log`.

The held-claim regression is resolved upstream by
`be3ad51571bb5e7b9c0e86588cfa32611053ed3d`; its owner's passing result is trusted,
and its unchanged test was not rerun here. The original 27 accepted tests were
not repeated during the fix round. Complete focused receipts, including actual
original failures, remain in `.agent/qsl-2758-2760-focused-receipts.md`.
No final integration gate is claimed by these focused results.
