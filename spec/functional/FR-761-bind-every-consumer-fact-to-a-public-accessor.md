---
id: FR-761
title: "Bind every consumer fact of the standard's inventory to a public accessor"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-519
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-248
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-535
    type: references
  - target: ix://agent-ix/filament-specification/FR-661
    type: depends_on
---
# FR-761: Bind every consumer fact of the standard's inventory to a public accessor

## Description

FSL SHALL bind every fact of the standard's consumer-fact inventory (FSpec
FR-661) to public accessors of one public module per consumer, in the binding
document `docs/consumer-facades.json`. `cargo xtask consumer-facades` SHALL
fail when a fact has no binding, a binding names no fact, or a bound accessor is
not public.

## Rationale

Consumers may not copy FSL logic, so each fact they read must be public. The
contract check turns a missing accessor into a failing FSL gate instead of a
downstream ticket.

## Inputs

The inventory `proposals/consumer-facades/consumer-facts.json` read from the
pinned standard (FR-760), and `docs/consumer-facades.json`.

## Outputs

A report on stdout listing every fact id with its resolved public paths, exit 0;
or a report listing every failure, exit 1. A usage error exits 2.

## Behavior

### The binding document

`docs/consumer-facades.json` is a closed JSON object: `bindings` is exactly
`fsl.consumer-facade-bindings/v1`; `consumers` holds one closed object
`{consumer, module, facts}` per inventory consumer, in inventory order. `module`
is the Rust path of one public module (`fsl_package::protocol_view`) or crate
root (`fsl_replay`). `facts` holds one closed object `{fact, accessors}` per
inventory fact of that consumer, in inventory order; `accessors` is a non-empty
list of item names or `Type::member` names, a member being a public method or
public field of a public type.

The committed bindings are:

| Consumer | Module |
| --- | --- |
| `ir` | `fsl_package` |
| `protocol` | `fsl_package::protocol_view` |
| `code-generation` | `fsl_replay` |
| `runtime` | `fsl_analyze::monitor_root` |
| `driver` | `fsl_replay` |
| `observation` | `fsl_package::checked_views` |

Each fact's accessors are those the document lists. Among them, the following
are required by this binding and supplied with it: `role_internality`
returning a `RoleInternality` of `Internal` or `External` (FSpec FR-217, fact
`CF-PR-04`); `SchedulabilityRoot::obligation_identity` (fact `CF-IR-04`);
`fsl_replay::check_closed_form`, the FR-248 facade entry (fact `CF-CG-04`);
`fsl_analyze::monitor_root::admit_progress_view`, which builds the
authenticated progress view and authority selections from a request or event
input (fact `CF-RT-02`); and `fsl_package::checked_views::subject_kinds`
returning each `SubjectKind` and relationship declaration from the checked
package's domain declarations (fact `CF-OB-01`).

### The contract check

`cargo xtask consumer-facades [--bindings <file>] [--inventory <file>]` reads the
inventory through the pinned `filament_specification::published_file`, unless a
test passes `--inventory`, and the committed binding document, unless
`--bindings` names another. It resolves each accessor at `<module>::<accessor>`
over the public API of the workspace crates as a crate outside them sees it,
following `pub use` re-exports. It passes only when every inventory fact has a
binding, every binding names an inventory fact and every accessor resolves. It
reports all failures in one run, each line naming the fact id and, for an
unresolved accessor, its full path. There is no ignored or pending binding. The
check is a step of `make ci`.

### Requirements naming a consumer fact

An FSL requirement that defines a fact a consumer reads names the fact's
inventory id and its accessor, and the change that adds it adds the inventory
fact in the standard and the binding here in the same release (FSpec FR-661).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-761-AC-1 | `cargo xtask consumer-facades` over the committed bindings exits 0 and its report lists every inventory fact id, `CF-PR-04` with `fsl_package::protocol_view::role_internality`, `CF-IR-04` with `fsl_package::SchedulabilityRoot::obligation_identity`, `CF-CG-04` with `fsl_replay::check_closed_form`, `CF-RT-02` with `fsl_analyze::monitor_root::admit_progress_view` and `CF-OB-01` with `fsl_package::checked_views::subject_kinds` among them. | Test (TC-2144) |
| FR-761-AC-2 | With a bindings file whose `CF-PR-03` accessors are `["no_such_accessor"]`, the check exits 1 and names `CF-PR-03` and `fsl_package::protocol_view::no_such_accessor`; a crate-private item named as an accessor fails the same way. | Test (TC-2145) |
| FR-761-AC-3 | With a bindings file that omits `CF-PR-04`, the check exits 1 and names `CF-PR-04` as unbound. | Test (TC-2146) |
| FR-761-AC-4 | With a bindings file that adds a binding for `CF-PR-99`, the check exits 1 and names `CF-PR-99` as a binding with no inventory fact. | Test (TC-2147) |

## Dependencies

- FSpec FR-661 owns the inventory and the binding rule.
- [FR-760](FR-760-consume-the-standard-and-publish-the-facades-through-versioned-cargo-pins.md)
  pins the standard the inventory is read from.
- FR-519, FR-121, FR-248, FR-251 and FR-535 define the bound facts.
