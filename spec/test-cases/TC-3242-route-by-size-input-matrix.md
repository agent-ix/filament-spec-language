---
id: TC-3242
title: "The public routes preserve typed outcomes across the QSL-3457 size matrix"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-869
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-277
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-264
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-862
    type: verifies
---
# TC-3242: The public routes preserve typed outcomes across the QSL-3457 size matrix

## Description

Exercise every workload row from the QSL-3457 route-by-size matrix through the
public compile, check/lower, checked-package read, analyze and replay routes.
The test matrix has five rows and five route columns. The inheritance/redefinition
row has two fixture variants in every route cell: inheritance and operation
redefinition. No row or route is excluded.

Scope: FR-869-AC-5..10, FR-277-AC-8..12, FR-264-AC-10..14,
FR-620-AC-12..16 and FR-862-AC-4..8. FR-259-AC-3 / TC-729 remains adjacent
domain-package JSON intake evidence; it is not the checked-package I2 reader
cell. FR-264-AC-5 remains the separate 100,000-deep I2 malformed-wire case.

## Test Procedure

The route fixtures use these exact inputs:

1. **1,000 nested:** a source expression with exactly 1,000 nested Boolean
   `not` nodes. For checked-package reading, use its emitted v2 package. For the
   replay route cell, exercise FR-862-AC-4's nested request-array fixture.
2. **10,000 flat terms:** a left-associated sum of exactly 10,000 integer
   literals, with the emitted package reused for read, analyze and replay.
3. **1,000 declarations:** exactly 1,000 uniquely named real enum type
   declarations, each with two uniquely named variants. I1 counts 1,000 type
   rows and 2,000 variant rows, so `model.declaration_records=3000` admits
   the fixture and 2,999 refuses with next charge 3000. Reuse the package
   and assert all 1,000 types and their 2,000 variants downstream.
4. **1,000 inheritance/redefinition links:** (a) 1,000 `extends` edges across
   1,001 types; (b) 1,000 operation `redefines` edges across 1,001 types, one
   query operation per type, only the deepest type concrete, matching
   FR-083-AC-5. Every route cell exercises both variants.
5. **100,000 collection values:** one ordered
   `Sequence<Integer>[0, 100000]` with values `0` through `99,999`, carried
   through the checked package and route fixture.

For each source workload, exercise compile and check/lower with the exact
counter controls in FR-869/FR-277. Where compile emits a checked package, reuse
its bytes for I2, analyze and replay and verify its `package_id`, values and
declarations. For the operation-redefinition compile cell, assert FR-869-AC-8's
typed `emit.output_bytes` refusal; its I2, analyze and replay cells use separate
valid route-specific checked-package/request fixtures for the same 1,000-link
shape, so those cells do not depend on that compile artifact. Analyze each route
fixture with one `Bounded` item; the collection fixture additionally asserts
`size(values)=100000`. Replay each valid package fixture and compare its result
with in-process `fsl_replay::replay`. Reuse FR-862-AC-4's recorded nested
request-array evidence for the 1,000-nested replay cell without rerunning it.
Each negative control changes only its named counter and expects that
counter's owning typed outcome.

Use focused filters in the existing route test modules:

- `fsl-command/tests/it/size_route_matrix.rs` for compile;
- `fsl-package/tests/it/size_route_matrix.rs` for I2 read;
- `fsl-analyze/tests/it/size_route_matrix.rs` for native analyze;
- `fsl-replay/tests/it/size_route_matrix.rs` for public check/lower;
- `fsl-command/tests/it/replay_size_matrix.rs` for native replay through `command::run`.

Tag each added test with its cell's FR acceptance criterion. The 1,000-level
replay-array test remains tagged with FR-862-AC-4.

For the compile rewrite complexity criterion FR-869-AC-10, the 10,000-value
fixture asserts at most 6 node-ID slot visits per value during operand-slot
emission. Inverse read-index rewrite work counts one unit per ID-slot visit,
per full path-length inspection allowance for an ID predicate or
structural-key lookup, and per structural-key lookup; the 12,000-value fixture
asserts at most 48 such units per value, including source-map and node-held
IDs. The measured post-fix work is 504,344 units for 12,000 values (42.03
per value), while the recorded pre-fix repeated scan used 432,168,038 units.
This 48-unit bound admits the linear post-fix result and rejects that recorded
quadratic baseline by more than 700 times. These counters are test-support
instrumentation, not caller-visible stage counters.

For the additional compile rewrite criterion FR-869-AC-11, the accepted
12,000-value fixture counts both structural-to-index rewriting and operand-slot
restoration. One work unit is an ID-slot visit, one full path-length inspection
allowance for an ID predicate or rewrite, one structural-key lookup, or one
operand-row lookup. The measured linear total is 2,233,442 units
(1,584,882 structural-to-index and 648,560 operand-slot-to-index), or 186.12
per value; adding 14% headroom and rounding up gives the 213-unit ceiling.
The test reports each component and their combined total, rejects the
4,753,944,407-unit repeated-replacement baseline, and asserts identical
checked-package bytes between the repeated-reference and one-pass paths.
These counters are test-support instrumentation, not caller-visible stage
counters.

## Expected Results

| Workload row | Compile | Check/lower | Checked-package read | Analyze | Replay |
| --- | --- | --- | --- | --- | --- |
| 1,000 nested | FR-869-AC-5: package emitted at counted `s1.nodes`; one below is `stage_limit_exceeded` naming `s1.nodes`. | FR-277-AC-8: success at counted `s3.nodes`; one below is `LimitExceeded{s3.nodes}`. | FR-264-AC-10: I2 verifies `package_id`; byte and node stops name `i2.input_bytes` and `i2.nodes` as `StageFailure::Limit`. | FR-620-AC-12: exact `request_bytes` admits `Bounded: proved`; one added byte yields document-level `incomplete`/`resource_exhausted`, exit 22. | FR-862-AC-4: nested request refuses `invalid-request` at `request`, exit 20, names its member, and does not overflow. |
| 10,000 flat terms | FR-869-AC-6: package emitted at counted `s3.nodes`; one below is `stage_limit_exceeded`; `emit.output_bytes` is separate. | FR-277-AC-9: success at counted `s3.nodes`; one below is `LimitExceeded{s3.nodes}`. | FR-264-AC-11: I2 verifies `package_id`; one below node count is `StageFailure::Limit`/`i2.nodes`. | FR-620-AC-13: exact `request_bytes` gives `Bounded: proved`; one byte above yields the FR-624-AC-6 incomplete document. | FR-862-AC-5: replay is `reproduced-without-witness`, equal to in-process replay; envelope over-limit names `replay.input_bytes`. |
| 1,000 declarations | FR-869-AC-7: all 1,000 enum types and 2,000 variants emit at counted `model.declaration_records=3000`; 2,999 yields `resource_exhausted`/`insufficient-next-charge`. | FR-277-AC-10: same 3,000-record model-intake outcome before S3; separate S3 limit is `LimitExceeded{s3.nodes}`. | FR-264-AC-12: I2 retains all 1,000 enum types, their 2,000 variants and package id; byte/node limits name `i2.input_bytes`/`i2.nodes`. | FR-620-AC-14: exact request is admitted and `Bounded` is `proved`; one byte above yields the FR-624-AC-6 incomplete document. | FR-862-AC-6: replay is `reproduced-without-witness`, equal to in-process replay; envelope over-limit names `replay.input_bytes`. |
| 1,000 inheritance/redefinition links | FR-869-AC-8: inheritance emits at `model.ancestor_steps=1000`; its one-under model limit refuses `resource_exhausted`/`insufficient-next-charge`. The redefinition chain completes normalization at `model.derivation_facts=1504502` but compile stops with `stage_limit_exceeded` at default `emit.output_bytes=16777216`, actual 16777217; one-under model facts refuses before emission. | FR-277-AC-11: both route fixtures preserve their model-limit outcomes; an S3 stop remains `LimitExceeded{s3.nodes}`. | FR-264-AC-13: separate valid checked-package fixtures preserve both relations and package ids; one-under `i2.edges` (inheritance) / `i2.nodes` (redefinition) is `StageFailure::Limit`. | FR-620-AC-15: separate route fixtures' `Bounded` items are `proved`; request-byte and engine-limit outcomes remain distinct. | FR-862-AC-7: separate valid package fixtures replay to `reproduced-without-witness`, equal to in-process `fsl_replay::replay`; envelope over-limit names `replay.input_bytes`. |
| 100,000 collection values | FR-869-AC-9: default `model.sequence_maximum=10000` yields `resource_exhausted`/`intake-limit-exceeded` with `kind=node-count-exceeded`, bound 10000, actual 100000 and setting `model.sequence_maximum`; at 100,000 plus required source/check/emit bounds a package is emitted. | FR-277-AC-12: the same typed model-intake outcome occurs before S3; at 100,000, check succeeds at counted `s3.nodes`, one below is `LimitExceeded{s3.nodes}`. | FR-264-AC-14: I2 preserves all ordered values and package id; one-under byte/node limits name the corresponding `i2.*` setting. | FR-620-AC-16: `Bounded` with `size(values)=100000` is `proved`; request overrun is the FR-624-AC-6 incomplete document; under FR-106-AC-27 every value form counts, so the exact observation-value bound is 100002 (scalar field, sequence form and 100,000 elements) and one-under 100001 returns typed `stage_limit_exceeded` naming `observation.values` with count reached 100002. | FR-862-AC-8: replay is `reproduced-without-witness`, equal to in-process replay; envelope over-limit names `replay.input_bytes`. |

The byte counters remain distinct: compile uses `request_bytes`,
`dependent_bytes` and `emit.output_bytes`; analyze uses `request_bytes`; I2
read uses `i2.input_bytes`; replay uses `replay.input_bytes`. `identity.input_bytes`
is used only when a route encodes or reads a canonical identity key (FR-259-AC-6/7),
not for checked-package reading.
