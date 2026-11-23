---
id: TC-721
title: "Every setting raises its limit through the library, the replay request and the CLI"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
---
# TC-721: Every setting raises its limit through the library, the replay request and the CLI

## Description

Verify that each setting name raises its limit at each entry point, that
the settings operation refuses a malformed `--limit` operand before any
stage runs, and that an
unconfigured limit takes its default.

Scope: FR-255-AC-4, FR-255-AC-5, FR-255-AC-6, FR-255-AC-15 to FR-255-AC-20, FR-255-AC-24 and FR-255-AC-25; FR-644-AC-15 to FR-644-AC-19.

## Test Procedure

1. For each row of FR-255's setting table, take TC-720 step 1's input at
   the row's default bound and run it again with the row's setting raised
   to fit it: through the limits type's builder method; through a replay
   request whose `stage_limits` carries that entry, for each row whose stage
   a replay runs; and through the settings operation given the operand
   `<name>=<value>`.
2. Call the settings operation with `s9.nodes=1`, with `s3.nodes=ten`, with
   `s3.nodes=-1`, and with `s3.nodes=5` and `s3.nodes=6` together.
3. Compile a unit with no limit configured and read the checked package's
   effective limits.
4. Read the native identity-join fixture with explicit limits and no outer
   I2 configuration (FR-255-AC-15), then with the outer configuration supplying
   the table defaults.
5. Set `i2.work_units` and `i2.native_join_storage` independently through the
   library limits, replay `stage_limits`, the settings operation and the FR-517
   request's `native_join` (FR-255-AC-16).
6. Set each once to `0` and once to `u64::MAX` (FR-255-AC-17).
7. Measure the fixture's native join charges W (work) and S (storage), then read
   with (W, S), (W - 1, S) and (W, S - 1) (FR-255-AC-18).
8. Retain each refused read's `stage_limit_exceeded`-class refusal, the IR error
   and the shared budget's first unpaid charge (FR-255-AC-19).
9. Inspect the public declaration of `ExactTypeProjectionError` or compile a
   public API probe against it; enumerate its public fields and check whether a
   field named `cause` exists.

10. Set `emit.output_bytes` through the emit limits' builder, a replay
   request's `stage_limits` and the settings operation; read the outer
   configuration's default; set `0` and `u64::MAX` (FR-255-AC-20).
11. For each of `analyze`, `monitor` and `replay`, construct the command's
    caller-limit object from every exact setting in FR-255's “Lifecycle
    caller-limit coverage”. Verify each accepted name changes only its
    corresponding bound, and a recognized engine setting that the operation
    does not execute is refused before work. For monitor, separately set
    `monitor.preparation_work_units` and `monitor.preparation_storage` and
    inspect the corresponding preparation bounds. For monitor requests with
    `target`, supply `monitor.max_activations` and inspect the matching plan
    activation capacity; omit `target` and verify that
    `monitor.max_activations` is operation-unavailable while the preparation
    settings cause no preparation. Verify `replay.input_bytes` reaches the
    replay-reader argument and is not accepted in request `stage_limits`.
12. For FR-644-AC-16's timed monitor request with a valid target and
    `max_event_rate: null`, measure plan-preparation work W and storage S.
    Run with work W and storage S; then independently run with work W-1 and
    storage S-1. Confirm the refusal fields, setting names, and that no record
    is read on either reached limit. Repeat the request without `target` and
    confirm neither preparation counter is charged (FR-644-AC-15, AC-18,
    AC-19; FR-255-AC-24).

Tag the tests `#[trace("TC-721", "FR-255-AC-20")]`, `#[trace("TC-721", "FR-255-AC-4")]`, `#[trace("TC-721", "FR-255-AC-5")]`, `#[trace("TC-721", "FR-255-AC-6")]`, `#[trace("TC-721", "FR-255-AC-15")]`, `#[trace("TC-721", "FR-255-AC-16")]`, `#[trace("TC-721", "FR-255-AC-17")]`, `#[trace("TC-721", "FR-255-AC-18")]`, `#[trace("TC-721", "FR-255-AC-19")]`, `#[trace("TC-721", "FR-255-AC-24")]`, `#[trace("TC-721", "FR-255-AC-25")]`, `#[trace("TC-721", "FR-644-AC-15")]`, `#[trace("TC-721", "FR-644-AC-16")]`, `#[trace("TC-721", "FR-644-AC-17")]`, `#[trace("TC-721", "FR-644-AC-18")]` and `#[trace("TC-721", "FR-644-AC-19")]`.

## Expected Results

- Step 1: every rerun passes the limit it reached at the default.
- Step 2: each returns a usage refusal naming the offending operand, and no
  stage runs.
- Step 3: each stage ran at FR-255's defaults, and the recorded effective
  limits equal them.
- Step 4: with explicit limits the fixture reads under them; with the outer
  configuration the defaults are `i2.work_units=1000000` and
  `i2.native_join_storage=16777216`; a limits value with no storage value is
  not constructible.
- Step 5: every override changes only its matching field.
- Step 6: zero and `u64::MAX` are accepted; zero storage refuses the nonempty
  join.
- Step 7: `(W, S)` admits the join; `(W - 1, S)` refuses naming
  `i2.work_units`; `(W, S - 1)` refuses naming `i2.native_join_storage`.
- Step 8: each refusal is `stage_limit_exceeded`-class with `kind`, `bound`,
  `actual` and `setting`; the IR error has code `exhausted_work` or
  `exhausted_storage` and no separate public `cause` field, and
  `NativeJoinBudget::first_unpaid()` gives the first denied charge kind and
  amount.
- Step 9: `ExactTypeProjectionError` exposes `code` and has no public field
  named `cause`.
- Step 10: each entry point changes only the output-byte bound; the outer
  configuration's default is 16777216; zero and `u64::MAX` are accepted;
  `EmitLimits` has no `Default`.
- Step 11: each operation exposes exactly the names listed for it; accepted
  values change their matching bounds, unavailable names refuse before work,
  the monitor preparation bounds remain independent; a monitor target
  requires `monitor.max_activations`, the value reaches only
  `MonitorTarget.max_activations` / `activation_capacity`, and the setting is
  unavailable without a target; `replay.input_bytes` reaches the reader only.
- Step 12: a targetless monitor charges no preparation work or storage. With a
  target, the exact measured work and storage bounds admit preparation, each
  one-short bound refuses before reading records with `stage_limit_exceeded`
  and its `kind`, `bound`, `actual` and setting fields, naming the corresponding
  setting as FR-255 Behavior 1 states.
