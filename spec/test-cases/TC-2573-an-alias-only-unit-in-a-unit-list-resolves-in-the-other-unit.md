---
id: TC-2573
title: "An alias-only unit in a unit list resolves in the other unit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-910
    type: verifies
---
# TC-2573: An alias-only unit in a unit list resolves in the other unit

## Description

An alias-only unit in a unit list resolves in the other unit.

Scope: FR-910-AC-4.

## Test Procedure

Under the header `language "ix:native" edition "1-draft";` and `profile v = "filament.value.complete/v1";`, compile the unit list `type Port = Integer;` then `function f(p: Port): Port pure { p }`, then the same units in the other order, then the alias-only unit alone.

Tag the tests `#[trace("TC-2573", "FR-910-AC-4", "FSpec-TC-1812", "FSpec-FR-746-AC-4")]`.

## Expected Results

- Both orders compile to a package holding `f`.
- The alias-only unit alone refuses `missing_declaration`/`empty-package`.
