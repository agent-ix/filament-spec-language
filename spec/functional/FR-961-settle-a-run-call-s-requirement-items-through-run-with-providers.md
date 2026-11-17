---
id: FR-961
title: "Settle a run call's requirement items through run_with_providers"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-950
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-395
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-280
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-960
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-962
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-963
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-964
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-711
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-775
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-967
    type: depends_on
---
# FR-961: Settle a run call's requirement items through run_with_providers

## Description

FSL's run-side provider entry SHALL be

```rust
fsl_command::command::run_with_providers(
    request: &Path,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Result<RunResult, Box<RunError>>
```

It runs the call the native-run/1 request names as FR-100 does and, when the
call carries `requirements` (FR-960), settles the call's requirement items
against the registry `Engines::fsl()` together with `providers`, by FR-635's
rule for every provider-routing entry. `fsl run` calls it, with the processes
it started from `--provider-config` as `providers` (an empty vector when the
option is absent) and the limits it resolved from `--provider-limit` as
`options.provider_limits` (FR-287, FR-964).

### Decision: the negotiator is a parameter

Negotiation is not an FSL stage: `filament-contract-codegen`'s `negotiate_item`
is the one negotiation point (FSpec FR-711), and FSL depends on no CG crate
(FR-280). So the entry takes the caller's negotiator, the same
`fsl_analyze::analyze::document::Negotiate` that `analyze_with_providers` takes,
and FSL calls no negotiation of its own. The driver passes its adapter over CG's
`negotiate_item`, which is the production negotiator, to both entries. The run
and analyze therefore settle the same item over the same registry with the
same disposition, and a test passes a conforming FSpec FR-290 double.

### Decision: plain `command::run` settles with the linked engines

`fsl_command::command::run` and its siblings (`run_with_overrides`,
`run_with_limits`, `run_with_limits_and_overrides`) SHALL take the caller's
negotiator as one more parameter, `negotiate: &dyn Negotiate`, and SHALL be
`run_with_providers` with an empty `providers`, `ProviderOptions::default()`
and a handle that is never cancelled. A call carrying `requirements` through
them therefore settles every item of the call with the linked engines alone:
an item that negotiates to a linked engine is decided by it, and an item that
names a backend the linked registry does not hold settles `invalid-request`
`unknown-backend` (FR-960). No provider is called, because none is
registered. The negotiator is called only for the items of a call carrying
`requirements`; a request without them, and a `native-monitor/1` or
`native-replay/1` request the same entry reads, never calls it.

## Inputs

- The native-run/1 request file, prepared by FR-100's one preparation path.
- The caller's negotiator.
- `providers`: the started process providers, each an `Engine::external` over
  its `Process` descriptor and its `ExternalProvider` transport.
- `options`: the run's `ProviderOptions` (FR-964).
- The caller's `Cancel`.

## Outputs

The `RunResult` of the run, with the call's outcome and, for a call carrying
`requirements`, one settled entry per item of the call (FR-963).

## Behavior

The entry SHALL work in this order:

1. Prepare the request (FR-100 "Request-file preparation for the driver").
2. Build the registry from `Engines::fsl()` and `providers` before any stage
   runs. A provider repeating a registered `BackendId` refuses the request
   `invalid-request` naming `providers`, before any stage, as
   `analyze_with_providers` does.
3. Compile and run the call through FR-395's `run_with_requirements`, which
   returns the call's outcome, the same as `run` returns for the same inputs,
   with the items of the call. A refusal before the call settles no item and
   is the run's outcome.
4. When the call carries `requirements`, for each item of the call, in
   FR-395's package order: write its named backend and options (FR-960),
   check its options against the named backend's manifest (FR-967): an item
   whose named backend the registry does not hold settles `unknown-backend`
   and is not checked further, and one naming an option that backend does not
   publish settles `invalid-request` `invalid_capability`/`unknown-option` and
   is not negotiated; then compute its candidates by FR-075 and FR-962, and
   negotiate it with `negotiate`. Items are settled whatever the call's outcome, since the
   items of a refused, undefined or incomplete call are those of a completed
   one (FR-395). No provider is called while items are built and negotiated.
5. Settle each negotiated item as FR-281 states: an item `Supported` on a
   linked engine runs on it in-process; the items `Supported` on an external
   provider go through `fsl_analyze::provider::settle_items`, which builds one
   `ProviderRequest` per backend by FR-950's table, its `package` the
   admitted bytes of this run's own S4 emission (FR-940), and calls
   `ExternalProvider::settle` once per request under `cancel`; an item
   settled `requires-bound`, `unsupported` or `invalid-request` runs nowhere
   and its record is settled by that disposition. A provider's `refuted`
   verdict settles `refuted` only after S6a replay reproduces its
   counterexample.
6. Return the call's outcome and the settled items (FR-963).

- Once `cancel` fires, no further request SHALL be sent, and every item not
  yet settled SHALL settle `incomplete` with cause `cancelled{source}`.
- The call's own evaluation SHALL be unchanged by `requirements`, `providers`
  and `options`: the same call gives the same value, undefined reason or
  exhausted limit with or without them.
- There SHALL be one provider path for a run: no run entry builds an FSpec
  FR-331 request item, reads package bytes for a provider, or negotiates an
  item by any other route.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-961-AC-1 | `run_with_providers` over a run of `Even using v(n: Integer): Boolean pure { n * 2 - n = n }` with `n = 4` and `call.requirements` `{"backend": "fsl.test-recording-provider"}`, the recording provider (advertising `value-validity` `unbounded`) in `providers` and `options.provider_limits.deadline_ms` 250, calls the negotiator once per item of `Even`, then the provider's `settle` exactly once with one `ProviderRequest` holding every item of `Even` in package order, whose `package` decodes with the v2 reader to the run's admitted package and whose `request.deadline_ms` is 250; the result holds `{"boolean": true}` and each item `proved` naming the provider with certification `trusted`. | Test (TC-2701) |
| FR-961-AC-2 | The same request through `command::run` with the same negotiator settles each item `invalid-request` `invalid_capability`/`unknown-backend` naming `fsl.test-recording-provider`, holds the same `{"boolean": true}`, and no provider is called; the same call with `requirements` `{}` and no providers settles each item by the negotiation of its linked candidates and calls no provider. | Test (TC-2701) |
| FR-961-AC-3 | A run without `call.requirements` through `run_with_providers` with the recording provider registered never calls the negotiator or the provider, and its result equals `command::run`'s for the same request with no `requirements` member in it. | Test (TC-2701) |
| FR-961-AC-4 | Negative: two providers with one `BackendId` refuse the request `invalid-request` naming `providers` before any stage, and neither provider is called; a call whose compile refuses settles no item and calls neither the negotiator nor a provider. | Test (TC-2701) |
| FR-961-AC-5 | A recording provider that reports `refuted` with counterexample `n = 4` for an item that the native evaluator does not violate at `n = 4` settles that item `inconclusive` with cause `replay-parity`, never `refuted`; a handle cancelled before the run settles every item `incomplete` with `cancelled{source: requested}` and sends no request. | Test (TC-2701) |

## Dependencies

- FR-100, FR-395: the call and its items. FR-960: naming their backends.
- FR-635, FR-950: the routing rule and the provider request this entry reuses.
- FR-962: unnamed items. FR-963: the outcome. FR-964: the options type.
- FSpec FR-711: the negotiation point; FSpec FR-775: the standard's `execute`
  settlement of requirement items.
