---
id: TC-2987
title: "Package route names a model by its package identity and refuses a source alias"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-2987: Package route names a model by its package identity and refuses a source alias

## Description

Verify that a `package` analyze request names a model by the package
identity its model selection records, settles a derived item so named with
the source route's record, and settles an item naming the model by its
source alias `invalid-request` with the typed cause `model-alias` naming the
identity to use. Scope: FR-850-AC-9.

## Test Procedure

1. Compile the Counter unit importing its model as `M` with a `handoff`
   member, and analyze `{"claim": "Bounded"}` and the deadlock-freedom item
   with `claim` `test/counter` over source and over the handoff directory.
2. Repeat with the deadlock-freedom item's `claim` `M` on both routes.
3. Compile the same unit importing its model as `N` and compare the two
   `checked-package.json` files.

## Expected Results

- Step 1: both items settle with equal records on both routes.
- Step 2: the source route settles the `M` item as in step 1; the package
  route settles `Bounded` as in step 1 and the `M` item `invalid-request`
  `invalid_capability`/`model-alias`, naming `M` and `test/counter`, with
  no `not-yet-implemented` diagnostic and no `unsupported` item.
- Step 3: the two files are byte-equal.
