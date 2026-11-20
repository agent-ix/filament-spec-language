---
id: SR-2026112019364601
title: "code-review of PR 3214 package route batch 2 second PR"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@aaa5643860cf44b49faa592a17348f077832d009; git diff origin/main...HEAD (33 files): fsl-analyze, fsl-command (package_run.rs, tl.rs, tl/canonical.rs), fsl-package (bare_term.rs, reader_protocol_projection.rs, wire_form.rs, emit tests), fsl-semantics (checked_dispatch.rs, lowering/model.rs), fsl-eval/temporal, fsl-replay/spine, reviews, spec FR-931/FR-940, tests/it"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: reviews
---
# SR-2026112019364601: code-review of PR 3214

## Summary

Ticket: QSL-2726. PR agent-ix/filament-spec-language#3214, head aaa5643860, base origin/main ad5547bde6. This review includes the rust-review lane. The PR adds a print mode for tl_lowering atom text over `{file}` and `{file, equality}`, FR-620 default items over a package selection, the owed tests, and emitter identity fixes (source tuple/union clock places, effective_precondition function, source population node). The commit also silently reverts all of #3213 (QSL-1848/QSL-2673).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The single commit reverts every change of #3213 (QSL-1848, QSL-2673), main's head. Its tree equals the WIP commit a1562b1ad4, which was made on 3da51c9719 (main before #3213), committed after `reset: moving to origin/main`. The 17 files #3213 touched are byte-identical to ad5547bde6^ at HEAD. Lost: timed observed-trace evaluation, timed monitor traces, checkpoint resumption positions, their tests, and reviews/qsl-2673-*.md. Fix: rebase the PR's own changes onto origin/main so that `git diff origin/main...HEAD` no longer touches those 17 files | fsl-eval/src/temporal/observed.rs; fsl-replay/src/spine/clause/temporal.rs; fsl-replay/src/spine/request.rs; fsl-analyze/src/monitor.rs; fsl-analyze/tests/it/monitor_inputs.rs; tests/it/run_wire.rs; reviews/qsl-2673-code-review.md |
| FND-002 | high | Package default items do not match the source route's (FR-620). `default_selections` gives one item per claim with no `kind`, which resolves to a single fixed kind (TemporalSatisfaction or OperationContract), and appends only a deadlock-freedom item per model selection. Source `default_items` gives one item per kind the claim's records admit (Capability::ALL order), then task-set schedulability claims, then FR-368 or protocol derived items. A package whose claim admits two kinds, or that has a task set or a protocol subject, gets a different item list over the package. Fix: build the package defaults the same way as `default_items` (per-kind records, task-set claims, the same derived-item builder) and add a fixture that has a multi-kind claim | fsl-analyze/src/analyze/document/package_run.rs:1505-1555; fsl-analyze/src/analyze/document.rs:2358-2460 |
| FND-003 | medium | Default-item order sorts by (authority string, identity string, start byte). FR-620 orders units in request order (for a package, its lock source order), not by identity string, so a multi-source package can come out in a different order. Fix: key on the unit's position in the package's source list, as `default_items` does with `units.position` | fsl-analyze/src/analyze/document/package_run.rs:1513-1532 |
| FND-004 | medium | `bare.world().ok()` discards every world failure, including faults and limit refusals, not only a missing identity row. A malformed or over-limit `{directory}` identity file now prints silently instead of being refused, and FR-940's new text says the print "builds no world" while the code still builds one. Fix: do not build the world in print mode (use declared composites alone), or keep the error for every cause except missing identity rows | fsl-command/src/command/tl.rs:1387 |
| FND-005 | medium | Three new emitter tests are traced to ACs they do not assert: FR-230-AC-65 (clock places w.c/d) on a Clock-parameter constraint/reset test, FR-996-AC-1 (d and w.c rows) on the tuple/union spelling test, and FR-569-AC-6 (object_type nodes) on the effective_precondition test. Fix: trace them to the FSpec rows they check (FR-789-AC-4, FR-322 item 5) or to ACs that state the behaviour | fsl-package/src/emit/tests/source_objects.rs:1697,1732,1767 |

## Scope and evidence

- Print mode (examined): `project_holds_for_print` skips `has_unrepresented_types` and falls back to `declared_type` / `print_effective_id` only when `node_value_type` is Unsupported. Faults still propagate. `CanonicalRows::from_projection` merges world composites with declared composites and keeps a world row first.
- FR-1039 equality read (examined): `RunCause::Limit` now passes through `read_equality_file`, which closes the prior FND-007 of #3207.
- Effective precondition (examined): a single-term source candidate goes through the combinator branch. `reduce` over one term gives that term, and `source_declaration` names `[T, op, "effective_precondition"]`.
- Clock place spelling (examined): `PackagePlace.source` is true only for source timed models and for fields when `source_package` is set; model-package places keep `/tuple/i` and `/union/case`.
- Population reference (examined): `insert_ref` with `declaration [name]` attaches the matching SourceOwner, per FSpec FR-322 item 4.
- Ignored seq test (examined): see the gap analysis. It is a new test added ignored with an honest reason, not an existing test bent to pass.
- Control runs: see the `## Control runs` section.

## Control runs

`cargo test -p fsl-package --lib -j 2 -- --test-threads=2` through qsl-slot. Main is ad5547bde6 in /Users/peter/dev/qsl-2631-code; the PR is aaa5643860 in /Users/peter/dev/qsl-2726-code.

- main: 926 passed, 52 failed, 51 ignored. PR: 930 passed, 51 failed, 51 ignored (4 new tests).
- Red on the PR and green on main (regressions): none.
- Green on the PR and red on main: `emit::tests::source_objects::fixtures_s_s_r_and_s_p_emit_source_objects`.
- The other 51 failures are on main as well (protocol, hybrid, timed families), so they are not caused by this PR.
- The three new source_objects tests pass on the PR.
- Root `--test it bare_package` was not compared. On main, `cargo test --test it -- bare_package` matches 0 of 1063 tests because `mod bare_package` is behind `#[cfg(feature = "checked-handoff")]`. The comparison needs `--features checked-handoff` and a full rebuild on both sides, which this review did not run.

## Verdict

Not mergeable. FND-001 alone blocks merge: it deletes main's newest feature. FND-002 is behaviour claimed done that diverges from the source route.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The `types` parameter of `CanonicalRows::from_projection` is now always `None` (its one caller passes `None`), so the world-composite loop is dead code. Fix: drop the parameter and the loop | fsl-command/src/command/tl/canonical.rs:35-66; fsl-command/src/command/tl.rs:1398 |

## Dispositions

Round 1, head 9e84cf3919 on main 5865aa2ffe.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `git diff origin/main...HEAD --stat` touches none of #3213's 17 files. The only line overlapping with #3202/#3210/#3153/#3215/#3217/#3219 is an import reorder that rustfmt 2021 produces |
| FND-002 | fixed | One item per kind in Capability::ALL order, pre/post state clauses included, task-set claims after the other claims, then deadlock-freedom. Per the arch ruling, the families with no package record rows refuse typed instead of giving a different list: a kind-less claim (refinement, hyperproperty, probabilistic) and a schedulability claim go to `Pending(Declaration)`, and a protocol subject goes to `Pending(ProtocolSubject)` |
| FND-003 | fixed | Order key is (position in `lock.sources`, start byte) |
| FND-004 | fixed | No `bare.world()` remains on the print path. `project_holds_for_print` reads only the witness rows, and `from_projection` gets `None` |
| FND-005 | still-open | The clock-parameter and tuple/union tags are fixed. But the retag removed `FR-569-AC-6` from the pre-existing `fixtures_s_s_r_and_s_p_emit_source_objects` (line 40), which does assert that AC. The new `a_source_operation_declares_its_effective_precondition_function` (line 1767) still carries `FR-569-AC-6`. Fix: restore the tag at line 40 and retag line 1767 (for example `FSpec-FR-322`, clause functions item 5, or TC-1408 alone) |
