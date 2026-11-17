---
id: TC-2811
title: "A ledger file carries each record sample's fingerprint through the command"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
---
# TC-2811: A ledger file carries each record sample's fingerprint through the command

## Description

Verify FR-641's `fingerprint` member of a covered ledger entry: written for
record samples only, read back byte for byte, checked across `command::run`
requests as in process, and refused when missing or misplaced.
Scope: FR-641-AC-10.

## Test Procedure

1. Run a `progressed` record-sample request over levels 1, 2 with `Complete { through: 1 }` through `command::run` and read the ledger file it writes.
2. Run requests carrying that ledger file over levels 1, 5, 3 and over levels 1, 2, 3; run the same requests in process.
3. Read the ledger file and write it again.
4. Run a request whose ledger lacks a record-sample entry's `fingerprint`, and one whose snapshot-position entry carries one.

## Expected Results

- Step 1: each covered entry holds `identity` (the index), `stamp` and `fingerprint` `sha256:` plus 64 lowercase hexadecimal digits; no sample text or value appears.
- Step 2: levels 1, 5, 3 refuse `Completeness`; levels 1, 2, 3 are accepted; the outcomes equal the in-process ones.
- Step 3: the bytes are reproduced.
- Step 4: each refuses `invalid-request` at stage `request` naming `ledger`.

Test: `fsl-command` ledger-file tests (the code ticket adds them).
