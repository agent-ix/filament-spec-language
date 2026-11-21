# QSL-2758 / QSL-2760 focused receipts

All commands exited. These are focused iteration receipts, not a final gate.
The lead owns the independent diff review and final checked-head integration.
Each Cargo command used the populated shared CARGO_HOME, this worktree's
retained target, debug=0, incremental=0, and parallelism derived from current
CPU/load/available memory. Later commands used `--test-threads=1` as directed.

| Evidence | Exit | Result | Log |
| --- | ---: | --- | --- |
| fsl-command `canonical_run_values` | 0 | 3 named tests pass: exact/normalized values, strict legacy refusal, independent item identities/order | `logs/canonical-run-values-integrated-2.log` |
| fsl-replay catalog normalization-owner denial | 101 aggregate | Catalog test passes; the other test in that invocation fails on its original incorrect conversion-only fixture | `logs/canonical-run-accounting-3.log` |
| fsl-replay corrected genuine cumulative admission + execution | 0 | 1 pass, original supplied Admission counter and typed limit stop | `logs/canonical-run-cumulative-4.log` |
| Source population expansion/reference reuse | 0 | 1 pass: document order, native Population argument, canonical roundtrip and external legacy refusal | `logs/canonical-run-population-2.log` |
| Authoritative values/plans | 101 aggregate | Values test passes its 16 authored ready cases. Plan test fails at plan-passes with actual ambiguous-backend, not invalid-request | `logs/canonical-run-authoritative.log` |
| Plan caller's existing named selector, after arch's final ruling | 0 | 1 named test: passed0, refuted10, refuted+blocked22, null-as-absent0; explicit unknown backend refuses20 without fallback | `logs/canonical-run-plans-selected-2.log` |
| Migrated source/package/budget/law/handoff fixtures | 0 each | 16 named passes before the reused-claim failure | `logs/migration-*.log` |
| Provider limits/overrides, previously unrun after loop stop | 0 | 1 pass on the actual #3393 pin base | `logs/migration-provider_run_accepts_run_limits_and_request_overrides.log` |
| Reused-claim regression, original then exactly one authorized new-base rerun | 101 each | UnknownRequirementClaim persists; no assertion weakened and no further rerun | `logs/migration-a_run_call_names_the_backend_of_its_requirement_items.log`, `logs/reused-claim-21ed.log` |
| Genuine checked PACKAGE canonical reference roundtrip | 0 | 1 pass: actual handoff/witnesses, removed original source, same-admission NormalizedModel catalog and two package calls | `logs/canonical-run-package-reference.log` |
| Value-only call at zero model work | 0 | 1 pass: canonical output introduces no unrelated model-normalization charge | `logs/canonical-run-zero-model-work.log` |

The seventeen migrated passes are the named tests for completed values,
argument refusal, exhausted call work, request/dependent byte ceilings,
deterministic run output, prepared source execution, package-only calls,
package items, package snapshots, prepared package execution, integer division
laws, independent program/library laws, guarded integer division, transitive
and diamond handoffs, nested library composites, and provider limits/overrides.

Original failures remain in the log directory. They include missing exhaustive
Reference refusal projections; a missing borrowed-writer Reservation import;
the upstream temporal-test source repair; a stale standalone-budget re-export;
and the initial cumulative test's incorrect assumption that identity execution
spends the Conversion counter. The implementation now borrows the same run for
real canonical membership and interpreter admission; reference codec work still
uses its separate Conversion counter with the real argument/preorder locus.
One short-name `--exact` population command was stopped immediately, exit 130,
before tests; it is not counted as validation.

The authoritative examples checkout was used in place and was never modified
or copied. Its staged items array is explicitly invalid under the arch ruling:
FR-651 retains its specified object envelope, exercised by the independent
three-item and population regressions. Plan fixtures retain desired assertions;
no current wrong refusal is accepted as passing. The authorized selector
implementation now threads `/request/requirements/backend` through the actual
plan items and existing negotiation; `en-1` is the model-check manifest's
advertised backend (`fsl_route::tie::EN1`). Only private selector-bearing request
documents are materialized, with absolute references to the read-only originals.
The initial selected-plan test assumed a backend field that the existing result
envelope does not publish; its failure is retained in
`logs/canonical-run-plans-selected.log`, and the corrected test proves selection
with actual outcomes and an unknown-backend refusal. FR-753 gains one sentence
for the selector edge. The reused-claim producer defect is held under QSL-2744;
its test is unchanged and was not rerun again. The prior 26 accepted passes were
not repeated during this plan-only completion.

Dependencies integrated include actual main #3393
`21edb2ead1c8b69768beedaab003e077ce4ed042`, with IR `96554ea5` and both FSpec
aliases `f6b0480c`. No independent pin edits were made. Shared owner/codec/writer
helpers were separately published on `codex/qsl-2758-shared-reference-helpers`
through `700811ffd760f4ebe89fd5765e9a99e6e7ab1f97` for the existing reference
lane, without another PR, test run or review.

QSL-2760 remains open until the driver actually adopts the coordinated run API.
No full suite, aggregate gate, profiling, or peer-fixture rerun was performed.
