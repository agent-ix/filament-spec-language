---
id: SR-202611210022442313144108505
title: "code-review of QSL-1463 monitor checked-term origin rekey"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-package/src/emit/monitor_plan.rs, fsl-analyze/tests/it/monitor_root.rs; FR-251, FSpec FR-322, FSpec FR-487"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: references
---

## Summary

Ticket: QSL-1463. Reviewed frozen draft PR #3281 at 88a324b6344c1b51129d51fa5c6e603367181492 against origin/main...HEAD (base f3aa681d7e). The diff is 8 producer lines in `fsl-package/src/emit/monitor_plan.rs::finish` and one new integration test in `fsl-analyze/tests/it/monitor_root.rs`. The Rust lane (rust-review with rust-style) ran in this file.

Producer: correct by inspection. `finish` now clones `base.checked_term_data()` and applies the existing typed `replace_term_data_node` for every `(old, new)` in `all_changes`, in the same order and with the same sequential, exact-match semantics as the graph `rekey()` and the retained-row `Rekey`. So the sidecar and graph cannot diverge, even on chained renames. `all_changes` holds the step-1 record-value renames and the step-2 entry renames, all in the wire form (graph-local index segments) that `write_index_segments` produced from `base.resolved_graph()`. The node order is unchanged and the plan root is appended last, so the old ids equal the base sidecar's wire ids. The helper covers every node-id field of all five `CheckedTermDataV2` arrays at contract-model rev 7b76c8b: equality_decisions, protocol_locations, dispatch_tables, probabilistic_claims, and body_locations (application and origin.function). It touches no name, index, path, occurrence or ordinal. Declared helpers are not in `changes`, so their origins are kept. Rename order is safe: the reader orders body_locations by (application, occurrence, term_ordinal), which a function rename never changes, and application nodes are never renamed. No duplicate helper was introduced: the PR reuses `emit.rs::replace_term_data_node`, which `rekey_timed_model_nodes` already uses.

Test: the test is authentic. The same `emit_checked(combined)` base as `monitor_root.rs:233` is compared row by row with the strict-reader-admitted plan. It checks the four entry ids, the helper's unchanged location (name, index, path), occurrence and term_ordinal, and a stale-origin mutation refused at `/checked_term_data/body_locations/{row}/location/origin/function`. That is the pointer the reader's `body_locations::validate` emits for an origin function that is not a local function node. However, the refusal-code assertion names a variant that does not exist, so the test, and the whole `it` test binary, cannot compile.

## Verdict

**FAIL**: one high finding (the regression does not compile) and one low finding. Production source is mergeable by inspection. The PR is not mergeable until FND-001 is fixed and the delegated focused test is green.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The stale-origin assertion compares `refusal.code` with `CheckedPackageRefusalCode::InvalidValue`. That variant does not exist at the pinned contract-model rev 7b76c8b: `InvalidValue` is a `CheckedPackageRefusalCause`, and `body_locations::validate` refuses through `invalid()` = code `InvalidPackage` + cause `Some(InvalidValue)`. The test, and the whole fsl-analyze `it` test binary, therefore fails to compile (E0599), and the regression can never run. Assert `refusal.code == CheckedPackageRefusalCode::InvalidPackage` and `refusal.cause == Some(CheckedPackageRefusalCause::InvalidValue)`, with `"{refusal:?}"` context as in `fsl-package/src/emit/tests/protocol_controls.rs:240` `assert_refused_at`. | fsl-analyze/tests/it/monitor_root.rs:180 |
| FND-002 | low | The row-by-row comparison checks `location`, `occurrence` and `term_ordinal` but never compares `after.application` with `before.application`. A writer regression that re-points a body row's `application` (the other node-id field `replace_term_data_node` rewrites) to a different application node that the reader still accepts would pass the test. Add `assert_eq!(after.application, before.application)`. | fsl-analyze/tests/it/monitor_root.rs:160 |

## Coverage

Rust lane: rust-style core rules plus the testing reference were applied. API: no public surface changed. Implementation: no placeholder, TODO or no-op. Safety: no new panics on caller input in production; the clone is bounded by the existing sidecar. Boundaries: typed writer input, no narrowing conversions. Tokio: not used.

Gates run by this reviewer: `rustfmt --check --edition 2021` on both changed files (rustfmt 1.9.0), exit 0. `quire matrix --scope <worktree> --format json` (quire 0.52.0, engine 0.62.2), exit 0: FR-251-AC-9 is `tagged` and lists the new test at monitor_root.rs:91 as a binder.

Not run, per the brief (no local cargo): `cargo clippy`, `cargo test`, `cargo deny`. The delegated focused run `cargo test --locked -p fsl-analyze --test it monitor_root::monitor_body_locations_follow_entry_rekeys_and_preserve_helper_origins -- --exact` is PENDING with draco:QSL:code. This review predicts it fails to compile because of FND-001. No runtime pass is claimed.

Diff contains no CI or workflow, Cargo.toml, lockfile, budget, limit or pin change. No vendoring, copied fixture or ceremony was found. Out of scope and not counted: IR-1321 (quadratic body/protocol location validation) and existing repository-wide matrix diagnostics.

## Dispositions

Disposition round 1 was checked against fixed head 8297b136461fa2636fa84d57923721a4bbc4b8c9, fix commit 8297b13646. That commit touches only `fsl-analyze/tests/it/monitor_root.rs`; the producer is unchanged since 88a324b6. `rustfmt --check` on both changed files exited 0. No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | monitor_root.rs:180-189 now asserts code `CheckedPackageRefusalCode::InvalidPackage` and cause `Some(CheckedPackageRefusalCause::InvalidValue)`, each with `"{refusal:?}"`. Both names exist at contract-model 7b76c8b (shared.rs and the lib.rs re-export). The path assertion also prints `{refusal:?}`. Compilation and runtime are PENDING the delegated exact test. |
| FND-002 | fixed | monitor_root.rs:162 asserts `after.application == before.application` for every zipped body row. |

Disposition round 2 was checked against fixed head 1a10181328e188e8d2abb3dd4669a2fd3e33e63a (commit 1a10181328, on top of 0a036e69f9). It has no rows, because no finding is still open; every finding stays `fixed` from round 1, and no new findings were found.

The round-2 commit changes only the new test, `fsl-analyze/tests/it/monitor_root.rs`:

- It replaces `root_of(..)` with a direct `emit_plan_root(&PlanRootRequest { .. })` call. Domain, packages, source, name, path, clause, target, settings and Cancel are identical to `root_of`, so the full genuine emission and strict read-back path is unchanged.
- The only difference is `limits: SpineLimits { imports: default.imports.with_work(LIMITS.work /* 1e9 */), ..default }`. `imports` is the I2 bound that `fsl-analyze/src/monitor_root.rs:276` passes to `emit_monitor_plan_over` for the base read-back, which is where IR-1321 exhausted 1e6 at `/checked_term_data/body_locations/464/occurrence`.
- `with_work` (`fsl-package/src/checked_v2.rs:315`) sets only `work`. The bound is caller-scoped to this fixture. `SpineLimits::default`, `root_of` and all production code are unchanged.
- The required comment names IR-1321, the quadratic rescan, and the removal of the explicit bound once the fixed reader is pinned.
- Every assertion is kept: all four entry ids, the helper, location, occurrence, term_ordinal, application, and the stale-origin code, cause and path.
- `rustfmt --check` exited 0. The prior SRs under `reviews/` are byte-identical to disposition1 (`cmp` 0).
- Runtime: the delegated exact test at 1a101813 is PENDING. No pass is claimed.
