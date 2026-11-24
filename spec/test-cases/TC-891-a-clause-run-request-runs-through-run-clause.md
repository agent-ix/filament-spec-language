---
id: TC-891
title: "A clause-run request file runs through run_clause and writes a /2 document"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: verifies
---
# TC-891: A clause-run request file runs through run_clause and writes a /2 document

## Description

Verify FR-312: `read_clause_run_request` reads a `1-draft` request carrying
`clause`, the `run` command and the driver's run entry run it through
`run_clause` and write a `native-run-result/2` document, and the reader
refuses malformed requests and requests past the intake limit.

Scope: FR-312-AC-1 to AC-6 and AC-12 to AC-17, AC-19 to AC-20.

## Test Procedure

1. From FR-108's fixtures, write clause-run requests for healthy-parent and
   violating-parent (`ParentOrder`, the case's current snapshot), each with
   `self: child` and the snapshot's `anchor`. Call
   `command`'s `run` in process over each.
2. Edit the healthy-parent request, one edit per run: add a `call`; remove
   `clause`; add `function` beside `clause` in `selection`; add a member
   `note` to `clause`; point the selection at observation key `ghost`.
3. Set `dependent_bytes` one below the total size of the request's dependent files, then to that
   total.
4. Write a `function` request for `sameIdentity` over the
   distinct-identities snapshot with arguments `{b: child, a: root}`, and a
   `frame` request; build the same `ClauseRunRequest`s in process and call
   `run_clause`.
5. Build a current clause request with `self` and `anchor`, a pre clause
   request with `self` and `parameters: {}`, and an invocation clause request
   with neither context member. Compare each to the same FR-106 selection
   built in process. For current, change the request anchor to one that
   disagrees with the snapshot. In independent edits omit `anchor`
   from current, omit `parameters` from pre, add `parameters` to
   current, add `anchor` to pre, and add `self`, `anchor` or `parameters` to
   invocation. Select a post snapshot directly, then a document with an
   unrecognized `format`, then a snapshot with an unrecognized `observation`
   role. Omit `self` from a current request whose receiver is an object and
   expect an admission refusal `wrong-observation` at stage `admit`, exit 20.
6. Read a clause request with all ten `accounting` counters set to 1 through
   10, one with `accounting` omitted, and one with a legacy direct
   `work_units` member in `clause`.
7. Read a request whose `selection` is `context`; read items requests holding
   `value_invariants` and a population selector; put each of the latter two
   in `clause` and record its request-stage refusal.
8. On the healthy clause request, use a duplicate member and positional
   record array. Separately include two of `call`, `clause` and `items`, then
   none of them. Try an empty selection, a selection with two variants and a
   variant with an extra member.
9. Build extraction requests with `quire-extraction` enabled and disabled.
   Check a missing required selection member and a role-forbidden member.
10. Instrument observation reads and `run_clause`: run one valid request
    whose referenced files are in a sibling directory relative to the request;
    record read count and verify the admitted request carries bytes but no path.
    Also send an evaluation result and a refusal through the command entry.

Tag the tests `#[trace("TC-891", "FR-312-AC-n")]`.

## Expected Results

- Step 1: one `/2` document each: `success`, `truth: true`, exit 0;
  `violation`, `truth: false`, exit 10.
- Step 2: each refuses `invalid-request` at stage `request`, exit 20, no result document, naming the member or key.
- Step 3: the intake-limit refusal, then a run.
- Step 4: each document's disposition equals the in-process report.
- Step 5: each valid request gives the same report as its in-process
  selection, using the supplied current or pre `self`, the supplied current
  `anchor`, and the invocation document's own `self`. A mismatched current
  anchor gives FR-106's `wrong_snapshot`/`wrong-anchor` admission report.
  Every missing or forbidden context member refuses `invalid-request` at
  stage `request`, naming that member; a selected post snapshot, unknown
  format or unknown observation role names `observation`. Each request
  refusal returns no result document, exit 20, before `run_clause`.
- Step 6: all ten counter values reach `ClauseRunRequest.accounting`
  unchanged; omission uses FR-100's defaults; the legacy `clause.work_units`
  request refuses `invalid-request` at stage `request`, exit 20, no result document.
- Step 7: context is accepted, the items requests are admitted, and either
  items-only selection in `clause` refuses naming the member.
- Steps 8-9: each malformed or exclusive-form request refuses at request
  stage with the offending member or selection identified; extraction is
  accepted only with the feature enabled.
- Step 10: each observation is read once relative to the request file, bytes
  reach `run_clause` without a path, and each admitted request invokes it once;
  results use the report and a reader refusal uses the command-error envelope.
