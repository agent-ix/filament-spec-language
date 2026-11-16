---
id: FR-910
title: "Check and compile a unit that declares only aliases or dimensions with one verdict"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-480
    type: references
  - target: ix://agent-ix/filament-specification/FR-746
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-910: Check and compile a unit that declares only aliases or dimensions with one verdict

## Description

FSL's check stage SHALL write a node for every declaration that FSpec FR-322
represents as a node, whether or not anything names it, and SHALL refuse a
package whose units declare no such declaration as
`missing_declaration`/`empty-package` (FSpec FR-746). `fsl check`, `fsl
compile` and `fsl run` then give one verdict for a declaration-only unit.

## Rationale

A unit of dimensions, units or enums is a library its author checks before
anything uses it, so it must compile as it checks. A unit of type aliases only
has nothing the wire can hold: an alias introduces no node, and the wire admits
no empty graph. Refusing it at check, with the rule named, replaces a check
that passes followed by a compile refusal about the wire that names no rule.

## Inputs

The unit list of one package (FR-480).

## Outputs

A checked package and its `filament.checked-package/v2` bytes, or the
`missing_declaration`/`empty-package` refusal from check.

## Behavior

1. S3 keeps a node for each source `dimension`, `unit`, `enum`, `record`,
   `tuple`, `union`, `function`, `predicate` and selected `model` declaration
   in the checked graph, and E4 writes it, whether or not a claim, function or
   other declaration names it.
2. A `type` alias resolves at each use to its target's node and adds no node.
   An alias declared in one unit of a unit list resolves in the other units.
3. After every unit is checked, a package with no node refuses at S3 as
   `missing_declaration`/`empty-package`, located at the header of the first
   unit, retaining the package name and each unit's source identity. The
   message states that a checked package needs at least one declaration that
   has a node and that a type alias has none. No checked package is returned.
4. Compile and run reach this refusal at check, so they report its code,
   cause and locus and never reach the emitter's
   `unsupported_projection` refusal for an empty graph. The emitter's
   empty-graph refusal remains only as a defence for a checked graph built
   some other way.
5. A run request against a declaration-only unit that compiles refuses its
   function or clause selection as FR-100 and FR-312 state.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-910-AC-1 | Under the value-profile header, a unit whose only declaration is `dimension Length;`, and a unit of `dimension Length;` and `unit m : Length = rational(1, 1);`, each pass S3 check and compile through the spine to bytes that Contract IR's strict reader admits; the first package's graph holds exactly one source `scalar_type`/`dimension` node. | Test (TC-2570) |
| FR-910-AC-2 | A unit whose only declaration is `type I = Integer;`, a unit of `type I = Integer;` and `type J = Boolean;`, and a header-only unit each refuse at S3 check with code `missing_declaration` and cause `empty-package`, and compile through the spine refuses with the same code and cause; neither refusal has code `unsupported_projection`, and neither message contains `no node of the package`. | Test (TC-2571) |
| FR-910-AC-3 | A unit of `type I = Integer;` and `dimension Length;` compiles, and so does a unit of `type I = Integer;` and `function f(x: I): I pure { x }`; neither package holds a node whose declaration is `I`. | Test (TC-2572) |
| FR-910-AC-4 | A unit list of `type Port = Integer;` followed by `function f(p: Port): Port pure { p }`, and the same two units in the other order, compile to a package holding `f`; the alias-only unit compiled alone refuses as AC-2 states. | Test (TC-2573) |

## Dependencies

- FSpec FR-746 owns the rule and the `empty-package` cause; FSpec FR-322 owns
  the node forms and the non-empty graph.
- [FR-027](FR-027-export-compiled-native-package.md) owns the compile request
  and [FR-480](FR-480-compile-and-link-a-package-from-several-source-units.md)
  the unit list.
