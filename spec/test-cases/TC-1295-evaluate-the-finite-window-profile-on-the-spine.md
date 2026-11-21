---
id: TC-1295
title: "Evaluate a temporal clause under the finite-window profile on the spine"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: verifies
---
# TC-1295: Evaluate a temporal clause under the finite-window profile on the spine

## Description

Verify FR-501-AC-1 to AC-26 and FR-330-AC-9 (the finite-window part): the
spine evaluator returns a definite verdict on a closed complete window, pending
or indeterminate only for a genuinely unresolved obligation, and never refuses
the profile.

Catches a `ProfileNotEvaluated` refusal, false extension applied to a sparse
window, an empty window left unknown, a missing valuation read as false, an
incomplete input read as closed, an event at the inclusive deadline dropped, a
watermark that fails to settle a silent window, a result that depends on
ingestion order, and a repeated order key of one authority ordered by
observation identity for an order-sensitive operator, plus a package report
that changes an atom's authored text or obtains it from a source the caller
did not supply.

## Test Procedure

Each row compiles a unit selecting
`filament.temporal.timestamped-event.finite-window/v1` (clock `ms`, `Exact`
cover unless stated), builds the `TimestampedTrace` from the stated stamps,
valuations and progress, and runs it through the spine evaluator and through
`run_clause`. Stamps are in ms. `W` is the watermark, `C(t)` is
`Complete { through: t }`, `Inc` is `Incomplete`, scope is `Closed` unless
stated, and `p`, `q` are `holds` atoms of the fixture.

For rows 41 and 42, use the source and checked-package run routes with the
same caller-supplied `sources`, matching each atom region by source reference.
These are direct byte-only adapter requests after caller provisioning of
admitted `Source` values. They perform no source-file intake during execution.
Exercise a `Timestamped` trace, an FR-504 `Progressed` trace and a call carrying
the corresponding progress ledger; each fixture must produce non-empty
decision support. The package route receives the emitted checked package and
does not compile source again. Text comes only from the caller's `sources`,
including when a package handoff holds other source bytes; no implicit file
read supplies missing text.

For rows 43 and 44, exercise the command's `native-run/1` wire request with
`package` and the explicit top-level `sources` array from
[FR-1039](../functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md).
Use exactly the existing five-field source selections, for example
`{"file":"unit.native","authority":"example","identity":"unit-1","document":"doc-1","formal_revision":1}`.
The command explicitly reads those selected files during intake, then passes
the admitted sources to execution. This file intake is distinct from the
direct byte-only adapter's absence of file reads. Source-run command controls
omit `sources` and use their compilation's admitted sources.

| # | Clause (`on origin` unless stated) | Trace and progress | Expected |
| --- | --- | --- | --- |
| 1 | `always[0,5] holds(p)` | stamps 0,2,5; p = T,F,T; W 5; C(5) | `Completed(false)` at position 0 |
| 2 | same | p = T,T,T | `Completed(true)` |
| 3 | `eventually[0,1] holds(p)` | stamps 0,2,5; p = F,T,F; W 5; C(5) | `Completed(false)` at 0; with `[0,2]` `Completed(true)` |
| 4 | `always[0,1] holds(p)` | stamp 0; p T; W 1; C(1) | `Completed(true)`; the event-position control gives `Completed(false)` |
| 5 | `eventually[2,3] holds(p)`; then `always[2,3] holds(p)` | stamp 0; p T; W 3; C(3) | `Completed(false)`; `Completed(true)` |
| 6 | `eventually[0,5] holds(p)`, scope `Open` | stamps 0,1; p F,F; W 1; C(1) | `Pending {where 0, deadline 5}`; then W 5, C(5), no new event: `Completed(false)` |
| 7 | `eventually[0,5] holds(p)`; `always[0,5] holds(p)`, scope `Open` | stamps 0,1; W 1; C(1); p true at 1; p false at 1 | `Completed(true)`; `Completed(false)` |
| 8 | `eventually[0,5] holds(p)` | stamps 0,2,5; p F,absent,F; W 5; C(5) | `Indeterminate {MissingValuation, position 1, p}` |
| 9 | same, p true at 5 | p F,absent,T | `Completed(true)` |
| 10 | same | stamps 0,2,2,5; p = F,T,F,F (two observations at stamp 2 differ); W 5; C(5) | `Indeterminate {ConflictingValuation, position 1, p}` |
| 11 | row 8 with W 3 | C(3) | `Pending {deadline 5}` |
| 12 | `always[0,5] holds(p)` | stamp 0; p T; W 5; Inc | `Indeterminate {IncompleteInput}`; with C(5) `Completed(true)` |
| 13 | `eventually[2,3] holds(p)` | stamp 0; W 3; Inc; then C(2) | `Indeterminate {IncompleteInput}` both |
| 14 | `eventually[0,3] holds(q)` | stamps 0,3; q at 3; W 3; C(3) | `Completed(true)` |
| 15 | same | stamps 0,4; q at 4; W 4; C(4) | `Completed(false)` |
| 16 | `eventually[0,3] holds(p)` | stamp 0; p F; W 3; C(3); no later event | `Completed(false)` |
| 17 | `holds(p) until[0,1] holds(q)` and its `release` dual `on origin`; `holds(p) since[0,1] holds(q)` and its `triggered` dual `on each` (the anchor at position 2, where positions 1 and 2 participate) | stamps 0,1,1; no order keys; then a key on position 1 only | stage `evaluate`, `invalid_runtime_input`/`invalid-value` at position 2 each; `eventually[0,1] holds(p)` evaluates; the one-key case refuses at position 2 |
| 18 | row 17 `until` | distinct order keys, one authority; then two authorities; then trace listed in reverse, keys unchanged | evaluates; refuses; same result as the first |
| 19 | `once[0,4] holds(p)` `on each`, activations at positions 0 and 1 (future reach 0, established at W 4) | stamps 3,4; p F,F; W 4; C(4) | `HistoryCutoff`: `Indeterminate {MissingHistory}` with `where` 0 (the first activation in position order); `AuthoritativeOrigin`: `Completed(false)`; p T at 3: `Completed(true)` for both |
| 20 | `holds(req) implies eventually[0,3000] holds(ack)`, cover `Counter{u=0}` | req at 0; ack at 2998, 3000, 3001; W, C beyond 3001 | `Completed(true)`; `Indeterminate {StraddlesBound}`; `Completed(false)`; with `Exact` cover true, true, false |
| 21 | `holds(req) implies eventually[0,3000] holds(ack)`, `Counter{u=1}`, req at 0, ack at 2997, 2998, 3003; W and C beyond 3003 (`TCV-06` to `TCV-08`); and the comparison functions `within_bound` and `definitely_before` called directly on FSpec `tick-cover-vectors.json` `TCV-09` to `TCV-15` (the bounds 0, 1 and -1 of `TCV-13` to `TCV-15` and the `definitely_before` pairs are not clause intervals) | per the vectors | true, indeterminate (`StraddlesBound`), false for `TCV-06` to `TCV-08`; the file's expected value for each of `TCV-09` to `TCV-15` |
| 22 | `eventually[0,2] holds(p)` `on each` | stamps 0,10; p F,F; W 10; C(10); then p T at 0 | `Completed(false)` at 0; then `Pending {where 1, deadline 12}` |
| 23 | row 3's first clause, meter 6 then 7 | stamps 0,2,5; p F,T,F | `Incomplete` at `temporal.visit`; the verdict with 7 charges |
| 24 | `run_clause` over rows 1, 3 first case, 6, 8 | as rows | `violation`/exit 10 pos 0; `success`/0; `pending` with FR-285's pending-clause exit; `indeterminate`/11 cause `MissingValuation`; no `unsupported_profile` |
| 25 | the row 1 clause over `TraceInput::Finite`, and an event-position clause over `Timestamped` | none | stage `admit`, `invalid_runtime_input`/`invalid-value` each |
| 26 | rows 1 to 22 run twice | same inputs | equal outcomes and one result identity |
| 27 | row 20's clause, cover `Counter{u=1}`; then `Exact` | req at 0; no ack; W 3001, C(3002); W 3002, C(3002); then Exact W 3000, C(3000) | `Pending {deadline 3002}`; `Completed(false)`; `Completed(false)` |
| 28 | `always[0,5] holds(p)`, scope `Closed` then `Open`; `eventually[0,5] holds(p)` | stamps 0,2,5; p T,F,T; W 5; C(5); and p F,F,T | basis `closed-scope` / `decisive-counterexample`, verdict false both, support {(1,p)}; second clause basis `closed-scope`, true, support {(2,p)}; `Pending` basis `unsettled`; `Indeterminate` basis `unavailable` |
| 29 | row 28 with scope, surrounding execution, completeness and a work stop varied one at a time | as row 28 | only the named field (and the basis for scope, the verdict class for completeness as rows 12 and 13 state) changes; a work stop reports assessment `Failed` |
| 30 | row 1's request varied one input at a time: unit, cover, one stamp, one valuation, one order key, W, through, lower-boundary kind, a meter ceiling | each alone | a distinct result identity each time; the original identity is unchanged |
| 31 | `eventually[0,5] holds(p)`; then the same clause with an empty trace | stamps 0,2,5; p F,F,F; W 5, C(9); W 3, C(9) | `Completed(false)`; `Pending {deadline 5}`; stage `admit` `invalid_runtime_input`/`invalid-value` for the empty trace |
| 32 | row 1's first clause, observations listed as stamps 5,0,2 then 0,2,5 | p T at 0 and 5, F at 2 | equal outcome, support and `result_identity` for both listings (FR-501-AC-18) |
| 33 | row 32's listing 5,0,2 | p T at 0 and 5, F at 2 | `Completed(false)` at position 0; support {(1,p)} (FR-501-AC-19) |
| 34 | row 17's `until` over keys from one authority, listed as given and reversed | keys 1, 2 at stamp 1 | one outcome for both listings (FR-501-AC-20) |
| 35 | four observations at one stamp: keys A1, A2 (authority A), B1 (authority B), none; three listings | one clause, equal valuations | canonical order A1, A2, B1, keyless; one `result_identity` (FR-501-AC-21) |
| 36 | row 20's clause, the cover omitted from the trace input | req at 0; ack at 2998, 3000, 3001; W, C beyond 3001 | `Completed(true)`, `Completed(true)`, `Completed(false)` (FR-501-AC-22) |
| 37 | row 8's clause | stamps 0,2,2,5; p = F,T,T,F; W 5; C(5) | `Completed(true)` (FR-501-AC-23) |
| 38 | row 27's request with the supplied cover `Counter{u=1}`, W 3001, C(3002); then the same input with cover `Exact` | req at 0; no ack | `Pending {deadline 3002}`; `Completed(false)` and a different result identity (FR-501-AC-24) |
| 39 | row 1's request with `execution` `Open`, then `Closed` | as row 1 | the same verdict; the premises report `Open`, then `Closed` (FR-501-AC-25) |
| 40 | row 17's `until` and `release` `on origin`, and its `since` and `triggered` `on each` (the anchor at position 2); then `eventually[0,1] holds(p)` | stamps 0,1,1; positions 1 and 2 both keyed `(A, 1)`, distinct identities (a third atom differs); listed as given, then the two keyed observations reversed | stage `evaluate`, `invalid_runtime_input`/`invalid-value` at position 2 for each of the four operators and both listings; `eventually` evaluates with one outcome and one `result_identity` for both listings (FR-501-AC-26) |
| 41 | BothRoutesSupportAtomParity: row 28's clauses with the atom authored as `holds( p )` | row 28's timestamped inputs and equivalent FR-504 progressed inputs, with and without a valid ledger; the same caller-supplied sources on both routes | equal decision-support pairs on both routes, with the atom name exactly `holds( p )`, including its internal spaces and the whole `holds` atom; positions remain those of the corresponding trace |
| 42 | PackageMissingSourceRefusal: row 41's direct byte-only package request after caller provisioning | remove the caller-supplied source whose reference matches the atom region; then replace it with a source of another reference; then supply the matching reference with bytes shorter than the span or with a span endpoint inside a UTF-8 scalar | each case preserves FR-283's `SourceNotSupplied` contract: category refusal, `invalid_runtime_input`/`missing-member` naming `sources`, exit 20, with the region's source identity and no clause record; the run refusal stays at stage `call`, as on the source route, without using handoff source bytes, doing file intake during adapter execution or compiling source again |
| 43 | PackageCommandSupportAtomParity: row 41 through the command's package route | explicitly select the unit's exact original bytes in `request.sources`; use an authored file path different from the compile path and separately change only valid `document` and `formal_revision`; run timestamped and progressed inputs, with and without the ledger | non-empty support pairs equal the source command control and direct adapter; atom text is exactly `holds( p )`; native reference matching and text do not depend on the valid formal metadata or file path; explicitly selected files are admitted once before execution, no source compilation |
| 44 | PackageCommandMissingSourceRefusal: row 43 | omit `sources`, use `[]`, replace the native reference, shorten the valid admitted UTF-8 bytes, then move the region endpoint inside a UTF-8 scalar; retain different source bytes in the handoff | each gives row 42's stage `call` refusal and no clause record; the command's explicit source intake is permitted, while the handoff supplies no fallback text; an invalid-UTF-8 source file instead retains FR-001's source/S0 intake refusal, not `SourceNotSupplied` |

## Expected Results

- Outcomes are asserted as fields (variant, position, deadline, reason), never
  message text.
- Command-intake controls assert the outer `native-run-result/2.code`:
  `io-error` for a selected-file failure at stage `file` and
  `invalid-identifier` for a formal constructor failure at stage `request`,
  each exit 20. Native diagnostic catalog codes retain their underscore
  spellings; the constructor's existing `details.code` is `invalid_identifier`.
  FR-001 source/S0 and `SourceNotSupplied` call refusals are unchanged.
- No row returns a truth value for a case the table marks `Pending` or
  `Indeterminate`, and no row marked definite returns `Pending` or
  `Indeterminate`.
- The independent expected values come from the authored stamps and FSpec's
  vectors, never from the evaluator's own output.
