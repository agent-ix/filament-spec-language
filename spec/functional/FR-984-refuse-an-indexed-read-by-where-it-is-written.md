---
id: FR-984
title: "Refuse an indexed read by where it is written"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-612
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-171
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-501
    type: depends_on
---
# FR-984: Refuse an indexed read by where it is written

## Description

FSL SHALL admit an indexed read `e @ t`, a step label and `stutter` in the
body of a `hyper` clause, refuse them in any other temporal clause at S3 with
`unsupported_construct`/`expression-form` (exit 21), and refuse `@` outside
every temporal and hyper condition at S1 with
`invalid_syntax`/`unexpected-token` at the `@` (exit 20).

## Rationale

The standard settles the stage by position (FSpec FR-501 item 2). Inside a
temporal clause the `holds(...)` and guard expressions use one expression
grammar, so the parser reads the form and the checker, which knows whether
the clause binds trace variables, refuses it with a message that says to use
a `hyper` clause (FR-612). Outside temporal and hyper clauses there is no
trace position at all, so `@` is not part of the expression grammar there and
the parser refuses it. The two refusals therefore have different stages,
categories and exit codes, and each names what the author must change.

## Inputs

A complete-V1 unit holding `@`, a step label or `stutter`.

## Outputs

A parsed and checked hyper clause, or one located refusal per this table.

## Behavior

| Where the form is written | Stage | Code / cause | Locus | Category, exit |
|---|---|---|---|---|
| `holds(...)` or `on each .. when` guard of a `hyper` clause over `behaviours` | admitted | none | none | none |
| `holds(...)` or `on each .. when` guard of any other `temporal` clause | S3 (FR-612) | `unsupported_construct`/`expression-form`, message naming `hyper` | the first such form | `unsupported`, 21 |
| a function body, a state clause, a refinement row | S1 | `invalid_syntax`/`unexpected-token` | the `@` | `refusal`, 20 |

- The `parse` and `check` outcome documents (FR-286) SHALL carry the row's
  code, cause, locus, category and exit code.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-984-AC-1 | The `check` outcome document of a unit holding `record R { l: Integer; }` and `temporal T using c over (v: R) on origin { always holds(v.l @ a) }` under the infinite-trace profile has no S1 diagnostic, category `unsupported`, exit 21, and one diagnostic `unsupported_construct`/`expression-form` located at `v.l @ a` whose message contains `hyper`. | Test (TC-2780) |
| FR-984-AC-2 | The `parse` outcome document of a unit holding `function f using v(x: Integer): Integer pure { x @ a }` has category `refusal`, exit 20, and one diagnostic `invalid_syntax`/`unexpected-token` located at the `@`. | Test (TC-2780) |
| FR-984-AC-3 | `hyper NI using inf over behaviours (v: V::Vault) forall trace a of V forall trace b of V { always holds(v.l @ a = v.l @ b) }` parses with two `IndexedRead` nodes and no diagnostic. | Test (TC-2780) |

## Dependencies

- FR-612 owns the S3 refusal; FR-171 the hyper clause forms; FR-286 the
  outcome document; FSpec FR-501 item 2 the rule.
