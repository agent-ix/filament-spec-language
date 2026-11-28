---
id: SR-967175026302197768900474181941935084850
title: "Code review — QSL-3175 I2 native join limit tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-eval/tests/it/ieee_ordering_route_parity.rs; tests/it/bare_package.rs; FR-255-AC-17/18/19; TC-721 steps 6–9; PR #3899 at 78011a4a1cd6b487550034efc2e55eded3db81c3"
review_set: subset
---

## Summary

Ticket: QSL-3175. Reviewed the two Rust test-file changes against FR-255-AC-17/18/19 and TC-721 steps 6–9. The scoped tests cover the requested I2 identity-join boundaries and typed refusals; no defect was found.

## Verdict

**PASS** — the new assertions exercise the real package identity-row and evaluation-view paths, retain typed denial evidence, and carry the required FR/TC trace tags. No production or CI workflow changes are in this diff.

## Rust review

The `fsl-eval` test materializes identity rows from admitted package bytes, admits `(u64::MAX, u64::MAX)` and the measured exact work/storage pair, and checks the typed zero-work refusal against `NativeJoinBudget::first_unpaid()` and its locus. The root integration test measures the full evaluation-view join, admits maximum and exact bounds, verifies zero-storage refusal at a positive storage charge, and checks the one-short work/storage projection errors, setting names, and stage-limit fields. The projection error derives `Debug`, so the no-`cause` assertion also detects a newly exposed field in the current public error representation. The tests are deterministic and trace-tagged. No changed public API, production logic, dependency, wire conversion, unsafe code, or asynchronous behavior was introduced.

## Scope examined

- `fsl-eval/tests/it/ieee_ordering_route_parity.rs:149-203`
- `tests/it/bare_package.rs:1827-1946`
- `spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md` — FR-255-AC-17: Both I2 settings accept zero and `u64::MAX`; zero storage refuses a nonempty identity join at the first positive storage charge, and maximum admits it.
- `spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md` — FR-255-AC-18: Measured `(W, S)` admits; `(W-1, S)` refuses on work and `(W, S-1)` refuses on logical storage.
- `spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md` — FR-255-AC-19: Refusals carry `stage_limit_exceeded` evidence; IR projection errors use the exhausted-work/storage codes with no separate public `cause`; `NativeJoinBudget::first_unpaid()` exposes the first denied kind and amount.
- `spec/test-cases/TC-721-every-setting-raises-its-limit-at-every-entry-point.md` — steps 6–9 and expected results for these criteria.

## Verification

- `rustfmt --check --edition 2024 fsl-eval/tests/it/ieee_ordering_route_parity.rs tests/it/bare_package.rs` — exit 0.
- `git diff --check origin/main...HEAD` — exit 0.
- Coder's exact-filtered `fsl-eval` test `evaluator_identity_rows_accept_maximum_and_report_zero_work_refusal` — 1 passed, exit 0; log `/tmp/qsl-3175-fsl-eval-idrows.log`.
- Coder's exact-filtered root test `bare_package::the_view_the_constructors_and_the_writer_refuse_a_budget_one_short` — 1 passed, exit 0; log `/tmp/qsl-3175-fsl-package-i2-view-final.log`.
- Independent rerun was stopped while waiting for the locked build lane; no broader tests were run. The reported exact-filtered logs were inspected.
- The approved Quoin matrix/tooling issue remains a gate limitation. The scoped requirements-to-test check above was performed manually; matrix availability was not treated as acceptance evidence.
- Clippy and the aggregate workspace suite were not run because the requested test policy allows only name-filtered tests. No CI workflow files changed.
- No applicable AssuranceProfile was found for the changed test behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
