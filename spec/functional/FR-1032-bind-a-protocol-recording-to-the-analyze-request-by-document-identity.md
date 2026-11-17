---
id: FR-1032
title: "Bind a protocol recording to the analyze request by document identity"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-specification/FR-821
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-624
    type: references
---
# FR-1032: Bind a protocol recording to the analyze request by document identity

## Description

FSL's `native-analyze/1` reader (FR-620) SHALL provision a supplied FSpec
FR-737 protocol recording under one `{authority, identity}` document
identity, as FSpec FR-821 states: the identity the recording declares in its
own `identity` member, or the identity the `documents` entry states as
`{file, identity}`. A `global-conformance` item names the recording by that
identity in its `recording` member, which selects replay mode (FR-895,
FR-896), and the item's result carries the identity. The binding lives in
the analyze request because `global-conformance` is an analyze claim kind;
the native run request (FR-100, FR-312) selects snapshots by key and carries
no recording.

## Behavior

- A `documents` entry SHALL be `{file}` or `{file, identity}`, closed. The
  reader SHALL compute each document's provisioned identity (the declared
  identity, else the stated one) before compiling the program, and SHALL
  apply FSpec FR-821's refusals at stage `request`, code `invalid-request`,
  exit 20, at its pointers, as FR-624's first row.
- The item's FR-331 request identity SHALL cover the provisioned identity
  and the recording bytes. The item's result SHALL carry `recording`, the
  provisioned identity, beside FR-990's `obligations`, `closure` and
  `global`.
- The provisioned identity SHALL NOT enter any FR-990 closure-premise
  identity `{kind, instance, node[, role_instance]}`, and FSL SHALL compare
  premise sets without it. A recording bound under two identities yields
  equal premise sets and equal verdicts.
- A recording that is provisioned but that FR-895's admission refuses (an
  undeclared role, an unknown node, a contradictory key) SHALL refuse that
  item `invalid_runtime_input`, not the request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1032-AC-1 | The FR-895-AC-7 recording with no `identity` member, supplied as `{"file": "recording.json", "identity": {"authority": "test", "identity": "recording"}}` and named by the `Flow` item's `recording`, settles `proved` with every obligation `satisfied`, and the item carries `recording` `{"authority": "test", "identity": "recording"}`; the same recording declaring that identity and supplied as `{file}` gives equal `obligations`, `closure` and `global`. | Test (TC-2922) |
| FR-1032-AC-2 | The same recording bound as `{authority: "test", identity: "other"}` gives equal `closure.premises` and `obligations`. | Test (TC-2922) |
| FR-1032-AC-3 | A stated identity `{"authority": "test"}` refuses `invalid-request` at `/documents/<i>/identity`; a stated identity differing from the declared one refuses `invalid-request` at that pointer naming both; an unidentified recording supplied as `{file}` refuses at `/documents/<i>`; a recording stated under a supplied snapshot's identity refuses at the later entry; each exits 20 with `items` empty. | Test (TC-2922) |
| FR-1032-AC-4 | An item `recording` naming no provisioned document, one naming a snapshot, and `recording` on a `temporal-satisfaction` item each refuse `invalid-request` at `/items/0/recording`, exit 20. | Test (TC-2922) |

## Dependencies

- FSpec FR-821 (the binding), FSpec FR-737 (the recording), FSpec FR-520
  (the request), FR-620 (the reader), FR-624 (refusal stages), FR-895 and
  FR-896 (replay mode), FR-990 (premise identity).
