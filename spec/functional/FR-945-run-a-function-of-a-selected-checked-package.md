---
id: FR-945
title: "Run a function of a selected checked package from its bytes"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-014
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-945: Run a function of a selected checked package from its bytes

## Description

A `native-run/1` request selects an existing `filament.checked-package/v2`
artifact through its `package` member `{file}` (FR-100 Inputs) in place of a
`program`. The run SHALL admit the bytes with the strict checked-package
reader, build the `BarePackage` (FR-531) over the admitted package, resolve the
`call`'s function in it, read its parameter and result types from the
package, admit the arguments in FR-650's run value form and evaluate the call
through `BarePackage`'s `CheckedPackageEvaluation`. No source is compiled and no
`CheckedPackage` is built (FR-087 R-10, FR-940).

### Decision: run from the bytes through the bare package

FR-100 already gives native-run/1 the `package` member and requires execution
without fallback to source compilation; what it did not say is how a function
is found and called without a `CheckedPackage`. The answer is FR-940's: a
byte-read package is a `BarePackage`, and FR-531 already makes it an
evaluation entry. Adding the function call to that entry keeps one rule for
every consumer that holds bytes and no source, and it is the route by which a
function whose types FSL source cannot spell but a package can hold reaches
FR-650's "Types with no form" refusal.

## Inputs

- The request's `package` member `{file}`, resolved against the request's
  directory and counted toward `dependent_bytes`.
- The request's `models`, matched to the package's model selections as FR-850
  "Admission" matches them.
- The `call` object (FR-100, FR-650), and the call's `snapshot` when present.

## Outputs

FR-100's `native-run-result/2` outcome, equal to the outcome of the same call
over the source the package was compiled from, and FR-650's refusals.

## Behavior

- A request carrying both `package` and `program`, or `package` with
  `libraries`, `refinement_internals` or `integer_division`, SHALL refuse
  `invalid-request` at stage `request`, exit 20, naming the second member.
- Bytes the strict reader refuses SHALL refuse at stage `request`,
  `invalid_package` with the reader's cause and pointer, exit 20, before any
  call.
- The call's one-segment `function` SHALL resolve by the
  `declaration.qualified_name` of the package's function declarations (FSpec
  FR-322); a name that resolves to none refuses `missing_declaration` at stage
  `call`, exit 20, as FR-100 states for the source route.
- Each parameter and the result type SHALL be read from the package's checked
  type nodes and mapped through FR-650's one exhaustive mapping. A type the
  mapping gives no form (a population inside a collection, option, map or
  field) SHALL refuse `unknown_required_feature`/`unsupported-feature` at stage
  `call`, category unsupported, exit 21, before any argument is admitted and
  before any call, with `details` `{"position": P}` for the first such
  parameter and `{"function": "<function>"}` for the result; a parameter is
  checked before the result.
- Arguments SHALL be admitted and the call evaluated and charged as on the
  source route; the outcome, the accounting record and every refusal SHALL
  equal the source route's for the same package, except for the request's
  selection member.
- FR-531's `CheckedPackageEvaluation` for `BarePackage` SHALL provide the
  function-call entry this requires, and FR-531's capability inventory lists
  `call`, `call_supplied` and `call_observing_applications` as available
  through this requirement, so FR-531-AC-9's one-row-per-method check holds.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-945-AC-1 | The spine unit compiled to its v2 bytes and selected as `package`, with the same `models` and `call` of FR-650-AC-5's `same` over the healthy current snapshot, completes with the value and exit code of the source request; the same request with `program` also present refuses `invalid-request` naming `program`, and with a `package` file holding `{}` refuses `invalid_package` at stage `request`, exit 20. | Test (TC-2625) |
| FR-945-AC-2 | FR-650-AC-6's fixture `tests/fixtures/population-collection.checked-package.json` selected as `package` refuses `unknown_required_feature`/`unsupported-feature` at stage `call`, exit 21, before any call, `details` `{"function": "gives"}` for `gives` and `{"position": 0}` for `takes`; `call.function` `missing` refuses `missing_declaration` at stage `call`. | Test (TC-2625) |

## Dependencies

- FR-100 owns the `package` member; FR-650 owns the run value form and its
  type mapping; FR-531 and FR-940 own the byte-read package and its read view.
