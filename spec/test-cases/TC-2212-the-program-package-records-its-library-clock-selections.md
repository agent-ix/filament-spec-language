---
id: TC-2212
title: "The program package records its library clock selections"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
  - target: ix://agent-ix/filament-specification/TC-1661
    type: references
---
# TC-2212: The program package records its library clock selections

## Description

Verify that a compile writes the configuration it served to a library
unit's own clock into the program package's lock
`dependency_clock_selections`, that the configuration enters the package
bytes, and that the member is absent when no library takes a request clock.
Scope: FR-640-AC-10.

## Test Procedure

1. Through `command::compile`, compile the program importing `test/plant`,
   whose clause `Late` is on `lib-ms`, with `clocks` configuring `lib-ms` as
   `timestamped_event` in `ms`; read the package lock. Compile again with
   `timestamp_unit` `s`.
2. Compile the program against a `test/plant` with no clause and no `clocks`.

Test: `tests/it/clock_derivation.rs`
`the_program_package_records_its_library_clock_selections`, tagged
`#[trace("TC-2212", "FR-640-AC-10", "FSpec-TC-1661", "FSpec-FR-322-AC-452")]`.

## Expected Results

- Step 1: exit 0; the lock holds the one `lib-ms` entry with the `ms`
  configuration; the `s` compile gives different package bytes.
- Step 2: exit 0 and no `dependency_clock_selections` member.
