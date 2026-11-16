---
id: FR-670
title: "Select the integer-division law from a unit header profile"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-508
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-147
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-570
    type: depends_on
---
# FR-670: Select the integer-division law from a unit header profile

## Description

Integer `div` and `rem` have no meaning until one of three laws is chosen:
truncating, floor or Euclidean (FSpec FR-147). The law changes the result of
the same expression at negative operands, so it is part of what the source
means and part of the checked package's identity. FSpec FR-570 places that
choice in the unit's header: a `profile` declaration whose identity names one
of the three integer-division definitions selects that law for the unit. This
requirement states how FSL resolves such a header, how the selected law
reaches checking, and what is refused.

The header is the unit's own statement of its law because the source is the
only thing every consumer of a unit reads. A caller may supply a law instead,
through the compile facade (FR-495) or, from the command line and request
files, through FR-767's `--integer-division` option and `integer_division`
member. A supplied law fills the package selection's integer-division slot,
which is part of the package identity, so two compiles of one text under two
supplied laws give two packages with different identities, and a supplied law
that disagrees with the header is refused. A keyword on the operator would let one unit mix laws, which FSpec FR-147
forbids. A default law would choose a meaning the author never stated, which
FSpec FR-147 also forbids ("never inferred from operator spelling").

## Inputs

- The unit's header profile declarations, each an alias and an identity
  (FSpec shared grammar `profile = 'profile', ident, '=', string, ';'`).
- An optional caller-supplied owner-admitted selection, obtained through
  `admit_integer_division` and given to the compile facade (FR-495 "Public
  compile-facade division selection"), or carried as the lock selection's
  `integer_division_profile` alternative role (FSpec FR-001-AC-7).

## Outputs

- `PackageDeclarations::integer_division` holding the admitted law the
  header selected, or the caller-supplied law, or `None`.
- A located refusal: `invalid_package`/`conflicting-definition`,
  `unknown_profile`/`wrong-selection-role`, or the existing
  `missing_declaration`/`missing-selection` for the `IntegerDivision` role.

## Behavior

### Resolving a law header

The three law identities are `filament.value.integer-division.euclidean/v1`,
`filament.value.integer-division.floor/v1` and
`filament.value.integer-division.truncating/v1`.

- When a header profile's identity is one of the three law identities, E3
  SHALL resolve it to a law selection for that definition, not to a layer
  and not to `unknown_profile`/`wrong-selection-role`. It selects no layer
  and adds no admitted form.
- The assembler SHALL admit that definition through `admit_integer_division`
  and place the admitted law in `PackageDeclarations::integer_division`
  before checking, so checking, lowering and emission retain the law exactly
  as they retain a caller-supplied one (FR-495).
- Two law headers that name the same law SHALL both resolve, and the law is
  selected once.
- Two law headers that name different laws SHALL refuse
  `invalid_package`/`conflicting-definition` at the identity literal of the
  later header, retaining both definition references. No checked package is
  produced. One unit has one law because FSpec FR-147 forbids mixing laws.

### Scope of the selected law

- The selected law applies to every integer `div` and `rem` in the unit's own
  declarations: function bodies, clause bodies admitted under
  `filament.value.complete/v1`, and refinement rows (FSpec FR-001-AC-7). A
  function called from another function of the same unit evaluates under the
  same law.
- An imported library is compiled from its own unit, so its `div` and `rem`
  use the law of its own header. The importing unit's law never reaches a
  library's declarations, and a library's law never reaches the importer.
- `mod` is the Euclidean remainder under every law (FSpec FR-147-AC-5). A law
  header never changes a `mod`, and a `mod`-only unit needs no law header.
- A law header in a unit with no `div` or `rem` SHALL be admitted. The
  package selection holds no law, because the `integer_div_rem` trigger is
  absent (FSpec `value-complete.md`), so that unit's package identity equals
  the identity of the same unit without the header.

### Caller-supplied selection

- When the unit has a law header and no caller-supplied selection, the header
  decides the law.
- When the unit has no law header, a caller-supplied selection decides the
  law, as FR-495 states.
- When both are present and name the same law, the law is selected once.
- When both are present and name different laws, compile SHALL refuse
  `invalid_package`/`conflicting-definition` at the header's identity
  literal, retaining both definition references, before checking.
- When neither is present, each `div` and `rem` SHALL refuse
  `missing_declaration`/`missing-selection` for the `IntegerDivision` role at
  the operator (FR-495). No law is chosen by default.

### Using a law alias

A law header is not a layer, so its alias does not govern a declaration. A
declaration whose `using` alias names a law header, and a `protocol`,
`check`, `await` or `compensate` site whose alias names one (FR-508), SHALL
refuse `unknown_profile`/`wrong-selection-role` at the alias, retaining the
alias, the law identity and the required role.

### Commands and editor

- `fsl check`, `fsl run` and every other command that compiles a unit read
  the law from the unit's header through spine compile. `fsl check`'s
  `--integer-division` option and the requests' `integer_division` member
  supply a caller law, which composes with the header as "Caller-supplied
  selection" states (FR-767).
- The editor's profile check (FR-110 "Editor profile check") SHALL admit a
  header profile whose identity is one of the three law identities, so the
  editor and E3 agree on what a header may name.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-670-AC-1 | For each law `L` of `euclidean`, `floor` and `truncating`, a unit with `profile v = "filament.value.complete/v1";` and `profile d = "filament.value.integer-division.L/v1";` declaring `function q using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }` and `function r using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a rem b }` checks under `CheckMode::Linked` with `PackageDeclarations::integer_division` holding `L`, and calling `q(-7, 3)` and `r(-7, 3)` returns `-2` and `-1` under truncating, `-3` and `2` under floor, and `-3` and `2` under Euclidean. | Test (TC-1780) |
| FR-670-AC-2 | The unit of AC-1 with no law header, and no caller-supplied selection, refuses `missing_declaration`/`missing-selection` for the `IntegerDivision` role at each `div` and `rem`; the same unit with only `function m using v(a: Int[-7, 7], b: Int[1, 3]): Int[0, 2] pure { a mod b }` checks with no law. | Test (TC-1780) |
| FR-670-AC-3 | A unit with law headers `d1` naming floor and `d2` naming truncating refuses `invalid_package`/`conflicting-definition` at `d2`'s identity literal, retaining both definition references, and produces no checked package; with `d1` and `d2` both naming floor it checks with the floor law. | Test (TC-1781) |
| FR-670-AC-4 | `function q using d(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }`, where `d` is a law header, refuses `unknown_profile`/`wrong-selection-role` at the `using` alias `d`; `check c using d { true }` refuses the same way. | Test (TC-1781) |
| FR-670-AC-5 | The AC-1 unit with a floor law header compiled through the public compile facade with a caller-supplied floor selection checks with the floor law; with a caller-supplied truncating selection it refuses `invalid_package`/`conflicting-definition` at the header's identity literal before checking; with no header and a caller-supplied truncating selection it checks with the truncating law. | Test (TC-1782) |
| FR-670-AC-6 | A unit with a floor law header and only `mod` and `+` operators checks, and its checked package's selection holds no integer-division role; its package identity equals that of the same unit without the law header. A unit importing a library whose header selects truncating, itself selecting floor, evaluates the library function `ld(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] { a div b }` at `(-7, 3)` to `-2` and its own `q(-7, 3)` to `-3`. | Test (TC-1782) |
| FR-670-AC-7 | `complete::parse_with_catalog` with `complete::header_profile_catalog()` admits a header profile naming each of the three law identities and refuses `test:unknown-profile` with `unknown_profile`/`unsupported-selection`. | Test (TC-1782) |

## Dependencies

- FSpec FR-570 states the selection rule; FSpec FR-147 owns the laws and the
  ban on mixing or inferring them; FSpec `value-complete.md` owns the package
  selection and its `integer_div_rem` trigger.
- FR-110 owns header resolution and the editor catalog; FR-508 owns the
  per-site profile kind; FR-495 owns the compile facade, the
  `MissingSelection` refusal and the checking of the operators.
