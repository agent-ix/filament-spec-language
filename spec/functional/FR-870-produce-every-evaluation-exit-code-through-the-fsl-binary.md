---
id: FR-870
title: "Produce every evaluation exit code through the fsl binary"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-301
    type: references
---
# FR-870: Produce every evaluation exit code through the fsl binary

## Description

FSpec FR-301 gives each outcome category one exit code, and FR-285 maps the
library's `Category` to it. A code is only useful if a user can reach it, so
each evaluation code SHALL be produced by the `fsl` binary from an ordinary
request and asserted end to end, from the process exit status:

- exit 10 for a violated state invariant (FR-108's violating-parent case);
- exit 10 for a value run whose value is undefined (`x mod 0`,
  `undefined`/`division-by-zero`, FR-100);
- exit 11 for a timestamped-event finite-window clause whose window closes
  with its verdict unresolved: FR-641-AC-1's three samples stamped 0, 2 and
  5, watermark 5, `complete` through 5, under a `clocks` member configuring
  the clause's clock as `timestamped_event` (FR-640), with `p` false at 0
  and 5 and its valuation absent at stamp 2, which FR-501-AC-4 settles
  `Indeterminate { MissingValuation }`;
- exit 20 for a refusal and exit 22 for an exhausted budget, which never
  report 10 or 11.

The exit-11 case needs the clock configuration a request now carries
(FR-640) and the timestamped trace member (FR-641); no library test seam
stands in for the binary. The filament-examples suite carries one case per
code, so the examples gate (filament-driver FR-021) asserts them too.

## Inputs

A run request for each case.

## Outputs

The process exit status and the outcome document's category.

## Behavior

- When `fsl run` runs a violated state invariant, it SHALL exit 10 with
  category violation.
- When `fsl run` runs a call whose value is undefined, it SHALL exit 10 with
  category undefined and the reason.
- When `fsl run` runs a finite-window clause that settles `Indeterminate`,
  it SHALL exit 11 with category indeterminate.
- The binary's exit status SHALL equal FR-285's code of the document's
  category for every case.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-870-AC-1 | `fsl run` over FR-312-AC-1's violating-parent clause request exits 10, and its document's category is violation. | Driver test (TC-2463): only the binary has a process exit status, so the code ticket adds the test in the filament-driver repository (`fsl-cli/tests/binary_process.rs`). |
| FR-870-AC-2 | `fsl run` over a call request for `function m using v(): Integer pure { 7 mod 0 }` exits 10 with result `{"kind": "undefined", "reason": "division-by-zero"}`. | Driver test (TC-2463) |
| FR-870-AC-3 | `fsl run` over the timestamped finite-window request above exits 11 with category indeterminate and an item or report naming `MissingValuation`; the same request with `p` true at stamp 5 exits 0. | Driver test (TC-2463) |
| FR-870-AC-4 | `fsl run` over a request naming an unknown function exits 20, and over FR-100-AC-6's `seven` with `accounting.work_units` 0 exits 22; neither exits 10 or 11. | Driver test (TC-2463) |
| FR-870-AC-5 | The filament-examples suite holds a case expecting exit 11 (AC-3's request), and the examples runner fails when that case exits otherwise. | Inspection (TC-2463): the case lives in the filament-examples repository. |

## Dependencies

- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit
  codes.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the binary.
- [FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md),
  [FR-640](FR-640-select-clock-configurations-in-compile-and-run-requests.md)
  and [FR-641](FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md):
  the indeterminate case.
- FSpec FR-301: the exit table.
