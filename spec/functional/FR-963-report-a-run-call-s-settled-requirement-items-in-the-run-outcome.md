---
id: FR-963
title: "Report a run call's settled requirement items in the run outcome"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-961
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-775
    type: depends_on
---
# FR-963: Report a run call's settled requirement items in the run outcome

## Description

A run whose call carries `requirements` (FR-960) SHALL report each settled
requirement item beside the call's outcome, in the three forms a run
returns: the `spine-run-result/1` result document (FR-100), the typed
`fsl_command::command::RunResult`, and the FR-286 `execute` outcome document.
A run without `requirements` reports none and its three forms are FR-100's
and FR-286's unchanged.

Each item's verdict is the provider's or the linked engine's, settled by
FSL's one settlement path (FR-281, FR-290), so it is reported in exactly the
spelling an analyze item uses (FR-621 "Item members"). The driver's process
provider results therefore enter the same settlement path as first-party
results and reach the outcome as typed terminal records with their
diagnostics, never as untyped provider data (driver FR-006, FR-014). One
item shape across `analyze` and `run` lets a reader, and the driver, read a
run's items with the reader it already has.

## Outputs

### `spine-run-result/1`

Two members, present exactly when the call carried `requirements`:

- `requirements`: one entry per item of the call, in FR-395's package order.
  Each entry carries FR-621's item members for its item: `request_index`
  (FR-395's request index), `claim_name` (the called function's qualified
  name, the items' owner), `request` (the admitted FSpec FR-331 request item,
  with its `claim` `{node, occurrence}`, kind, extent, named `backend` when it
  names one, `options` and `candidates`), `category`, `record` (with `backend`
  exactly when `settled_by` is `result`, FR-621 "The record's `backend`
  member"), `certificate`, `counterexample` and `replay` where FR-621 gives
  them, and `accounting`.
- `registry`: FR-635's sorted `{backend, tool}` list of every backend the
  items were negotiated against.

`format` stays `spine-run-result/1`: the members are optional additions
whose absence means no settlement was asked for, and a reader that predates
them reads every run without `requirements` unchanged.

### `RunResult`

A new public field `requirements: Option<RequirementResults>`, `None` exactly
when the call carried no `requirements`. `RequirementResults` holds
`records: Vec<TerminalRecord>`, one per item in package order;
`counterexamples: Vec<ItemCounterexample>`, exactly one reproduced
counterexample per `refuted` record; and `registry`, the list the document's
`registry` member writes. `RunResult.value` is the `spine-run-result/1`
document above and `RunResult.outcome` the `execute` document below.

### FR-286 `execute` outcome

The document's `items` member SHALL carry one entry per settled item, with
the members of the `requirements` entry above, and the document SHALL carry
FR-635's `registry` member. Its `result` stays the call's result. A run
without `requirements` has `items` `[]` and no `registry`.

## Behavior

- The category of a run that settles items SHALL be the most severe of the
  call's category and every item's category under FR-285's multi-item order,
  and `RunResult.exit_code` and the document's `category` SHALL follow it;
  with no `requirements`, the call's category alone.
- An item settled by a disposition SHALL carry its FR-272 diagnostic in its
  `record` and no `cause` (FR-286).
- The item entries of the three forms SHALL be equal member for member.
- An FR-651 `items` run reports each `call` selection item's settlements in
  that item's `result`, which is a `spine-run-result/1` document.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-963-AC-1 | FR-960-AC-1's `sq` run with `requirements` `{}` returns a `spine-run-result/1` document holding `outcome` `{"kind": "completed", "value": {"integer": "16"}}`, `requirements` of three entries with `request_index`, `claim_name` `sq`, `request`, `category` `success`, `record` `proved` naming `backend` `value-checker` and `accounting`, and `registry` listing every `Engines::fsl()` backend; `RunResult.requirements` holds three `proved` records and no counterexample; the `execute` document's `items` equal the document's `requirements` entries and its `result` is the completed value. | Test (TC-2703) |
| FR-963-AC-2 | FR-961-AC-1's run reports each `Even` item `proved` naming `fsl.test-recording-provider` with certification `trusted`, and its `registry` lists the provider beside the linked backends. | Test (TC-2703) |
| FR-963-AC-3 | FR-960-AC-2's run naming `fsl.absent` has category `refusal`, exit 20, a completed `outcome`, and each item record settled by its disposition with the `unknown-backend` diagnostic, no `cause` and no `backend`. | Test (TC-2703) |
| FR-963-AC-4 | A run without `requirements` returns a `spine-run-result/1` document with no `requirements` and no `registry` member, `RunResult.requirements` `None`, and an `execute` document with `items` `[]`, byte-equal to the documents FR-100-AC-1 states for `seven`. | Test (TC-2703) |

## Dependencies

- FR-100, FR-286: the documents extended. FR-621: the item members reused.
- FR-635: `registry`. FR-285: the category order. FR-961: the settlement.
- FSpec FR-775: the standard's `execute` outcome of settled items.
