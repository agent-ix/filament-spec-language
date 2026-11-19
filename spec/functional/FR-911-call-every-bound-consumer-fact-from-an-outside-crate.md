---
id: FR-911
title: "Call every bound consumer fact from an outside crate over a checked package"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-519
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-535
    type: references
  - target: ix://agent-ix/filament-specification/FR-661
    type: depends_on
---
# FR-911: Call every bound consumer fact from an outside crate over a checked package

## Description

FSL SHALL hold, for every fact of the standard's consumer-fact inventory (FSpec
FR-661), a contract test that a crate outside the facade crates compiles and
runs: it calls each accessor that `docs/consumer-facades.json` binds to the
fact (FR-761), through that binding's module path, over a checked package or
run result, and asserts the value the fact's defining requirement gives for
that input.

## Rationale

FR-761's check proves that each bound accessor resolves to a public path. It
does not prove that a consumer can build the accessor's inputs from public
items, call it, and get the fact back. Each "expose X" request a consumer
filed was a fact the language computed but a consumer could not reach in one
of those ways. A test written as the consumer writes it fails to compile when
an input type, constructor or method is private, and fails at run time when
the accessor returns the wrong fact. It catches the next missing accessor in
FSL's own gate.

## Inputs

The pinned inventory (FR-760), `docs/consumer-facades.json`, and the source
units and published fixtures each test compiles or reads.

## Outputs

One passing integration test per inventory fact.

## Behavior

### The consumer crate

The contract tests live in the workspace member
`tests/fixtures/facade-contract-consumer` (package
`facade-contract-consumer`, `publish = false`). Its normal and development
dependencies on `fsl-package`, `fsl-replay` and `fsl-analyze` use their default
features only, and no dependency of the crate enables a `test-support` feature
of any `fsl-*` crate, so the crate sees exactly what an outside consumer sees.
Its tests are one integration-test target, `tests/it/main.rs`, with one module
per inventory consumer: `ir`, `protocol`, `code_generation`, `runtime`,
`driver` and `observation`.

### One test per fact

Each inventory fact `CF-<code>-<nn>` has at least one test function named
`cf_<code>_<nn>_<words>` (lowercase, `<words>` naming what it reads), in the
module of its consumer, carrying `#[test]` and no `#[ignore]`. The test:

1. builds its input through public items only: source compiled with
   `fsl_replay::compile_package` (or the bound compile entry its consumer
   uses), a published fixture read through the pinned standard (FR-760), or a
   run result written by the bound writer;
2. calls every accessor the binding lists for the fact, named by the binding's
   module path (`fsl_package::protocol_view::channel_endpoints`, not a
   re-export elsewhere);
3. asserts the value the fact's requirement states for that input, for
   example the declared sending role, receiving role and message type of a
   channel, the control a declared `await` follows, the owner and visible
   expressions of a declared choice, or the declared attempts maximum of a
   compensation; and
4. where the accessor's requirement gives a typed refusal for a wrong input
   (a node of another kind, an unknown key), passes one such input and asserts
   that typed refusal, with no panic and no default value.

A test reads no private field, uses no `test-support` item and copies no FSL
logic to compute the fact itself.

### What the crate cannot reach

The crate's library documentation holds one `compile_fail` doctest that names
the crate-private `fsl_package::protocol_view::relation_over`, pinned to error
`E0603`, so a change that made a non-contract item public, or a test that
reached a private item, is seen.

### Facts whose accessor is not yet built

A fact whose bound accessor FR-761 requires but FSL has not built yet gets its
test in the change that builds the accessor, written against the signature
FR-761 names. Until then FR-912's gate reports the fact as having no test.
There is no placeholder or ignored test for it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-911-AC-1 | `cargo metadata` lists the workspace member `facade-contract-consumer` at `tests/fixtures/facade-contract-consumer`; each of its dependencies on `fsl-package`, `fsl-replay` and `fsl-analyze`, normal and development, enables no feature beyond the default, and no dependency of the crate enables a `test-support` feature of an `fsl-*` crate. | Test (TC-2574) |
| FR-911-AC-2 | `cargo test -p facade-contract-consumer` runs and passes, with none ignored, one `cf_<code>_<nn>_*` test for each fact of the pinned inventory; among them the protocol tests read, over one compiled protocol clause, its view's roles and channels, the causal relation, each channel's sending role, receiving role and message type, the anchor of an `await`, the owner and visible expressions of a choice and the attempts maximum of a compensation, each equal to the declared value. | Test (TC-2575) |
| FR-911-AC-3 | For each bound accessor that takes a node or key and whose requirement gives a typed refusal for a wrong input, the fact's test passes a node of another kind or an unknown key and asserts that typed refusal; `channel_endpoints` given a role node returns its `ViewRefusal`, not a panic or a default. | Test (TC-2576) |
| FR-911-AC-4 | The crate's `compile_fail` doctest naming `fsl_package::protocol_view::relation_over` fails to compile with `E0603`; the same doctest naming the public `fsl_package::protocol_view::causal_relation` compiles. | Test (TC-2577) |

## Dependencies

- [FR-761](FR-761-bind-every-consumer-fact-to-a-public-accessor.md) owns the
  bindings each test follows; [FR-912](FR-912-fail-the-gate-on-a-consumer-fact-without-a-live-contract-test.md)
  fails the gate on a fact with no live test.
- FSpec FR-661 owns the inventory; the requirement each fact names owns the
  asserted value.
