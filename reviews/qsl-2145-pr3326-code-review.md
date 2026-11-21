---
id: SR-2026112115323300000001
title: "Code and Rust review of QSL-2145 owner routing, partial delivery (filament-spec-language PR 3326)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@1ebf1a70f4b58987bb09e928fd2682a43400a3e0; gh pr diff 3326 (27 files): fsl-eval/src/temporal/evidence/{owner.rs,owner/activation.rs,owner/completeness.rs,call.rs,package.rs,producer.rs,request.rs,ledger.rs,activation.rs,mod.rs}, fsl-eval/Cargo.toml, fsl-package/src/{bare.rs,checked_v2.rs}, fsl-eval/tests/it/*, tests/it/*, tests/support/mod.rs"
review_set: subset
date: 2026-11-21
---

## Summary

Ticket: QSL-2145 (partial delivery). The PR admits the evidence trace through
filament-observation (rev 1e356f9), reads supplied Booleans through
`partial::derive`/`read`, derives activation through `activation::derive` per
instance and for the empty optional scope, computes a completeness view, and
takes a caller-admitted producer bundle. Rust lane (rust-review) folded in.
Review is diff-only, by reading; no build or test was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Rebase damage: the root crate has no `filament-observation` dev-dependency (main's Cargo.toml was taken), but `tests/support/mod.rs` names `filament_observation::AdmittedStaticBundle` under `cfg(feature = "checked-handoff")`. The root test target with `--features checked-handoff` / `--all-features` cannot compile, so the four checked-handoff modules this PR edits (bare_package, temporal_package_route, temporal_package_route_formula, temporal_result_evidence) were never run; the author's root run (temporal_capture_run, default features) does not reach them | tests/support/mod.rs:12-18; Cargo.toml:103 |
| FND-002 | high | The behaviour-preservation evidence does not exercise the change: the owner path runs only when `canonical_package_bytes` is `Some`, and `temporal_activation::Env::subject` (used by temporal_rulings, causes, relation, identity, activation) sets it to `None`. The 86 temporal tests cited as passing therefore run the local path only; the owner path is covered by observation_owner (2 tests with `Some(b"{}")`), package_aggregate_route and checked_route | fsl-eval/tests/it/temporal_activation.rs:198-201; fsl-eval/src/temporal/evidence/call.rs:1660-1672 |
| FND-003 | medium | Owner routing is optional: `assess` skips the owner when the subject has no package bytes, so a byte-less `EvidenceSubject` and every bare package built with `BarePackage::new` (all fsl-command call sites) keep the local partial and activation rules as a second live path. The bare route also passes an empty `time_unit_scales`, so a `from_admitted_read` bare package with a FixedSample, Timed or TimestampedEvent clock would fault `checked-clock-has-time-unit-scale` | fsl-eval/src/temporal/evidence/call.rs:1660-1672; fsl-eval/src/temporal/evidence/package.rs:344-350; fsl-eval/src/temporal/evidence/owner.rs:851-862 |
| FND-004 | medium | The completeness view is computed and dropped: `owner.completeness_view(.., None)?` is called for its refusal only, with no retained assertion, and nothing compares its state with the local ledger. The owner cannot disagree with the verdict (it is never read), but the step only adds fault paths and the PR body counts it as delivered | fsl-eval/src/temporal/evidence/call.rs:1675-1684 |
| FND-005 | medium | Activation disagreement is silently preferred one way: for a locally `Active` instance the owner's non-`Active` answer replaces it, and for an empty scope the owner's disposition replaces `scope_disposition` with no comparison (an owner `Active` there yields `Outcome Active` with no evaluation). The `Unknown` kind is rebuilt from local `scope.evidence`, not the owner. QSL-2145's negative AC asks that a disagreement fail, not be preferred | fsl-eval/src/temporal/evidence/call.rs:1806-1812,1894-1924,1540-1565 |
| FND-006 | medium | Owner activation views are derived for every instance even when `request.trigger` is `None` (answers unused) or names one instance. Each costs one admission plus closure, completeness, capture and activation derivations, and an owner fault on an unrelated instance now fails the whole call | fsl-eval/src/temporal/evidence/call.rs:1813-1873 |
| FND-007 | low | Dead parameters and duplicated clock-unit matching: `clock_range` takes `epoch`, `period` and `time_factor` only to discard them (`let _ =`), and the clock-unit match is written three times | fsl-eval/src/temporal/evidence/owner.rs:1023,1059; fsl-eval/src/temporal/evidence/package.rs:153-166,630-640; fsl-eval/src/temporal/evidence/owner.rs:843-849 |
| FND-008 | low | Stale doc after the bare.rs merge: "`BarePackage::new` is the one constructor" is now false, because `from_admitted_read` is a second constructor | fsl-package/src/bare.rs:39 |

## Scope and evidence

- Rebase check, bare.rs: main's `dependency_views`/`with_dependency_artifacts`
  and the branch's `canonical_bytes`/`from_admitted_read` sit side by side, with
  no duplicate item and no dead copy. `new` initialises `canonical_bytes: None`.
  Dependency views are built with `new` and carry no bytes; nothing reads them.
- Rebase check, fsl-eval/Cargo.toml: `sha2` moved from dev-dependencies to
  dependencies (owner.rs uses it) and `syn`/`proc-macro2` were added for the
  AC-15 scan. `filament-observation` was already a workspace dependency on main.
- Undefined atoms: `valuation` returns the supplied `Undefined` unchanged and
  `derive_partial_facts` skips `Undefined`, so no fact is created (FR-944-AC-12).
  `Missing` maps to {false,true} with reason `Missing` and reads back `Missing`.
- Equal-content positions share one record. The canonical value excludes the
  FR-106 label and the side map keeps both labels, so the valuation lookup by
  label finds the shared fact.
- Empty trace: `TimestampedTrace::admit_uncertain` refuses `EmptyTrace` before
  the owner is reached, so `clock_range`'s `owner-trace-is-nonempty` fault cannot
  fire from input.
- `admit` delegates to `admit_uncertain` with no intervals, so switching
  `admit_trace` to `admit_uncertain` keeps the order when no interval is supplied.
- Trace identity now encodes interval, arrival, admitted document and retained
  capture facts. Identities are computed for each run and are not persisted
  across builds by fsl-eval, so the change is consistent.
- Half-migrated completeness: the view is never consumed, so it cannot
  contradict `revise_complete_through` at runtime (FND-004 covers the gap).
- fsl-analyze monitor_incremental failures (two tests that assert `level` is
  absent from a report's Debug): no file under fsl-analyze or fsl-replay is in
  the diff. fsl-replay uses only `TraceBinding`, `ClauseSelection`, `ClauseFacts`,
  `TemporalEvidenceLimits` and `EvidenceCounter` from `temporal::evidence`, and
  none of these changed. Its ledger is its own `EvidenceLedgerFile`, not
  fsl-eval's `RetainedInstance`. `EvidenceKey.clause` is the declaration key
  text, which is untouched. This diff cannot affect those two tests.
- No Python and no logic-bearing shell in the diff. Every new test carries
  `#[trace]`/`#[ix_trace_rs::trace]`; the accuracy of those tags is in the
  gap-analysis SR.

## Verdict

Fix round needed. FND-001 (the checked-handoff tests do not compile) and
FND-002 (the cited temporal suite never runs the owner path) block merge. If
the remainder states them precisely, FND-003 to FND-006 may stay out of this
partial. Otherwise each needs either the fix named on the ticket or an explicit
line in the PR's Not-done list.
