---
id: TC-2571
title: "An alias-only unit refuses at check naming the rule"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-910
    type: verifies
---
# TC-2571: An alias-only unit refuses at check naming the rule

## Description

An alias-only unit refuses at check naming the rule.

Scope: FR-910-AC-2.

## Test Procedure

Under the header `language "ix:native" edition "1-draft";` and `profile v = "filament.value.complete/v1";`, run S3 check and then compile through the spine over a unit of only `type I = Integer;`, a unit of `type I = Integer;` and `type J = Boolean;`, and a header-only unit.

Tag the tests `#[trace("TC-2571", "FR-910-AC-2", "FSpec-TC-1811", "FSpec-FR-746-AC-2")]`.

## Expected Results

- Each check refuses with code `missing_declaration` and cause `empty-package`.
- Each compile refuses with the same code and cause.
- No refusal has code `unsupported_projection`, and no message contains `no node of the package`.
