---
id: SR-202611210022442313144108506
title: "gap-analysis of QSL-1463 monitor checked-term origin rekey"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-package/src/emit/monitor_plan.rs, fsl-analyze/tests/it/monitor_root.rs; FR-251-AC-9, FSpec FR-322-AC-309, FSpec FR-487"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: references
---

## Summary

Ticket: QSL-1463. This is a planless gap analysis of frozen draft PR #3281 at 88a324b6344c1b51129d51fa5c6e603367181492, scoped to the actual diff (origin/main...HEAD, base f3aa681d7e).

The changed behavior is the plan-root writer `finish` re-keying the `checked_term_data` sidecar through every writer rename. FR-251 owns that behavior: the plan root is admitted by the strict reader, and the writer re-keys through every rename it performs. FSpec FR-322 also owns it: body_locations name a local `function` node, and declared helpers keep their declaration key (FR-322-AC-309). There is no reverse gap.

The one new binder targets FR-251-AC-9 and FSpec-FR-322-AC-309. Both bindings are correct in intent. However, the binder does not compile (see the code-review SR-202611210022442313144108505 FND-001), so the producer fix has no executable regression evidence.

## Verdict

**FAIL**: the only new evidence for the fix cannot execute. Once the refusal assertion is corrected and the delegated focused test is green, no gap remains in this PR's scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new FR-251-AC-9 / FSpec-FR-322-AC-309 binder `monitor_body_locations_follow_entry_rekeys_and_preserve_helper_origins` references the nonexistent `CheckedPackageRefusalCode::InvalidValue` (it exists only as a `CheckedPackageRefusalCause` at contract-model 7b76c8b). The binder is counted as `tagged` in the computed matrix but can never run, so the 8-line producer fix rests on hollow evidence until the assertion is corrected. | fsl-analyze/tests/it/monitor_root.rs:180 |

## Coverage

Plan completion: not assessed

Mode: planless. No plan was supplied, and none was selected.

Matrix: `quire matrix --scope /home/peter/dev/worktrees/qsl-1463-monitor-origin-rekey --format json` (quire 0.52.0, engine 0.62.2), exit 0. Raw output is in matrix.json and matrix.stderr beside this artifact. FR-251-AC-9 has status `tagged` with 5 binders, including the new test at monitor_root.rs:91. FSpec-FR-322-AC-309 is a cross-repository id, following the repository's established `FSpec-FR-` tag convention; it was checked against filament-specification origin/main FR-322-AC-309 text. `quoin matrix --json` was not used: it hung on this repository in a sibling review, and the documented static fallback was taken.

Bindings examined: new test → FR-251-AC-9, correct (the admitted plan root with its four entries as content-keyed functions). New test → FSpec-FR-322-AC-309, correct (entry keys differ from their declared keys, and the helper keeps its declaration key and origin).

Reverse gap: zero unowned behaviors. The producer change is a sidecar re-key owned by FR-251 and FSpec FR-322.

Stubs and coverage inflation: none in the diff, apart from FND-001's non-compiling binder.

Semantic review: performed for this diff only, as the brief explicitly requested.

Not run, per the brief: cargo build and test. The delegated focused test is PENDING; no runtime pass is claimed. Out of scope and not counted: IR-1321, and the existing repository-wide untagged rows and symbol-id-collision diagnostics.

## Dispositions

Disposition round 1 was checked against fixed head 8297b136461fa2636fa84d57923721a4bbc4b8c9, fix commit 8297b13646, which is test-only. No new gaps were found.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-251-AC-9 / FSpec-FR-322-AC-309 binder now names only existing reader variants (code InvalidPackage, cause Some(InvalidValue)), so it no longer rests on a non-compiling assertion. Executable evidence is PENDING the delegated exact test. |

Disposition round 2 was checked against fixed head 1a10181328e188e8d2abb3dd4669a2fd3e33e63a (commit 1a10181328, on top of 0a036e69f9). It has no rows, because no finding is still open; every finding stays `fixed` from round 1, and no new findings were found.

The round-2 commit changes only the new test, `fsl-analyze/tests/it/monitor_root.rs`:

- It replaces `root_of(..)` with a direct `emit_plan_root(&PlanRootRequest { .. })` call. Domain, packages, source, name, path, clause, target, settings and Cancel are identical to `root_of`, so the full genuine emission and strict read-back path is unchanged.
- The only difference is `limits: SpineLimits { imports: default.imports.with_work(LIMITS.work /* 1e9 */), ..default }`. `imports` is the I2 bound that `fsl-analyze/src/monitor_root.rs:276` passes to `emit_monitor_plan_over` for the base read-back, which is where IR-1321 exhausted 1e6 at `/checked_term_data/body_locations/464/occurrence`.
- `with_work` (`fsl-package/src/checked_v2.rs:315`) sets only `work`. The bound is caller-scoped to this fixture. `SpineLimits::default`, `root_of` and all production code are unchanged.
- The required comment names IR-1321, the quadratic rescan, and the removal of the explicit bound once the fixed reader is pinned.
- Every assertion is kept: all four entry ids, the helper, location, occurrence, term_ordinal, application, and the stale-origin code, cause and path.
- `rustfmt --check` exited 0. The prior SRs under `reviews/` are byte-identical to disposition1 (`cmp` 0).
- Runtime: the delegated exact test at 1a101813 is PENDING. No pass is claimed.
