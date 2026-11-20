---
id: SR-2026112022385700000000001
title: "Code review — QSL-2277/QSL-2144/QSL-2138 PR #3226 timed emission batch"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3226 diff at a67196a6c: fsl-analyze/src/analyze/document.rs, fsl-analyze/tests/it/{analyze_family_cli,analyze_request_wire,hyper_world}.rs, fsl-command/src/{command,emit_checked_v2}.rs, fsl-package/src/{emit,checked_v2/timed_model}.rs, fsl-package/src/emit/tests/{hyper_clause,timed_clauses}.rs, fsl-replay/src/spine/clocks.rs, fsl-replay/tests/it/{hyper_per_trace_clock,main,vault_world}.rs, fsl-semantics/src/check/{lowering/timed_clause,refusal,timed_model}.rs, tests/it/{clock_derivation,clock_reach}.rs"
review_set: subset
evaluated_revision: "a67196a6c6c75900f93b1f326c4693b8ecf1a486"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
---

## Summary

Tickets: QSL-2277 (primary), QSL-2144, QSL-2138. I reviewed PR #3226 at head `a67196a6c` (merge base `487422e8f1`, one commit behind FSL `origin/main` `82e13e97f`; #3214 touches none of the PR's files) by reading `gh pr diff 3226`. I checked it against FSL FR-640-AC-12, FR-230-AC-50/67/69, FR-173-AC-3/AC-6, FSpec FR-342 "Point-bound timed clauses" and FR-342-AC-11/AC-13, and FSpec FR-322 (clock collections, presence, guard uniqueness). Context came from `git show` on `origin/main` and the PR head ref in `/Users/peter/dev/filament-spec-language`. That is a read-only use of the main checkout: the brief named `/Users/peter/dev/qsl-2631-code`, which does not exist. Contract-ir context came from `/Users/peter/dev/filament-contract-ir` at the Cargo.lock pin `af45a983`.

These parts are correct:

- QSL-2277 per-trace selection. `derived_configurations` builds the trace list and the alias list from the same `hyper.quantifiers()` iteration, and S4 removes each trace's selection by the key `(source, span, Some(quantifier.variable))`. Two traces therefore cannot swap clocks. Two traces with the same alias would collide on that key and refuse `DuplicateTemporalClockSelection` instead of sharing a clock silently. A non-hyper clause still produces one selection with `trace: None`, under the same key and with the same row. A single-model hyper clause emits the same wire as before, because main already set `trace` on each hyper row. `TemporalBindingRows::get` now skips per-trace rows; its only caller is a temporal-clause test.
- QSL-2144 refusal deletion. FR-230 "Point-bound clauses" says a clock-reading clause bound to an execution point "SHALL be written, never refused". FR-230-AC-50 says "never refusing `TimedModelNotEmitted`". FR-230's selected-constraints paragraph keeps `PointQualifiedClause` only "until their own forms (FR-230-AC-50) are written". FSpec FR-342-AC-11 and AC-13 now admit `point_anchor`. No spec case still requires the refusal, so deleting `PointQualifiedClause` and its test is correct.
- #3224's absent-`clocks` fix is intact at `fsl-command/src/command.rs:3382-3386`: `clocks.package_selections(...)` runs unconditionally.
- Boundaries hold. The diff does not touch clock-place path spelling, `clock_ref` checking or `SlotPresence` lowering.

Two things are wrong:

- Point-bound guards and resets are not merged with the point-free clauses of the same operation.
- A two-model hyper clause compiles but its `--handoff` cannot be written.

## Verdict

**FAIL — fix round needed.** One high (FND-001), three mediums (FND-002 to FND-004) and one low (FND-005). The PR has no Python and no logic-bearing shell.

## Scope and skill provenance

The code-review and rust-review lanes ran by reading the diff. The brief allows no worktree, build or test run, so `quoin gap-analysis` could not run at the PR head; the manual AC-to-test check below replaces it, in this file, which is the single artifact the brief names.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | Per-trace rows match FR-640-AC-12 and FSpec FR-322-AC-453. Point-bound guard and reset merging violates FSpec FR-342 (FND-001). The handoff round-trip of a two-model hyper clause fails (FND-002). |
| Tests | The unknown-trace reader assertion is `is_err()` only (FND-003). New and re-ignored tests are covered under "Ignored tests". |
| Formatting | `fsl-package/src/emit.rs:35-55` reorders imports in the 2024 style. The repo uses edition 2021 and has no `rustfmt.toml` (FND-005). |
| Safety/concurrency/wire | No unsafe code and no concurrency. The wire gains `point_anchor` on `clock_guard`/`clock_reset` and per-trace hyper rows, both specified. |

## Coverage (manual AC-to-test check)

| Criterion | Test | Result |
| --- | --- | --- |
| FR-640-AC-12 compile half (two models, event-position and fixed-sample) | `tests/it/clock_derivation.rs::a_hyper_clause_over_two_models_binds_one_clock_per_trace`, `fsl-replay/tests/it/hyper_per_trace_clock.rs` | covered |
| FR-640-AC-12 analyze and replay half | `analyze_family_cli.rs::a_hyper_clause_over_two_models_analyzes_and_replays_per_trace`, `::a_two_model_hyper_counterexample_replays_through_the_engine` | ignored (see below) |
| FSpec FR-322-AC-453 reader rejects an unknown trace | `hyper_clause.rs::hyper_clock_rows_are_trace_keyed_and_reader_rejects_an_unknown_trace` | FND-003 |
| FR-230-AC-50 / FSpec FR-342-AC-13, a single point-bound guard | `timed_clauses.rs::a_point_bound_clock_clause_names_its_point_anchor` (un-ignored, I2 read-back added) | covered |
| FSpec FR-342: a point-bound and a point-free clause of one operation give one guard | none | FND-001 |
| FR-194-AC-1/AC-2 over source (QSL-2138) | `clock_reach.rs::every_window_aggregate_kind_reaches_a_verdict_from_source` | ignored, QSL-2338 (FND-004) |

## Ignored tests

- `analyze_family_cli.rs:876` `a_hyper_clause_over_two_models_analyzes_and_replays_per_trace`. It was already ignored on main, so this restores main's state with a new reason. The `FormNotDecided` half is accurate: `fsl-semantics/src/check/hyper_clause.rs:1549` sets a form only for `InfiniteTrace`, and `fsl-analyze/src/model_check/hyper.rs:250` refuses `None`. The `unknown-claim` half is plausible, since `ClaimItem::new` refuses when `claim_family` or the capability does not apply, but it is unverified without a run. QSL-2277 is open, so the owner is acceptable. However, `FormNotDecided` for this clause contradicts QSL-2643's ruling 2 and FSL FR-173-AC-3/AC-6 on main: "FR-640-AC-12's `Saves` settles `proved` or `refuted` ... never `FormNotDecided`". QSL-2643 was closed as "already delivered", yet main's code is unchanged. No open ticket owns the event-position form implementation. This is a tracking note for the lead, not a PR defect.
- `analyze_family_cli.rs:920` `a_two_model_hyper_counterexample_replays_through_the_engine` is new and newly ignored, for the same reason and with the same note.
- `clock_reach.rs:132` `a_quantile_aggregate_reaches_a_verdict_through_run` was already ignored on main (as QSL-2138), so this restores main's state. The cause is verified: contract-ir `af45a983` `crates/filament-contract-model/src/checked_package/v2/temporal.rs:1919-1929` takes the owner from `graph.node(&clause_node.semantic_type)`, and contract-ir `origin/main` has the same code. The owner is wrong, see FND-004.
- `clock_reach.rs:197` `every_window_aggregate_kind_reaches_a_verdict_from_source` is new and newly ignored. It has the same cause and the same owner problem (FND-004).

## Open question: `size(<clock collection path>)` in a timed predicate

- FR-230-AC-67 requires `pre S ... { size(self.ts) >= 1 and size(self.ts[0].ys) >= 2 }` (with `ys: Sequence<Clock>[0, 2]`) to discharge the presence of reset slots `0` and `1`, "never `TimedModelNotEmitted`".
- FR-230-AC-69 requires `size(self.ts) >= 1 and size(self.ts[0].ys) >= 2 and self.ts[0].ys[1] >= 1 ms` to write slot terms `0` then `1`.
- FSpec FR-322's Clock row says a clock collection whose element holds data (`Sequence<Timer>`) has a `composite_type` node. So `size(self.ts)` is an ordinary data read.
- The same row says a clock collection whose element holds no data (a `Set<Clock>`, a `Bag<Clock>`, a `Sequence<Option<Clock>>`) "has no node, and the timed model records its kind, bounds and key domain with the clock place, so that the shape reads (`size`, `present`, an index or key being in range) are recorded data predicates of the timed model and read no clock". It continues: "A `field` application whose member resolves to a clock field or a clock collection in any other position, a value position, refuses `invalid_timed_model`/`clock-read-outside-constraint`".
- What is undefined: FR-322 "Presence" gives `present(p)` a checked-term place form: a `presence_read` application of `filament.op.option.present` whose arguments are the `clock_place` template, the owner term and the slot terms. No equivalent `size` place form exists, and no catalogued operation takes a `clock_place` template. `self.ts[0].ys` therefore has neither a node for a `field` application nor a defined shape-read term. FR-230 and FR-322 do not say how a recorded shape read is written inside a guard's predicate. This needs an FSpec FR-322 addition, for example a `size` place form parallel to `presence_read`, before AC-67 and AC-69 can be written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Guards and resets merge only when `point_anchor` also agrees, so one operation's point-bound and point-free clauses become two guards (or mixed resets); the reader then refuses the package | fsl-semantics/src/check/lowering/timed_clause.rs:90, 156-158, 210, 254; fsl-package/src/emit.rs:5824-5830 |
| FND-002 | medium | A two-model hyper clause now compiles, but its `--handoff` cannot be written: the writer emits one clause-name selection per distinct clock, which `clock_selections` refuses as `DuplicateClause` or (with equal clocks) as the misworded `PerTraceClockConfigurations` | fsl-command/src/command.rs:3105-3146; fsl-command/src/emit_checked_v2.rs:624, 672; fsl-package/src/emit.rs:238 |
| FND-003 | medium | The unknown-trace reader test asserts only `is_err()`, so it passes for any refusal of the mutated wire | fsl-package/src/emit/tests/hyper_clause.rs:783 |
| FND-004 | medium | Two tests are ignored on QSL-2338, which is Done, so no open ticket owns un-ignoring them | tests/it/clock_reach.rs:132, 197 |
| FND-005 | low | Import reorder in the 2024 style, which edition-2021 rustfmt reverts | fsl-package/src/emit.rs:35-55 |

## Findings detail

- **FND-001.** FSpec FR-342 "Point-bound timed clauses" says: "the guard or reset of an operation is point-bound when any source clause it holds is bound to that point, and then holds every clause of the operation ... a model holds at most one of them for each operation". FSpec FR-322 says "A model has at most one `clock_guard` for each operation anchor". FR-342-AC-13 requires the reader to refuse "a point-bound guard and a point-free guard of one operation in one model". FSL FR-230 says "Every guarded `pre` of the operation is then written into that one point-bound guard, and every reset of the operation into point-bound resets". The lowering instead computes `point_anchor` per source clause (`timed_clause.rs:90, 210`) and merges a guard only when `held.point_anchor == point_anchor` (`:157`). A point-free `pre` and a point-bound `pre` of one operation therefore give two `clock_guard` nodes, which the strict reader refuses. That is an internal failure on an input the spec says SHALL be written. Resets behave the same way: each `post` keeps its own anchor, and `emit.rs:5827` adds the anchor to the reset dedup key. Fix: decide the operation's anchor once, as the `pre{op}` anchor when any of its guard clauses is point-bound (and the `post{op}` anchor when any of its reset clauses is), give it to every guard and reset of that operation, merge guards on `operation` alone as main does, and add a test with a mixed point-free and point-bound `pre` that emits one guard and reads it back.
- **FND-002.** `handoff_clock_selections` dedups `(clause, clock)` pairs. For `Saves` over `V` and `N` under event-position the two traces have different authorities, so the handoff carries two `"Saves"` entries. `equality_file` passes them to `emit_checked_v2::clock_selections`, which returns `Error::DuplicateClause` at `:624`, a request-shaped error for a file the producer wrote itself. When the clocks are equal (fixed-sample) the handoff carries one entry, and `:672` refuses it `PerTraceClockConfigurations`. That variant's message was reworded at `emit.rs:238` to "a model trace's event sequence authority is ambiguous", which no longer describes this path. The PR claims "On read-back it becomes one row per trace again", which holds only for a single-model hyper clause. Fix: do not write model-derived hyper selections into the handoff. Re-derive them on read with `RequestClocks::model_selections`, as compile does, and drop the `aliases.len() > 1` refusal. Add a two-model `--handoff` test.
- **FND-003.** The test mutates `temporal_bindings[0].trace` to `"unknown"`, re-serializes the wire and asserts `refusal.is_err()`. Any refusal satisfies that, including one unrelated to the trace join. Fix: assert the refusal's code and its pointer `/temporal_bindings/0/trace`.
- **FND-004.** QSL-2338 is Done, and its own done condition ("the QSL-2138 p95 run test passes") is unmet. The cause in the reason is real and still on contract-ir main (`temporal.rs:1919-1929`). An ignore that names a closed ticket has no owner, so neither test will be un-ignored. Fix: reopen QSL-2338 (or name the open IR ticket that owns the `source_owner_for_clause` fix) and keep the reason text.
- **FND-005.** Main has `use fsl_foundation::source::node_table::{scoped, DebugTable, DebugWith, With};` followed by `use fsl_foundation::Code;`, and `native_diagnostics_catalog` first in its list. That is the edition-2021 style. The PR moves `Code` first, `scoped` last and `native_diagnostics_catalog` last, which is the 2024 style, so `cargo fmt --check` would flag it. The last commit says "drop import churn", but these hunks remain. Fix: restore main's two import blocks.

## Findings data

```yaml
clean: false
findings: [FND-001, FND-002, FND-003, FND-004, FND-005]
```

## New findings (disposition pass 1)

Head `d8ca6f8d0d`, rebased on FSL `origin/main` `82e13e97f`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The aliases refusal was removed from `clock_selections`, so an explicit clause-name selection for a hyper clause over two models now gives every trace one request clock. FR-517 says such a clause "is not given one request clock for every trace", and the compile path derives each trace's own authority | fsl-command/src/emit_checked_v2.rs:662-676; fsl-command/src/checked_handoff/selection.rs:474-498; tests/it/checked_handoff.rs:3147 |

- **FND-006.**
  - **What changed.** The fix for FND-002 deleted the `aliases.len() > 1` check. It also rewrote the only test of that refusal, the TC-3128 request-path case with `"Saves": event_position "events-authority"`, into a round trip with no selection. A user `--clocks`/`temporal_selections` entry naming `Saves` over `S` and `N` now emits two rows with the single request authority. `Selection` copies it to both traces (`named_binding`).
  - **Why it is wrong.** `fsl compile` derives the two models' own `ConfigVersion` identity texts for the same unit (FR-640-AC-12). The request path and the compile path now emit different clocks for the same clause. FR-517 still says such a request is refused.
  - **Fix.** Keep the refusal for an explicit clause-name selection of a hyper clause whose quantifiers name more than one model. A handoff written by compile never trips it, because compile no longer writes derived selections. Restore the request-path refusal test next to the new round-trip test. Update FR-517's "until per-trace clock derivation lands (QSL-2277)" sentence to the landed behaviour.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `timed_clause.rs:89-112` decides each operation's guard anchor and reset anchor once, before writing. Guards merge on `operation` alone (`:189`). Tests `a_point_free_and_a_point_bound_pre_make_one_guard` and `a_point_free_and_a_point_bound_reset_share_one_anchor` (`timed_clauses.rs:1574, 1605`) both read back through I2. The point-free clause now carrying the anchor is what FSpec FR-342 requires ("holds every clause of the operation"). Its constraints and state clause are unchanged, because an operation has exactly one `pre` and one `post` point. The pre-pass's extra `clause_anchor` calls are content-addressed and add no node or occurrence. |
| FND-002 | fixed | `handoff_clock_selections` skips model-derived per-trace selections. `clock_selections` re-derives unnamed hyper clauses with `RequestClocks::model_selections`. The sidecar writes each trace's derived clock, and on read a clause-name selection is produced only when all traces agree. That selection is applied to every trace, which reproduces the compiled clock. The selection is skipped only when it equals the derived clock, so no explicit selection is lost. The two-model `--handoff` round-trip test is at `checked_handoff.rs:3147`. The removed aliases refusal is FND-006. |
| FND-003 | fixed | The test asserts `InvalidPackage` and the pointer `/temporal_bindings/0/trace`. |
| FND-004 | fixed | QSL-2338 is reopened (In Progress), and both ignores name its cause. The analyze-half ignores now cite QSL-2643 (Specced) with the `FormNotDecided` cause. |
| FND-005 | fixed | Main's import order is restored in `emit.rs:35-55`. |
