---
id: FR-898
title: "Check and decide a hyperproperty over a declared finite trace set"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-021
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-171
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-631
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-191
    type: implements
  - target: ix://agent-ix/filament-specification/FR-739
    type: implements
  - target: ix://agent-ix/filament-specification/FR-402
    type: depends_on
---
# FR-898: Check and decide a hyperproperty over a declared finite trace set

## Description

FSL SHALL check, write and decide a `hyper` clause over a declared finite
trace domain,
`hyper H using p over t in Q bounded N forall trace a exists trace b { body }`
(FSpec FR-191's finite reading, ADR-023 HS-1). S3 SHALL NOT refuse the form
as `FiniteDomain`.

**S3.** Each check below refuses at the span it names:

| Check | Refusal |
| --- | --- |
| `Q` names a record type, declared in the unit or exported by a selected model, which is the state record of each trace (an FR-610 record trace subject) | `missing_declaration`/`missing-name` for no declaration, or `invalid_model_binding`/`wrong-export` for another kind, required `record`, at `Q` |
| `N` is at least 1 | `invalid_package`/`invalid-value` at `N` |
| The `using` profile is a finite-trace profile (event-position, fixed-sample or timestamped-event) | `ill_typed`/`type-mismatch` at the alias, expected `a finite-trace profile` |
| No quantifier has `of` | `unsupported_construct`/`expression-form` at the quantifier, as FR-172 already refuses it |
| Quantifier variables are distinct | `ambiguous_declaration`/`ambiguous-name` at each |
| The body reads only quantified variables | `missing_declaration`/`missing-name` at the read |
| The body is Boolean | `ill_typed`/`non-boolean-root` at the body |

The body is a formula of the clause's profile over indexed atoms `a.f`, the
field `f` of trace `a`'s record at the current joint position, with
`f` a field of `Q`.

**S4** SHALL write the clause as FSpec FR-402's hyperproperty node, with its
finite domain: the population's record type, the bound `N` and the
quantifier prefix in source order.

**Request.** A `hyper` item (FR-631) over the clause SHALL carry `traces`:
an FSpec FR-739 trace population document. That document holds the
population's identity, its closure (`closed` or `open`) and the traces,
each with a trace identity and a finite sequence of `Q` records. The
population is refused `invalid_runtime_input`/`invalid-value` naming the
trace when a record does not conform to `Q`, or when two traces share an
identity with different records. A population of more than `N` traces
settles `requires-bound` naming `N` (FSpec FR-191-AC-3).

**Evaluation.** Quantifiers are evaluated left to right over the
population's trace identities. Each variable ranges independently over every
identity, so one trace may fill two variables. Traces are aligned in
lockstep: joint position `j` is position `j` of each trace. A read of trace
`a` at a position past `a`'s last record is an absent alignment, so that
tuple's value is incomplete and is never read as `false` (FSpec
FR-191-AC-5).

**Result.**

| Label | When |
| --- | --- |
| `proved` | the prefix holds over a `closed` population with every needed tuple decided |
| `refuted` | a decisive tuple violates it: a universal tuple for which every existential completion is decided false. The counterexample names the universal tuple's trace identities and, for each existential completion, the joint position at which the body is false |
| `inconclusive` | the population is `open`, or a needed tuple is incomplete and no decisive tuple exists |

**Replay.** `fsl-replay` reproduces a refutation from the population
document and the counterexample alone, by re-evaluating the body over the
named traces. A counterexample whose trace identities or positions are
edited does not reproduce.

### Decision: the trace domain is a record type, and the traces are request data

A finite trace set is evidence: recorded runs, test executions, or logged
sessions. It changes from one assessment to the next while the property
stays the same, so the clause names only the shape of a trace (the record
type `Q`) and its size bound `N`. The traces arrive with the request, as a
supplied trace does for an offline monitor (FR-283). Lockstep is the one
alignment that needs no further declaration. A read past a trace's end
stays incomplete, because FSpec FR-191 forbids comparing unrelated positions.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-898-AC-1 | `hyper Det using ep over t in Obs bounded 3 forall trace a forall trace b { (a.in = b.in) -> (a.out = b.out) }`, `Obs` a record `{in: Int[0, 3]; out: Int[0, 3]}`, checks and compiles to a hyperproperty node carrying `Obs`, bound 3 and the prefix. | Test (TC-2534) |
| FR-898-AC-2 | Refusals, each at its span: `Q` naming no declaration; `Q` naming an object type; `bounded 0`; an infinite-trace profile; two quantifiers named `a`; a body reading `c`, which is not quantified; and a non-Boolean body. | Test (TC-2534) |
| FR-898-AC-3 | Over a closed population of three traces in which equal `in` values always give equal `out` values, `Det` settles `proved`. With a third trace whose `out` differs at position 1, it settles `refuted`, naming the two traces and joint position 1. | Test (TC-2535) |
| FR-898-AC-4 | `forall trace a exists trace b { a.out = b.in }` and `exists trace b forall trace a { a.out = b.in }` over one population give different results, as FSpec FR-191-AC-2 requires. A population of four traces under `bounded 3` settles `requires-bound` naming 3. An open population settles `inconclusive`. | Test (TC-2535) |
| FR-898-AC-5 | A body that reads `a` at position 2 when trace `a` has two records leaves that tuple incomplete: the clause settles `inconclusive`, unless another tuple refutes it. | Test (TC-2535) |
| FR-898-AC-6 | `fsl-replay` reproduces AC-3's refutation from the population and the counterexample. The counterexample with one trace identity swapped, or the joint position changed, does not reproduce. | Test (TC-2535) |

## Dependencies

- ADR-023 HS-1: the two domain forms.
- [FR-171](FR-171-build-forms-for-hyper-clauses-over-behaviours.md) and
  [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md):
  hyper forms and their S3 checks.
- [FR-631](FR-631-request-hyperproperty-items.md): the hyper item.
- FSpec FR-191, FR-402 and FR-739.
