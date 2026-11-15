---
id: FR-634
title: "Request a simulation of finite nondeterministic behaviour through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-181
    type: depends_on
---
# FR-634: Request a simulation of finite nondeterministic behaviour through analyze

## Description

A `native-analyze/1` item with `kind` `simulation` SHALL enumerate or sample
the declared nondeterministic choices of a model subject within explicit
bounds (FSpec FR-181) through FR-120's simulator, and report the traces, the
coverage and the unexplored frontier.

### Decision: simulation is an analyze item that settles no claim

Simulation explores behaviours as model checking does but proves nothing; it
needs the same subject, documents and budgets. It is therefore an item of
the analyze request rather than a separate verb, with its own kind
`simulation` outside FSpec FR-290's claim vocabulary. It is not negotiated:
it runs on the in-process simulator and no other backend.

## Inputs

### Request item

- `kind`: `simulation`; no `claim`.
- `temporal.subject`: optional, the request's default subject otherwise.
- `simulation`: `{"mode": "enumerate" | "sample", "seed", "samples",
  "max_steps", "max_states", "max_state_key_bytes"}`: `seed` and `samples`
  only for `sample`; every bound a positive count with FSpec FR-181's
  published default.

## Outputs

- `category`: `success` when enumeration exhausted the bounded behaviours or
  sampling drew every requested sample; `incomplete` when a bound stopped it,
  with cause `LimitReached{limit}`, `state-key-limit-exceeded` or
  `cancelled`.
- `traces`: each trace's states in FSpec FR-181's typed canonical form and
  the choice taken at each step.
- `coverage`: `{"states", "transitions", "choices"}` counts.
- `frontier`: the unexplored states by state key, or by position for a key
  beyond `max_state_key_bytes`.
- `seed`: echoed for `sample`.
- No `record`, `certificate`, `counterexample` or `axes`: the item settles no
  claim.

## Behavior

- Enumeration is deterministic; two runs of one request give equal
  documents. Sampling with one seed gives equal documents.
- **Cancellation** returns `incomplete` with cause `cancelled` and the
  frontier at the time of cancellation.
- **Refusals.** A subject that does not admit refuses the whole request
  (FR-624); `seed` with `enumerate` refuses `invalid-request` naming it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-634-AC-1 | Enumerating the Counter subject gives `success`, `coverage.states` 4 and `coverage.transitions` 3, one trace reaching `value = 3`, and an empty frontier. | Test (TC-1594) |
| FR-634-AC-2 | `max_states` 2 gives `incomplete` `LimitReached{max_states}` with a non-empty frontier. | Test (TC-1594) |
| FR-634-AC-3 | Two `sample` runs with seed 5 and `samples` 3 give equal documents. | Test (TC-1594) |

## Dependencies

- FR-120 owns the simulator; FSpec FR-181 the exploration contract.
