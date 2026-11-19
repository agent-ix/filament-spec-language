---
id: FR-964
title: "Share one provider options type across the provider-routing entries"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-950
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-961
    type: references
  - target: ix://agent-ix/filament-specification/FR-462
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-777
    type: depends_on
---
# FR-964: Share one provider options type across the provider-routing entries

## Description

The FSpec FR-462 limits of a run's providers SHALL be carried by one public
type, `fsl_analyze::provider::ProviderOptions`, which every provider-routing
entry takes as its `options` parameter: `analyze_with_providers`,
`analyze_with_providers_and_limits`, `run_with_providers` (FR-961), the
provider entry of `command::monitor` (FR-644) and the FR-280 entries the
driver's `prove`, `replay` and `generate` call.

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProviderOptions {
    pub provider_limits: ProviderLimits,
}
```

`ProviderLimits` is the existing type holding the seven FR-462 limits by
FR-287's names, with FR-287's defaults.

### Decision: the limits belong to the run, not to an engine

FR-287's `--provider-limit` sets each limit once for a run, and every
provider the run uses works under it; FSpec FR-777 states the same of every
provider-routing operation. Limits held on each `Engine::external` engine
would let two providers of one run carry different limits, which no
invocation can ask for, and would leave the linked engines with no place to
read `deadline_ms`. So `Engine::external(descriptor, tool, provider)` takes no
limits, and the entry reads `options.provider_limits` when it builds each
`ProviderRequest`: `request.deadline_ms` from its `deadline_ms`, and the
unserialized `transport_limits` from the other six. The type is named for
provider routing, not for one verb, because analyze and run share it. No
`AnalyzeOptions` type exists; where FR-635, FR-950 and FR-287 name the run's
limits, they name `ProviderOptions.provider_limits`.

## Inputs

The limits the driver resolves from `--provider-limit` before any stage runs
and any plugin starts (FR-287).

## Outputs

One `ProviderOptions` value per entry call.

## Behavior

- Each provider-routing entry SHALL take `options: &ProviderOptions` and SHALL
  write `options.provider_limits` into every `ProviderRequest` it builds.
- `ProviderOptions::default()` SHALL hold FR-287's defaults, so an entry
  given the default value sends `deadline_ms` 600000.
- An entry SHALL read no limit from an engine, an environment variable or a
  file.
- A linked provider SHALL receive `deadline_ms` in its request and ignore the
  transport limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-964-AC-1 | `analyze_with_providers` and `run_with_providers`, each given a recording provider and `ProviderOptions { provider_limits: ProviderLimits { deadline_ms: 250, max_frame_bytes: 1048576, ..Default::default() } }`, send that provider a `ProviderRequest` whose `deadline_ms()` is 250 and whose limits hold `max_frame_bytes` 1048576 and FR-287's defaults for the other five; both entries given `ProviderOptions::default()` send `deadline_ms` 600000. | Test (TC-2704) |
| FR-964-AC-2 | `Engine::external` takes a descriptor, a tool and a provider and no limits, and two recording providers of one run receive requests with equal limits, the run's. | Test (TC-2704) |

## Dependencies

- FR-287: the option and defaults. FR-635, FR-950, FR-961: the entries.
- FSpec FR-462: the limits; FSpec FR-777: one limit set per run.
