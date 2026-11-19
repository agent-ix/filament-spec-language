---
id: TC-1740
title: "Extension definitions supplied through the request compile and parse"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-660
    type: verifies
---
# TC-1740: Extension definitions supplied through the request compile and parse

## Description

Verify that the request `extensions` member supplies the catalog a unit selects an extension from, for compile, run and parse. Scope: FR-660-AC-1, FR-660-AC-6.

## Test Procedure

1. Write FSpec FR-467-AC-1's `retention` definition document and a `1-draft` unit selecting `profile r = "ext.retention";` and declaring `retention Audit { subject: "audit", days: 30 }`.
2. Compile it through `fsl_command::command::compile` with an `extensions` entry `{authority: agent-ix, identity: ext.retention, file}`.
3. Run the same unit through a `native-run/1` request with the same member.
4. Call the `parse` operation over the unit with and without a catalog holding the definition.

Tag the tests `#[trace("TC-1740", "<AC id>")]`.

## Expected Results

- Step 2: exit 0; one `extension` node of kind `retention` and a `definition_selections` row `{agent-ix, ext.retention}`.
- Step 3: the same package is compiled.
- Step 4: with the catalog an admissible source holding one `retention` form; without it the unselected-form refusal.
