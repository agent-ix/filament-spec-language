---
id: FR-575
title: "Answer unsupported for constructs outside the bounded-static profile"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-570
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-571
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-573
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-174
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-175
    type: depends_on
---
# FR-575: Answer unsupported for constructs outside the bounded-static profile

## Description

FSL SHALL admit two projection and realizability profiles. Under `bounded`
it SHALL decide every construct a checked protocol holds (FR-890, FR-891,
ADR-036 §7). Under `bounded-static` it SHALL decide only the fragment below,
and SHALL answer `unsupported` for every construct outside it, naming a
located construct cause under the existing `unsupported_projection` code,
with no substitute graph and no guess. It implements ADR-036 PA-21, PA-22
and PA-26 and FSpec FR-174-AC-3, and fixes the open item that ADR-036 left
to the requirements: the profile words and the cause slugs.

### Decision: the fragment stays selectable, and nothing is out of reach

A consumer that built its corpora against the fragment keeps a request
word whose answers do not change. A consumer that needs loops, every join
policy, replicated roles, per-trigger activation, symbolic capacity,
compensation or timed awaits names `bounded`, and gets a decision for each
of them. No construct is unavailable to a corpus: the slugs below are the
answer to a request that asked for the fragment, and their remedy is the
`bounded` profile.

## Inputs

A projection or realizability request (FR-570, FR-571) with a profile and a
checked protocol.

## Outputs

An `unsupported` result (FR-573) carrying `unsupported_projection` with one
cause slug and the checked node id of the construct; or, for a profile word
other than `bounded` or `bounded-static`, a refusal.

## Behavior

The fragment profile is `bounded-static`: activation `on origin`, static roles,
literal queue capacities, sequence, send, receive, `choice`, and `parallel`
with `join all`. The
causes, each a slug of `unsupported_projection` in the style of
`unsupported-requested-capability` and `unbounded-extent` (FR-057), are:

| Slug | Construct |
| --- | --- |
| `activation-on-each` | A protocol activated `on each` trigger: an instance per trigger, whose roles' steps would share one observation. |
| `dynamic-role-population` | A role with a dynamic population. |
| `symbolic-queue-capacity` | A queue with a symbolic capacity. |
| `unsupported-join` | A `join` with policy `any`, `quorum` or `predicate`, or an `outstanding` policy with no rule; the policy is named. |
| `repeat` | A `repeat` construct. |
| `compensation` | A compensation. |
| `timed-await` | A timed await. |
| `unsupported-control` | A `check`, `attempt`, `effect`, `event`, `commit` or `fence` control, which the profile's list does not include. |
| `evaluator-unsupported` | Realizability only: a construct `fsl-eval` refuses to step; the `fsl-eval` cause is carried. |

When a protocol holds several such constructs the engine reports the first in
the order activation, roles, queues, compensations, then the other constructs in layout
order; `unsupported-control` is reported only when no other construct is found.
FSpec FR-466 lets a `protocol` node own a choice, but the FSL checker refuses
such a choice (`NodeRole`) before any analysis, so the profile states no cause
for a choice the protocol assigns to no role.

Names are chosen as the construct's own name, kebab case, one slug per
construct, so a corpus author reads the construct from the slug; `join any`,
`quorum`, `predicate` and `outstanding` share `unsupported-join` because the
remedy is the same (a `join all`) and the named policy distinguishes them.

- FSL shall refuse a profile word other than `bounded` or `bounded-static` at admission,
  before any analysis, `invalid_runtime_input`/`invalid-value` naming the
  member `profile`; it is a request value, not a construct, so it is not an
  `unsupported` result.
- The engine shall never answer a located unsupported construct with a
  projected graph, a strategy, a sampled result or a guessed one (PA-22).
- Under `bounded-static`, FSL shall answer `repeat` and non-`all` joins under
  projection and realizability with the causes above; a corpus that needs a retry
  loop or such a join names `bounded`, which decides it (FR-890, FR-891).
- When a construct is both in a row above and refused by `fsl-eval`, the
  engine shall report that row's own slug; `evaluator-unsupported` is
  reported only for a `fsl-eval` cause that has no row above.
- Under `bounded`, the only `unsupported` answer is `evaluator-unsupported`,
  for the two `fsl-eval` limits `UnenumerableRecord` and `WeakMemory`
  (FR-882, ADR-036 PA-33); no row above arises.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-575-AC-1 | For each construct row other than `evaluator-unsupported`, a protocol using it yields `unsupported`, `unsupported_projection` with exactly that slug and the construct's checked node id, for projection and for realizability, and returns no local graph, strategy or counterexample (implements FR-174-AC-3). | Test (TC-1415) |
| FR-575-AC-2 | A construct `fsl-eval` refuses (an unenumerable step binder, FR-882) yields `evaluator-unsupported` for realizability carrying the `fsl-eval` cause, except that a refused construct that has a row above (a symbolic capacity, `activation on each`, compensation) reports that row's slug. | Test (TC-1415) |
| FR-575-AC-3 | A request with profile `bounded-static` over a protocol using only the profile's constructs is decided, a request with profile `bounded` over the same protocol gives the same result in every member but the key's profile, and a request with any other profile word is refused `invalid_runtime_input`/`invalid-value` naming `profile` before any analysis. | Test (TC-1415) |
| FR-575-AC-4 | A `NegotiationCause` has exactly its three existing variants and a kind with no candidate still negotiates `unsupported_projection`/`unsupported-requested-capability`; the slugs above arise only from an engine result and add no code family. | Test (TC-1415) |
| FR-575-AC-5 | For each construct row other than `evaluator-unsupported`, the protocol of AC-1 under profile `bounded` yields no `unsupported` result for projection or realizability; a protocol whose binder type has a `String` field and one whose resolved memory model the system does not implement each yield `evaluator-unsupported` under `bounded`, carrying `UnenumerableRecord` and `WeakMemory`. | Test (TC-2520) |

## Dependencies

- ADR-036 PA-4, PA-21, PA-22, PA-26 and PA-33.
- [FR-890](FR-890-project-every-protocol-construct-under-the-bounded-profile.md) and [FR-891](FR-891-decide-realizability-over-every-protocol-construct-under-the-bounded-profile.md): the `bounded` profile.
- [FR-057](FR-057-admit-shared-capability-kinds.md): `unsupported_projection` and its slugs.
- [FR-570](FR-570-project-a-global-protocol-onto-one-role.md), [FR-571](FR-571-decide-whether-local-strategies-realize-a-global-protocol.md), [FR-573](FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md).
- FSpec FR-174, FR-175.

## References

- ADR-036; QSL-1889, QSL-1893.
