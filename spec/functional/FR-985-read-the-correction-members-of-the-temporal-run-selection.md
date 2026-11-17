---
id: FR-985
title: "Read the correction members of the temporal run selection"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-975
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-966
    type: references
---
# FR-985: Read the correction members of the temporal run selection

## Description

The `temporal` selection of a clause-run request SHALL be the closed object
`{name, trace, over, ledger, relation, evidence_ledger, admit_late}`, and the
run request reader SHALL decode `relation`, `evidence_ledger` and
`admit_late` with the shapes and refusals below.

## Rationale

A correction chain of finite-window results (FR-526) reaches `command::run`
through three members of the temporal selection: the relation of this run's
result to an earlier one, the evidence ledger file an earlier run wrote
(FR-975), and the late observations a superseding repair admits. The member
set FR-641 and FR-312 stated, `{name, trace, over, ledger}`, left them out,
so a reader following those requirements would refuse every correction
request as an unknown member, and FR-975's file could never be read back.
The three members exist only for a `timestamped` trace, because only a
finite-window result has a correction relation.

## Inputs

The `temporal` member of FR-312's `selection`.

## Outputs

FR-526's relation, evidence ledger and admitted late observations on the
`ClauseRunRequest`, or a request refusal.

## Behavior

### Members

| Member | Shape | Required | Meaning |
|---|---|---|---|
| `name` | string | yes | the temporal clause (FR-312) |
| `trace` | one FR-641 trace kind | yes | the trace (FR-641) |
| `over` | FR-330's reference | exactly when positions are snapshot keys | the `over` binding (FR-641) |
| `ledger` | relative path string | no | the progress ledger file (FR-641) |
| `relation` | `"original"`, `{"superseding": ID}` or `{"invalidating": ID}`, `ID` a string | no, `"original"` when absent | the relation of this result to the result whose `result_identity` is `ID` (FR-526) |
| `evidence_ledger` | relative path string | no | the `filament.finite-window-evidence-ledger/1` file (FR-975), read once under FR-100's `dependent_bytes` limit, relative to the request file |
| `admit_late` | array of strings | no, empty when absent | the late observations a superseding repair admits, each by observation identity (FR-526) |

### Request-stage refusals

Each refuses `invalid-request` at stage `request`, exit 20, with no result
document and no ledger, naming the member:

- an unknown member of the selection, a duplicate member, or a `null` value
  for any member (FR-641's closed-envelope rules);
- `relation` of another JSON shape: a string other than `"original"`, an
  object with other than one member, a member other than `superseding` or
  `invalidating`, or a non-string `ID`;
- `evidence_ledger` that is not a string; `admit_late` that is not an array
  of strings;
- `relation`, `evidence_ledger` or `admit_late` on a trace other than
  `timestamped`, naming that member;
- an `evidence_ledger` file that is missing, unreadable, or not a closed
  FR-975 document, as FR-975 states.

### Evaluate-stage refusals

These need the evaluated result and are FR-526's, refused through a run as
FR-966 "A refused correction through a run" states (stage `evaluate`,
`invalid_runtime_input`/`invalid-value`, its `cause`, no result document,
no ledger): a `superseding` or `invalidating` relation with no
`evidence_ledger` (`no-ledger`, member `relation`), an `ID` the ledger entry
does not hold (`predecessor-unknown`), an `ID` it holds as superseded
(`predecessor-not-latest`), an unchanged input (`unchanged-input`), and
`admit_late` on an `original` or `invalidating` relation
(`admission-without-repair`, member `admit_late`) or naming no late
observation (`admitted-not-late`, member `admit_late`).

### Obligation key

The run request carries no trigger delivery and no execution origin, so a
run's FR-975 entry key has `trigger` `null` (FR-516: a request with no
delivery and no origin has no `trigger`).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-985-AC-1 | Through `command::run`, a `timestamped` request whose temporal selection holds all seven members, with `relation` `{"superseding": r1.result_identity}`, the `evidence_ledger` file `r1`'s run wrote and `admit_late` naming the late observation of FR-526-AC-31, is read with no request refusal and reports FR-526-AC-31's repair; the same request with `relation` absent is read as `"original"`. | Test (TC-2781) |
| FR-985-AC-2 | A temporal selection with an extra member `trigger`, with `relation` `"superseded"`, with `relation` `{"superseding": 1}`, with `relation` `{"superseding": "x", "invalidating": "y"}`, with `evidence_ledger` `null`, and with `admit_late` `"o1"` each refuses `invalid-request` at stage `request`, exit 20, naming the member, with no result document. | Test (TC-2781) |
| FR-985-AC-3 | On a `finite` trace, a selection carrying `relation` `"original"`, one carrying `evidence_ledger`, and one carrying `admit_late` `[]` each refuse `invalid-request` at stage `request`, exit 20, naming that member. | Test (TC-2782) |
| FR-985-AC-4 | A `timestamped` request with `relation` `{"superseding": "r"}` and no `evidence_ledger` refuses at stage `evaluate`, `invalid_runtime_input`/`invalid-value`, member `relation`, `cause` `no-ledger`, with no result document and no ledger. | Test (TC-2782) |

## Dependencies

- FR-641 and FR-312 own the temporal selection and its other members;
  FR-526 owns the relation and its admission; FR-975 owns the ledger file;
  FR-966 owns the refusal of a correction through a run.
