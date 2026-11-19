---
id: SR-2026111612172400000048
title: "Code and Rust review of QSL-1979 export request surface, first slice (filament-spec-language PR 2953)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@3a3b4fed164701f1ea8ccecdb7a05179a2cb0e9f; git diff origin/main...HEAD (PR 2953, branch qsl-1979-export-surface, 11 files); focus: fsl-command/src/command/export.rs, fsl-command/src/command/export/request.rs, fsl-command/src/command/export/map.rs, fsl-command/src/command.rs, fsl-command/src/command/output.rs, fsl-replay/src/spine.rs, fsl-replay/src/outcome.rs, fsl-foundation/src/setting.rs, fsl-semantics/src/check/refusal.rs, tests/it/export_surface.rs"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-1979 (first, partial slice). The PR reads a `native-run/1` request's
`export` member strictly, adds the three `export.*` settings, merges `--limit`
overrides with a conflict refusal, applies the FSpec FR-811 fidelity floor, writes the
`filament.export-outcome/v1` document and the FR-286 `export` operation outcome, and
writes the target file only on `assembled`. No clause is lowered: every requested state
or temporal clause becomes an `unrepresented` record and the text is a bare module
skeleton. The rust-review lane is folded in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `export.work_units` is never charged: the mapper only refuses when the limit is exactly 0. A reviewer probe of the counter export with `limits {"export.work_units": 1}` under `minimum_fidelity any` assembles, exit 0, but FR-255-AC-22 requires `incomplete` naming `export.work_units`, bound 1, exit 22, and FSpec FR-817 charges one unit per node mapped and per region written. The `incomplete` member also carries no bound (`{"limit": name}` only). The PR body's "a denied charge is `incomplete`, exit 22" is therefore true only for output_bytes and nodes | fsl-command/src/command/export/map.rs:70-82; fsl-command/src/command/export.rs:285-289 |
| FND-002 | high | Under `minimum_fidelity any`, the skeleton is reported as success. The run gets `status` `assembled`, exit 0, a `package` key, and a written file holding an empty `---- MODULE M_… ----` / `====` module, and every record carries `target-unrepresentable`. FSpec FR-811 "Coverage" marks state clauses and invariants `preserved` for both profiles, so the cause blames the target for a gap that is really the mapper's. Downstream readers (FR-1022/FSpec FR-815 evidence intake by package key) cannot tell this apart from a real target limit. FSpec FR-817 Exits gives "a mapper failure `30`", and the PR even defines `RunCause::ExportMapper` (RuntimeInvariant) but never constructs it. Needs either the mapper-failure exit or an explicit spec ruling (see spec question Q1) | fsl-command/src/command/export/map.rs:95-104,112-120; fsl-command/src/command.rs:707-714 |
| FND-003 | medium | The cause record does not follow FSpec FR-811's `target-unrepresentable{target, construct, node, locus}`. Probe output shows `target` is the profile id (`filament.output.tla2/v1`) where FR-811 wants the family (`tla`); `construct` is the clause kind (`invariant`) where it wants the node's `node_tag`/`semantic_form`; `node` is the clause name where it wants the node id; `locus` is prose (``state clause `Bounded` ``) where it wants the FR-203 source region | fsl-command/src/command/export/map.rs:112-120 |
| FND-004 | medium | `initial` is read and parsed as any JSON, then thrown away (`let _ = (&admitted, &initial);`). It is never checked as a `filament.state.snapshot/v1` document and never reaches the output. A probe with `{"format": "not-a-snapshot"}` as the initial file exits 0 `assembled`. This is a member accepted and silently ignored. It should either refuse until Init is lowered or validate the format and record that it was not used | fsl-command/src/command/export.rs:185-195,226 |
| FND-005 | low | `export.nodes` is charged as `obligations + 1`, not as the nodes of the dependency closures read (FR-255, FSpec FR-817). `export.output_bytes` is compared only after the whole text is built, which is not a charge as each byte is written. Both are fine for a skeleton but will undercount once lowering lands | fsl-command/src/command/export/map.rs:70-77; fsl-command/src/command/export.rs:346-351 |
| FND-006 | low | If the package this run just compiled fails `admit_checked_package`, the run refuses `invalid-request` (exit 20) and puts a `{refusal:?}` Debug dump in the message. That failure is an internal invariant (exit 30), not a caller error | fsl-command/src/command/export.rs:215-225 |
| FND-007 | low | `observer`, a valid FSpec FR-817 member, is refused `invalid-request` exit 20 ("not run by this build"), which tells the caller their well-formed request is malformed. Unsupported-in-this-build is not caller input error (see Q1) | fsl-command/src/command/export/request.rs:171-176 |
| FND-008 | low | Nits. The existing-path refusal on the export path reads "a compile never overwrites it" (probe P7). `RunCause::ExportMapper` is dead code. `status` from `assembled()` is only used by a tautological `debug_assert!(!status.is_empty())`. `mod export_surface;` is out of alphabetical order in tests/it/main.rs | fsl-command/src/command.rs:702,707-714; fsl-command/src/command/export.rs:292; tests/it/main.rs:82 |
| FND-009 | high | Obligation selection contradicts FR-1015 as amended on main by W53 (4ea19785d, FR-1015-AC-6), in three ways. `obligations: []` is accepted and exports every state and temporal clause; probe P3 gives three records, including the precondition `CanInc` and the postcondition `Inc`. It should refuse `invalid-value` at `/request/export/obligations`, exit 20. An absent member gets the generic "expected an array" message, not `missing-member`. An undeclared name (`Nope`) refuses the whole request, exit 20 (probe P4), and a pre, post or constraint clause becomes an exported obligation. Both should give a `refused` record with `not-an-obligation{name}` in request order, which here means exit 21 | fsl-command/src/command/export/map.rs:54-69; fsl-command/src/command/export/request.rs:193-199; fsl-command/src/command/export.rs:227-252 |
| FND-010 | medium | Stop order is inverted. `export.output_bytes` (22) is checked before the floor (21), so a below-floor request with `export.output_bytes` 1 exits 22 (probe P5). Amended FR-1015 ("admission, then classification, then the floor, then the text") and FR-1015-AC-7 require exit 21 under any `export.output_bytes` | fsl-command/src/command/export.rs:346-366 |
| FND-011 | medium | The generator is `{"identity": "filament-spec-language/export", "version": fsl-command's CARGO_PKG_VERSION}`, which comes out as 0.1.0 while the application crate is 0.3.0. Amended FR-1015 and FR-1015-AC-8 require `{"identity": "filament-spec-language", "version": filament_spec_language::VERSION}`, the same pair `fsl version` reports, with no `/export` suffix and no crate version | fsl-command/src/command/export.rs:21-22,371 |
| FND-012 | medium | The branch predates W53 (4ea19785d) and does not merge cleanly with origin/main (`git merge-tree` reports a conflict; main adds tests/it/export_rulings.rs and a `mod` line in tests/it/main.rs). It needs a rebase onto main, and the ruling stubs keyed QSL-1979 (FR-1015-AC-5..8) are now the authority for FND-002/003/009/010/011 | tests/it/main.rs:82 |

## Scope and evidence

- Read every changed file in full. Spec read: FSL FR-1015, FR-255 (export rows, Behavior, AC-22), FR-286 "Export outcomes" and AC-22, and FSpec FR-811 and FR-817 at agent-ix/filament-specification origin/main 0bf58005. Also read the QSL-1979 and QSL-2359 ticket comments (W21/W47 rulings).
- Reviewer probe: a temporary local test in the review worktree, removed before cleanup and never pushed. It ran over the counter fixture and checked each refusal and exit code:
  - Limits conflict `limits` + `RequestOverrides::limits` on export.output_bytes: invalid-request, member `limits`, names the setting, exit 20, stage request. Correct.
  - Override-only `export.output_bytes` 1: `incomplete` naming the setting, exit 22, FR-286 category incomplete, result kind `exported`. Correct.
  - Below floor: exit 21, category unsupported, `unsupported_projection`/`below-fidelity-floor`, failing records with causes, no `target_text`, no `package`. Correct.
  - Existing path: invalid-request naming the path, exit 20, stage request (no stage reached), file untouched. Correct.
  - Member + `--output` override: names `output` and `--output`, exit 20, neither file written. Correct.
  - Override-only output: written, `output.path` as given. Correct.
  - target_family mismatch, unknown member, unknown or non-integer, negative or float limit, unknown obligation: each invalid-request at its `/request/export/...` pointer, exit 20. Correct.
- Under `minimum_fidelity any`, `fails()` returns false for every record, which matches FSpec FR-811 ("under `any`, nothing fails it"). `conditional` fails `unrepresented`, and `preserved` fails everything not preserved. Correct.
- The target is written only for `Category::Success`, after assembly, through `write_new_file` (temp file plus hard link, never overwrites, no partial file). No file is written on any refusal or incomplete path. The existing-path check runs before any stage.
- Ordering: a request that is both below the floor and over output_bytes reports `incomplete` 22, because the size check runs first (probe P5). The spec states no precedence (Q8).
- No `#[allow]` in the diff. No `unwrap`/`expect`/`panic` on request input: `unwrap` appears only in tests, and `as_u64` / `try_from` with fallbacks are total. The extended `unreachable!` in `CompileRefusal` stays unreachable, because no compile stage charges an `export.*` setting.
- Tests: `cargo-slot cargo test --test it -- export_surface:: --test-threads=1` plus the house skips gave 5 passed, with the plan_export_surface stubs ignored. Clippy results are in the reviewer report.

- After the first post, the W53 ruling comment on QSL-1979 (84b1f169, merged as
  language 4ea19785d) was read. It amends FR-1015 on main with the rulings that
  FND-002 and FND-003 anticipated: no cause for a construct that has not been lowered,
  mapper failure `internal-failure` exit 30, and the cause shape. It also adds
  FND-009..011. FND-002 is confirmed by that text.

## Verdict

The request reader, the limit merge and conflict refusal, the floor arithmetic, the
FR-286 outcome and the file custody are correct, and every refusal checked matches its
spec exit code and spelling. Not mergeable as is. FND-001 is a spec violation under an
AC this PR tags (FR-255-AC-22). FND-002 makes a skeleton read as an `assembled`, exit-0
export carrying causes that FR-811 contradicts. Fix both in this PR, or get an explicit
spec ruling on FND-002. FND-003 and FND-004 are cheap in-PR fixes. The W53
amendments now on main (FND-009..012) make FND-002/003 binding and add the
obligation-set, stop-order and generator rulings. The branch must be rebased and
brought to FR-1015-AC-5..8 before merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | A mapper failure writes an outcome with `last_stage` `null`, and the native result reports `stage` `request`, although the program was compiled and packaged (S4) before the mapper ran (probe C1: category internal-failure, exit 30, `last_stage` null, `result` null). FR-286 "Export outcomes" says `last_stage` is the greatest compile stage the export's program reached (S4 for an export of a checked package), and `null` only for a refusal of the request itself. The `RunCause::ExportMapper` error goes through the generic `RunError::outcome_document` arm with `None` for the stage | fsl-command/src/command/export.rs:386-395; fsl-command/src/command.rs:1240-1250 |

## Dispositions

Round 1, head d0b22461599864133bc136765ba904d34a4b10ed (fix commit d0b224615 on top of merge cac423ef2). Verified by reading the code and by a temporary reviewer probe (never committed), as well as the PR's tests.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `Meter::charge` charges one node and one work unit per distinct dependency-closure node (shared nodes once, via `charged`). work_units 1 and 2 stop `incomplete` `{limit: export.work_units, bound}` with exit 22 (probe C3, test `the_closure_of_an_obligation_is_charged_node_by_node`) |
| FND-002 | fixed | A representable obligation with no lowering yields `Unlowered` and no record. After the floor, `assembled()` returns `RunCause::ExportMapper`: internal-failure, exit 30, no package, no file, under default, `any` and `preserved` floors, and with the `--output` override (probes C1, C6; test `an_unlowered_invariant_is_a_mapper_failure`) |
| FND-003 | fixed | Causes now read `target` `tla`/`lean`, `construct` `node_tag/semantic_form`, `node` the package node id text, and `locus` the source-map region in FR-286 form `{kind: region, source, start, end}`, with one cause per distinct flow node (probe B2/B4). `locus` is null when the node has no source-map region (spec question, not a defect) |
| FND-004 | fixed | `initial` must parse with `format` `filament.state.snapshot/v1`, or the request refuses at `/request/export/initial`, exit 20 (test `the_initial_member_must_name_a_snapshot`). It stays unused, which is in scope: Init is FR-1035 |
| FND-005 | fixed | Nodes are charged per closure node. `export.output_bytes` is still compared after the text is built, but that now matches the amended stop order (text last) |
| FND-006 | still-open | Unchanged: an admission failure of the run's own compiled package still refuses `ClauseRequest`, invalid-request exit 20, with `{refusal:?}` in the message (export.rs:245-255). It should be `RunCause::ExportMapper` or another internal cause, exit 30 |
| FND-007 | deferred | `observer` is still refused invalid-request, "not run by this build". W53 did not rule on it, and the coordinator has asked arch; outcome follows that ruling |
| FND-008 | still-open | The overwrite message, the use of `ExportMapper` and the tautological `debug_assert` are fixed. `mod export_rulings;`/`mod export_surface;` still sit before `mod exact_decimals;` in tests/it/main.rs, which is not alphabetical (a one-line nit) |
| FND-009 | fixed | An absent `obligations` refuses `missing-member` and `[]` refuses `invalid-value`, both at `/request/export/obligations`, exit 20 (probes B5/B6, test `obligations_are_nonempty_declared_names`). Pre, post and constraint clauses and undeclared names give `refused` records `not-an-obligation{name}` in request order (probes C4/C5) |
| FND-010 | fixed | `assembled()` now runs in this order: the floor (21), then the mapper failure (30), then `export.output_bytes` (22). Classification limits are charged in `map::run` before that. The below-floor request with output_bytes 1 now exits 21 (test `the_floor_is_decided_before_the_text_is_charged`) |
| FND-011 | fixed | `generator` is `build_inventory(..).application`: `{"identity": "filament-spec-language", "version": "0.3.0"}` (probe), equal to `filament_spec_language::VERSION` (test `the_generator_is_the_application`) |
| FND-012 | still-open | W53 is merged in (cac423ef2), but the branch again conflicts with origin/main. `git merge-tree` reports a content conflict in fsl-replay/src/outcome.rs against 2ad065147 (#2940, replay result rows), so a rebase or merge is needed before merge |
| FND-006 | fixed | An admission failure of the run's own compiled package is now `RunCause::ExportMapper` (internal-failure, exit 30) with a fixed message and no Debug dump |
| FND-007 | fixed | Following the W61 ruling, a valid `observer` is no longer invalid-request: it refuses `unsupported_projection`/`unsupported-requested-capability` naming the observer, exit 21, no package or file, before any limit charge (probe O1; test `an_observer_the_build_cannot_run_is_unsupported`). The remaining W61 gaps are FND-014 and FND-015 |
| FND-008 | fixed | tests/it/main.rs now orders `mod exact_decimals;` before `mod export_rulings;`/`mod export_surface;` |
| FND-012 | fixed | `git merge-tree --write-tree origin/main 94d3a8264` merges cleanly; W61 (38bf72188) and #2940 are merged in |
| FND-013 | fixed | `RunError::outcome_document` gives `ExportMapper` `last_stage` `S4`, and the native result reports stage `emit` (probe M1: category internal-failure, exit 30, last_stage S4, result null; asserted in `an_unlowered_invariant_is_a_mapper_failure`) |

Round 2, head 94d3a82647b51c8eac60a9316f8f98aa04533076 (fix commit 94d3a8264 on merge a740f2a16). Verified by code reading, the focused tests (20 passed) and a temporary reviewer probe (never committed). Round-2 rows cover only findings whose latest outcome was still-open or deferred.

Round 3, head dcd70c8d0527b009437d513a39fecf34338417a8 (fix commit dcd70c8d0). Verified by code reading, the focused tests (20 passed) and a temporary reviewer probe (never committed). The `RunCause::Io` message now reads "cannot access {path}", which covers a write failure; no test asserts the old wording.

| FND | outcome | reason |
| --- | --- | --- |
| FND-014 | fixed | The observer refusal is now raised in `run_bytes` right after `request::read`, before any stage. `last_stage` is null, the refusal and an outcome diagnostic carry `details` `{"path": "/request/export/observer"}`, there is no package and no file (probe X1; asserted in `an_observer_the_build_cannot_run_is_unsupported`) |
| FND-015 | fixed | `observer` must be one of `sany`/`tlc`/`apalache`/`lean`; any other name refuses `invalid_runtime_input`/`invalid-value` at `/request/export/observer`, exit 20 (test case `coq`) |

Round 4, head 5986cfee3fe3b496055d808f0b18c5485b109546 (fix commit 5986cfee3). Verified by code reading and the focused tests. The arch ruling is implemented as stated: a write failure after a successful export is `RunCause::ExportOutput` (code and cause `output-failure`, category internal-failure, exit 30, `last_stage` S4, stage `emit`); a late `OutputExists` race stays exit 20. The observer refusal keeps `capability`/`reason`, per that ruling.

| FND | outcome | reason |
| --- | --- | --- |
| FND-016 | fixed | The observer check now runs after the `output`/`handoff` refusal and `target_path`. An existing path and a member plus override both refuse `invalid-request`, exit 20, `last_stage` null, with the file untouched (test `output_admission_refusals_precede_the_observer_refusal`) |
| FND-017 | fixed | `unsupported_observer` writes `output` `{"path": <as given>, "bytes_written": 0}` through the shared `output_name` (test `an_unsupported_observer_names_its_output_with_no_bytes`) |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | medium | The unsupported-observer refusal does not match W61 FR-1015 Behavior in two ways. It writes `last_stage` `S4` (probe O1), where Behavior says "`last_stage` null" and places the refusal "after admission and before classification". The check runs inside the `check_sources` closure, after S1 to S4 compile and packaging, and `finish()` always stamps S4. It also carries no `details` `{"path": "/request/export/observer"}`, which Behavior requires; the refusal object has `capability`/`observer`/`reason` only, and the outcome has no diagnostic. Moving the check ahead of the compile (right after `request::read`) would give a null `last_stage` and avoid a wasted compile | fsl-command/src/command/export.rs:286-290,327-338 |
| FND-015 | low | `observer` is not checked against FSpec FR-817's four structural observers (`sany`, `tlc`, `apalache`, `lean`). `"observer": "bogus"` is refused `unsupported-requested-capability`, exit 21 (probe O2), so a misspelt name reads as a missing build capability rather than a malformed member (`invalid_runtime_input`/`invalid-value`, exit 20). Behavior says the member "names one of" the four | fsl-command/src/command/export/request.rs:193 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | medium | The observer refusal now runs before the admission refusals that FR-286 "Export outcomes" lists as exit 20. W61 says the observer refusal "comes after admission", but an observer request whose output path already exists, or that names both `export.output` and the `--output` override, now exits 21 `unsupported-requested-capability` instead of exit 20 `invalid-request` (probes X2, X3). The `output`/`handoff` compile-member refusal is pre-empted the same way. Moving the check below `target_path` keeps `last_stage` null and restores the admission order | fsl-command/src/command/export.rs:129-143 |
| FND-017 | low | The observer refusal outcome has no `output` member although the request named one (probe X1). FSpec FR-817 says `output` is present exactly when an output file was named, with `bytes_written` `0` when none was written, and round 2's refusal carried it. `unsupported_observer` builds the document without it | fsl-command/src/command/export.rs:150-190 |
