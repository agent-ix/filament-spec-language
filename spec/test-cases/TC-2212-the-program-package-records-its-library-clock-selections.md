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
Scope: FR-640-AC-10, FR-640-AC-16, FR-640-AC-17.

## Test Procedure

1. Through `command::compile`, compile the program importing `test/plant`,
   whose clause `Late` is on `lib-ms`, with `clocks` configuring `lib-ms` as
   `timestamped_event` in `ms`; read the package lock. Compile again with
   `timestamp_unit` `s`.
2. Compile the program against a `test/plant` with no clause and no `clocks`.
3. Emit one package with a bounded event-position clause through
   `emit_dependency_with_temporal_selections_and_cancel` twice, under two
   sequence authorities, then call `emit_checked` on it.
4. Emit a library with a bounded event-position clause under one sequence
   authority, link an importer, emit the library again under a second, and
   emit the importer. Link one package over two importers of the library that
   linked it under different authorities.

Test: `tests/it/clock_derivation.rs`
`the_program_package_records_its_library_clock_selections`, tagged
`#[trace("TC-2212", "FR-640-AC-10", "FSpec-TC-1661", "FSpec-FR-322-AC-452")]`.
Step 3: `fsl-package/tests/temporal_occurrences.rs`
`a_later_dependency_emission_replaces_the_remembered_one`, tagged
`#[trace("TC-2212", "FR-640-AC-16")]`.
Step 4: the same file,
`an_importer_keeps_the_library_emission_it_linked`, tagged
`#[trace("TC-2212", "FR-640-AC-17")]`.

## Expected Results

- Step 1: exit 0; the lock holds the one `lib-ms` entry with the `ms`
  configuration; the `s` compile gives different package bytes.
- Step 2: exit 0 and no `dependency_clock_selections` member.
- Step 3: the two emissions differ, and `emit_checked` returns the second,
  whose `temporal_bindings` clock holds the second authority.
- Step 4: the importer's lock `dependency_clock_selections` and the library
  emission it holds name the first authority, while `emit_checked` of the
  library names the second; the two-importer link refuses
  `invalid_package`/`conflicting-definition` naming the library.
