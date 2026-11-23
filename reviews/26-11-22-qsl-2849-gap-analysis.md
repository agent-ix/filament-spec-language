---
id: SR-3554002
title: "gap-analysis of QSL-2849 slice 2 against FR-264, FR-126 and the ticket acceptance (filament-spec-language PR 3554)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; fsl-package/src/emit.rs; fsl-eval/src/simulation/protocol/reach.rs; spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md"
review_set: subset
---

# Gap analysis of QSL-2849 slice 2 (filament-spec-language PR 3554)

## Summary

Ticket: QSL-2849. This is a gap analysis scoped to the PR diff at head 0a62aae839, not a full
repository audit. It used a manual check of acceptance criteria against tests: the computed
`quoin matrix` was not run, because this PR changes no spec file. Plan completion: not assessed.

Units examined: FR-264 Behavior 5, FR-264-AC-6, FR-264-AC-7, FR-126-AC-22, FR-126-AC-23,
FR-850-AC-15 (replay request-level, by code reading), and the ticket's acceptance ("per site, a test
that exceeds the limit and shows the refusal arrives before the allocation or loop").

Bindings checked: `a_bound_below_the_source_map_refuses_before_the_typed_rows_are_built` traces to
TC-738 and FR-264-AC-6. It asserts AC-6's bound and actual at bound 0 and at N-1, so the binding is
correct. `a_level_over_the_state_budget_refuses_within_one_chunk_of_expansions` traces to
FR-126-AC-23. It asserts the same refusal value across worker limits 1, 2, 4 and 8, so the binding
is correct.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The emit site is only partly closed against the ticket ("move the check before the work it bounds, or meter the work as it is done"). Only a package whose source map alone exceeds `emit.output_bytes` now refuses early. A package whose source map fits but whose typed rows overflow still builds every typed row before the final encode refuses it: temporal bindings, claim names, equality rows, dispatch tables, source type rows, requirement rows and operand rows. `body_locations` is the only exception, because it was already metered per row. QSL-2849 lists this site as open, and the PR uses `Refs`, so it can be deferred to a later slice. Record it on the ticket so the site is not treated as done. | fsl-package/src/emit.rs:7840-7977 | missing-requirement |
| FND-002 | low | No spec AC states the property this slice delivers: that the refusal arrives before the work it bounds. FR-264-AC-6/7 pin only the refusal's values, and FR-126-AC-23 pins only that the refusal is the same across worker counts. Both new tests trace to ACs that a whole-level or after-all-rows implementation would also pass, so the early-refusal behaviour has no requirement owning it, and a later change could undo it while still meeting every AC. Add an AC to FR-264 Behavior 5 and to FR-126's direct-search section, for example "refused before rows beyond the source map are built" and "at most `worker_limit * 64` expansions past the crossing state". | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:608 | missing-requirement |

## Verdict

The PR's claims hold, apart from the open scope above. The emit source-map floor is sound (see
SR-3554001). The protocol chunking is deterministic and refuses within one chunk. The replay
request-level `input_bytes` check runs before `package_input` and `admit_package` on all three
package routes. fsl-bench's 10,000,000 is the FR-255 default, which is correct for a benchmark.
FND-001 can be deferred, since the ticket stays open. FND-002 is a spec follow-up.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Partly closed: the source map and the temporal, claim, equality, dispatch, protocol-location, probabilistic and requirement row sections are charged as each is built. Still open, and stated in the PR body under "Still open on QSL-2849", with the PR marked Refs: per-row metering and the semantic nodes, which are built before the source map is complete. The work left after the last charge (operation bodies, correlation rows, operand slots, the final encode) stays on QSL-2849. |
| FND-002 | deferred | A spec-only change (an early-refusal AC in FR-264 Behavior 5 and FR-126) outside this code PR. Track it on QSL-2849 or as a spec follow-up. |
