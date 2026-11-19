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
