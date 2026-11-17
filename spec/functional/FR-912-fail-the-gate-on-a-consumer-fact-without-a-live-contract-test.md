---
id: FR-912
title: "Fail the gate on a consumer fact without a live contract test"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-911
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-661
    type: depends_on
---
# FR-912: Fail the gate on a consumer fact without a live contract test

## Description

`cargo xtask facade-tests` SHALL fail when a fact of the pinned consumer-fact
inventory has no live contract test (FR-911), when a contract test is ignored
or can be disabled, or when a contract test names a fact the inventory does
not hold. It is a step of `make ci`.

## Rationale

A contract test that is missing, ignored or compiled out protects nothing, and
the fact it should cover is then found missing by a consumer. The gate makes
each of those a failing FSL build. A fact whose accessor is not built yet fails
the gate until the accessor and its test land; an allowlist would hide exactly
the gap the gate exists to show.

## Inputs

The inventory read through the pinned `filament_specification::published_file`
(FR-760), or the file `--inventory <file>` names, and the Rust sources under
`tests/fixtures/facade-contract-consumer/tests/it`, or the directory
`--tests <dir>` names.

## Outputs

A report on stdout listing each fact id with the contract tests that cover it,
exit 0; or a report listing every failure, exit 1. A usage error, an unreadable
input or a source that does not parse exits 2.

## Behavior

1. The gate parses each `.rs` source with `syn` and collects every function
   named `cf_<code>_<nn>_<words>`, reading `<code>` and `<nn>` as the fact id
   `CF-<CODE>-<nn>`.
2. A collected function is **live** when it carries `#[test]` and carries no
   `#[ignore]` (with or without a reason), no `#[cfg(...)]` or
   `#[cfg_attr(...)]` attribute, and is not inside a module that carries one.
3. The gate passes only when every inventory fact has at least one live
   function and every collected function names an inventory fact. It reports
   all failures in one run: each fact with no live test, naming the fact; each
   function that is not live, naming the function, its file and why; and each
   function naming an id the inventory does not hold, naming the function and
   the id.
4. The gate holds no list of facts, tests, hashes, digests or exemptions of
   its own: the inventory and the test sources are its only inputs.
5. The gate is a `make ci` step after the contract tests' build, so the
   contract tests both compile and run under `make ci`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-912-AC-1 | Over a scratch test directory whose sources hold a live `cf_<code>_<nn>_*` test for every fact of the pinned inventory, `cargo xtask facade-tests --tests <dir>` exits 0 and its report lists every inventory fact id with its test name. | Test (TC-2578) |
| FR-912-AC-2 | Over the same directory with the `cf_pr_03_*` test removed, the gate exits 1 and names `CF-PR-03` as having no live test. | Test (TC-2579) |
| FR-912-AC-3 | Over the same directory with `#[ignore]` added to the `cf_pr_03_*` test, and again with `#[cfg(any())]` added to it, the gate exits 1 and names that function, its file and `CF-PR-03`. | Test (TC-2580) |
| FR-912-AC-4 | Over the same directory with a live `cf_pr_99_unknown` test added, the gate exits 1 and names `cf_pr_99_unknown` and `CF-PR-99` as naming no inventory fact; the `make ci` target runs `cargo xtask facade-tests`. | Test (TC-2581) |

## Dependencies

- [FR-911](FR-911-call-every-bound-consumer-fact-from-an-outside-crate.md)
  owns the contract tests and their naming.
- [FR-761](FR-761-bind-every-consumer-fact-to-a-public-accessor.md) owns the
  binding check that runs beside this gate.
- FSpec FR-661 owns the inventory.
