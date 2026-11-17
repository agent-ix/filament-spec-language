---
id: FR-975
title: "Write the finite-window evidence ledger as one closed file form keyed by the obligation instance"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-966
    type: references
---
# FR-975: Write the finite-window evidence ledger as one closed file form keyed by the obligation instance

## Description

A correction chain reaches `command::run` through a file: a run writes its
updated `EvidenceLedger` (FR-524, FR-526) as the result's `evidence_ledger`
member, and a later run request names that file in its temporal selection's
`evidence_ledger` member (FR-526 "On FR-641's run request"). This
requirement fixes that file form, `filament.finite-window-evidence-ledger/1`,
so the file a run writes is exactly what a later run reads, and so a
correction is admitted against the same obligation instance in a run as in
an in-process call.

### Decision: key the file by the whole obligation instance

FR-526 looks a predecessor up by the obligation instance: the package
identity, the clause, the clause occurrence and the trigger identity. A file
keyed by the clause name alone would let a result of one package admit a
correction in another package that declares a clause of the same name, and
would let a correction move to another occurrence or another trigger
instance of the same clause, which FR-526 refuses in process. The file
therefore carries the full key, spelled as FR-641's progress ledger spells
its binding, so the two caller-held ledgers of a run name an obligation the
same way.

### Decision: retain the superseded identities of the chain

A correction naming a result the ledger does not hold and a correction
naming an earlier result of the chain are different mistakes: the first
names a result of another chain, another obligation or no result at all, and
the second names a result a later correction already replaced. The entry
therefore lists the `result_identity` of every earlier result of its chain,
so the evaluator tells the two apart (FR-526 "Predecessor lookup"). The list
is flat, holds identities only and no result bytes, and its length equals
the entry's `lineage`, which FR-523's `lineage` counter bounds.

## Inputs

The `evidence_ledger` member of a run request's temporal selection: the
relative path of the file, read once under FR-100's `dependent_bytes` limit
and relative to the request file.

## Outputs

The result's `evidence_ledger` member, in the file form below. Its bytes are
what a later request's `evidence_ledger` file holds.

### The file form

The file is a closed JSON object:

```json
{"format": "filament.finite-window-evidence-ledger/1",
 "results": [
   {"key": {"package": "<package identity>",
            "clause": "<declaration node key>",
            "occurrence": {"role": "<role>", "ordinal": 0},
            "trigger": null},
    "result_identity": "<FR-522 result identity>",
    "input_identity": "<FR-522 input identity>",
    "correspondence": null,
    "lineage": 1,
    "superseded": ["<FR-522 result identity>"]}
 ]}
```

- `format`: exactly `filament.finite-window-evidence-ledger/1`.
- `results`: an array of closed entries, one per obligation instance, in
  ascending key order (package, then clause, then occurrence role and
  ordinal, then trigger with `null` first), with no key repeated. An empty
  array is a ledger that holds no result.
- `key`: the obligation instance (FR-526 "Semantic authority and
  boundary"). `package` is the checked package identity; `clause` is the
  declaration node key's canonical text, as FR-641's `binding.clause`;
  `occurrence` is the selected `{ role, ordinal }` (FR-516 `occurrence`);
  `trigger` is the trigger identity the request's temporal selection
  assesses (FR-524 `trigger`) in the run request's spelling of that
  identity, or `null` when the request assesses no trigger instance (an
  `on origin` clause).
- `result_identity` and `input_identity`: the latest result's FR-522
  identities, in the spelling the run result writes them.
- `correspondence`: the correspondence the latest result echoes (FR-516),
  byte for byte as a JSON value, or `null` when it echoes none.
- `lineage`: a non-negative integer no greater than
  9007199254740991, the latest result's lineage.
- `superseded`: the `result_identity` of each earlier result of the chain,
  oldest first; its length equals `lineage`, and it is empty for an
  `Original` result.

## Behavior

- The run SHALL read the `evidence_ledger` file before evaluation.
- The run SHALL look up the request's predecessor by the whole key (package,
  clause, occurrence and trigger), never by the clause alone.
- The run SHALL apply FR-526's admission rules to the entry exactly as to the in-process `EvidenceLedger`,
  so two runs whose second request names the file the first wrote give the
  same outcome as the two in-process calls sharing one ledger.
- A missing or unreadable `evidence_ledger` file SHALL refuse at stage
  `request` naming the path, as a missing FR-641 `ledger` file does.
- If the file is not a closed `filament.finite-window-evidence-ledger/1`
  document (not JSON, a missing or unknown member at any level, a `format`
  other than `filament.finite-window-evidence-ledger/1`, a member of the
  wrong type), or its entries are out of key order or repeat a key, or an
  entry's `superseded` length differs from its `lineage`, then the run SHALL
  refuse `invalid-request` at stage `request`, naming `evidence_ledger`, with
  no result document and no ledger.
- When a call is accepted, the run SHALL write the updated ledger: the
  request's entries unchanged except the entry for the result's key, which
  holds the result's identities, correspondence, lineage and `superseded`
  list (FR-526: the predecessor entry's list followed by the predecessor's
  `result_identity` for a correction, empty for an `Original` result, whose
  entry replaces any entry the key held).
- When the relation is refused, the run SHALL write no ledger (FR-966 "A
  refused correction through a run").
- When the lineage counter stops the call, the run SHALL write the request's
  ledger unchanged (FR-526 "Lineage limit").
- The writer SHALL write the ledger canonically (members in the order above,
  entries in key order), so writing a read ledger reproduces its bytes.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-975-AC-1 | An `Original` run of FR-526-AC-31's request writes an `evidence_ledger` member equal to `{"format": "filament.finite-window-evidence-ledger/1", "results": [E]}` with `E.key` holding the run's package identity, the `Seen` declaration node key, its occurrence and `trigger` `null`, `E.result_identity` and `E.input_identity` equal to the result's, `E.lineage` 0 and `E.superseded` `[]`; the repair run of FR-526-AC-31 over that file writes an entry for the same key with `lineage` 1 and `superseded` `[r1.result_identity]`; reading a written ledger and writing it again reproduces its bytes. | Test (TC-2750) |
| FR-975-AC-2 | Two packages that each declare a clause `Seen`, run in turn with one ledger file carried between them, give a ledger holding two entries that differ in `key.package`; a `Superseding(r1)` run of the second package naming the first package's `r1` with that ledger refuses at stage `evaluate`, `invalid_runtime_input`/`invalid-value`, member `relation`, `cause` `predecessor-unknown`, and writes no result document and no ledger. | Test (TC-2750) |
| FR-975-AC-3 | With the ledger written by `r1` and by `r2` superseding `r1`, a `Superseding(r1)` run refuses `predecessor-not-latest` (member `relation`, stage `evaluate`), a `Superseding` naming an identity that neither the entry nor its `superseded` list holds refuses `predecessor-unknown`, and a `Superseding(r2)` run with changed inputs is accepted with `lineage` 2. | Test (TC-2750) |
| FR-975-AC-4 | An `evidence_ledger` file with an unknown entry member `extra`, one whose `format` is `filament.finite-window-evidence-ledger/2`, one whose entry has no `key`, one whose two entries repeat a key, one whose entries are out of key order, and one whose entry has `lineage` 1 and an empty `superseded` list each refuse `invalid-request` at stage `request` naming `evidence_ledger`, with no result document and no ledger; a path that does not exist refuses at stage `request` naming the path. | Test (TC-2751) |

## Dependencies

- FR-526: the correction relation, its admission rules and the
  predecessor lookup this file carries to a run.
- FR-524: the `EvidenceLedger` value; FR-522: the identities.
- FR-641: the run request, its `dependent_bytes` read and the progress
  ledger whose binding spelling the key follows.
- FR-966: the refusal of a correction through a run.
