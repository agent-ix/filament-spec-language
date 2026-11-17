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
  the choice taken at each step, as "Traces" below selects them.
- `coverage`: `{"states", "transitions", "choices"}`, each an ASCII decimal
  count over what the run visited, with one meaning in both modes:
  `states` the distinct state keys visited, `transitions` the distinct
  (pre-state, transition identity, post-state) edges traversed, and
  `choices` the distinct visited states at which more than one successor
  was enabled, the nondeterministic choice points the run met.
- `frontier`: the unexplored states by state key, or by position for a key
  beyond `max_state_key_bytes`.
- `seed`: echoed for `sample`.
- No `record`, `certificate`, `counterexample` or `axes`: the item settles no
  claim.

## Behavior

- Enumeration is deterministic; two runs of one request give equal
  documents. Sampling with one seed gives equal documents.

### Traces

- **Enumerate.** Enumeration is FSpec FR-181's breadth-first expansion,
  which coalesces equal state keys, so the explored behaviours form a graph,
  not a tree, and listing every path through it would grow without bound on
  a cycle. Each explored state other than an initial one has a *discovery
  edge*, the edge by which the expansion first reached it, and the discovery
  edges form a tree rooted at the initial states. The item SHALL report one
  trace per leaf of that tree (a state no discovery edge leaves: a terminal
  state, a frontier state, or a state whose every successor was discovered
  earlier), running from its initial state along discovery edges to the
  leaf, in the leaves' discovery order. Every explored state lies on at
  least one reported trace and each leaf ends exactly one, so the traces
  cover the exploration with no more traces than states, and each is a real
  behaviour prefix that replays.
- **Sample.** The item SHALL report one trace per sample, `samples` traces
  in trace-index order `0` to `samples - 1`, each named by the echoed seed
  and its trace index and drawn by FSpec FR-181's sampler, ending at a
  terminal state or after `max_steps` steps.
- **Cancellation** returns `incomplete` with cause `cancelled` and the
  frontier at the time of cancellation.
- **Refusals.** A subject that does not admit refuses the whole request
  (FR-624); `seed` with `enumerate` refuses `invalid-request` naming it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-634-AC-1 | Enumerating the Counter subject gives `success`, `coverage.states` 4, `coverage.transitions` 3 and `coverage.choices` 0, exactly one trace, from `value = 0` to `value = 3`, and an empty frontier. | Test (TC-1594) |
| FR-634-AC-2 | `max_states` 2 gives `incomplete` `LimitReached{max_states}` with a non-empty frontier. | Test (TC-1594) |
| FR-634-AC-3 | Two `sample` runs with seed 5 and `samples` 3 give equal documents. | Test (TC-1594) |
| FR-634-AC-4 | Enumerating the staying Counter (FR-627's `setTo` self-loop at `value = 0`) gives `success`, `coverage.states` 4, `coverage.transitions` 4 and `coverage.choices` 1, and exactly one trace, `value` 0, 1, 2, 3, since the self-loop's edge reaches a state already discovered. | Test (TC-2293) |
| FR-634-AC-5 | A `sample` run with seed 5 and `samples` 3 over the staying Counter reports three traces with trace indices 0, 1 and 2 in that order, each starting at `value = 0`, the item's `seed` 5, `coverage.states` equal to the number of distinct states on the three reported traces, and `coverage.transitions` equal to the number of distinct (pre-state, transition, post-state) steps on them. | Test (TC-2293) |

## Dependencies

- FR-120 owns the simulator; FSpec FR-181 the exploration contract.
