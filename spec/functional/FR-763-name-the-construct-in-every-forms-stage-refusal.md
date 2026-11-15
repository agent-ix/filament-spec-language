---
id: FR-763
title: "Name the construct in every forms-stage refusal"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-341
    type: references
  - target: ix://agent-ix/filament-specification/FR-272
    type: depends_on
---
# FR-763: Name the construct in every forms-stage refusal

## Description

When S2 refuses a unit because no form represents a construct it holds
(`FormsCause::UnrepresentedConstruct`) or because no dispatch entry reads a
declaration (`FormsCause::NoDispatchEntry`), the refusal SHALL carry FSpec
FR-272's typed cause `unsupported_construct`/`not-yet-implemented` with a
payload that names the construct, and its message SHALL name the construct
in the author's own spelling.

## Rationale

These constructs are valid complete-V1 source that FSL's forms stage does not
build yet: the source is not wrong, and FSpec FR-272 gives exactly that case
the cause `not-yet-implemented`, retaining the unbuilt feature's name, its
owning declaration and the selected profile. A refusal that says only "no form
represents this construct" gives the author nothing to find, and a consumer
nothing to switch on. With the construct's kind and text in the payload, the
author sees which line to change and a tool can group refusals by feature.

## Inputs

An S1 output that S2 refuses for one of the two causes above.

## Outputs

A `FormsRefusal` whose cause carries the construct payload, and, through
FR-286, a diagnostic whose `cause` is `not-yet-implemented` and whose payload
members are below.

## Behavior

### The payload

- `FormsCause::UnrepresentedConstruct` and `FormsCause::NoDispatchEntry`
  SHALL each carry `construct`, a `ConstructKind` from the closed vocabulary
  below; `text`, the construct's exact source text; `declaration`, the
  declared name of the declaration that holds the construct, or `null` for a
  declaration with no name; and `profile`, the identity of the profile the
  owning declaration's `using` alias selects, or `null` when it has none.
- `text` SHALL be the source bytes of the construct's span. When they exceed
  120 bytes, `text` SHALL be the longest prefix of at most 120 bytes that ends
  on a UTF-8 character boundary, followed by `...`.
- Both causes SHALL map to the catalog code `unsupported_construct` and the
  cause tag `not-yet-implemented` (FR-091 "Catalog codes"). No forms-stage
  refusal of these two causes has a null cause.
- FR-286 SHALL serialize the payload as the diagnostic members `construct`
  (the kind's spelling), `text`, `declaration` and `profile`, beside `code`,
  `cause` and `locus`. The locus is the construct's span.

### Construct kinds

| `construct` | Where S2 refuses it | `text` holds | Message |
| --- | --- | --- | --- |
| `null-value` | `null` as a primary outside a record field value | `null` | `` `null` is not supported as a value here yet `` |
| `index-selection` | `e[i]` that does not directly follow a field or selector of a clock place expression | the indexing expression, such as `s[i]` | `` the index selection `<text>` is not supported here yet `` |
| `size-of-type` | `size<T>(e)` | the whole call | `` `size<T>` is not supported in expressions yet `` with `T` as written |
| `qualified-reaches-edge` | `reaches(a, b, e)` whose edge `e` has more than one segment | the edge name | `` `reaches` over the qualified edge `<text>` is not supported yet `` |
| `hyper-condition` | a hyper expression as a temporal condition | the hyper expression | `` a hyper expression is not supported as a temporal condition yet `` |
| `declaration` | a declaration whose leading keyword has no dispatch entry, such as `synthesis` or `verify` | the leading keyword | `` `<text>` declarations are not supported yet `` |

- The message SHALL be the row's sentence with the placeholders filled from
  `text`, followed by ` (declaration <declaration>)` when `declaration` is not
  `null`. The message begins with the construct, so a reader scanning a list
  of refusals sees which construct each one is.
- A construct that S2 later learns to build leaves this table with it; a new
  site that refuses with either cause adds a row. Every row's kind is a
  `ConstructKind` variant, and `ConstructKind` has no variant without a row.

### Gate classification

- FR-341 classifies these refusals by their cause, `not-yet-implemented`,
  under its rule 5 (`unsupported`, an FSL implementation gap), not as a
  profile prohibition.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-763-AC-1 | S2 over a unit holding a `synthesis` declaration, and over one holding a `verify` declaration, refuses with `NoDispatchEntry` whose `construct` is `declaration`, `text` the keyword, `declaration` the declared name and `profile` the `using` profile identity; the cause's catalog code is `unsupported_construct` and its tag `not-yet-implemented`. | Test (TC-2155) |
| FR-763-AC-2 | S2 over a function body `null` refuses `UnrepresentedConstruct` with `construct` `null-value`, `text` `null`, the function's name and profile, and the span of `null`. | Test (TC-2156) |
| FR-763-AC-3 | S2 over a function body `size<Integer>(xs)` refuses with `construct` `size-of-type` and `text` `size<Integer>(xs)`; over a function body `xs[0]` it refuses with `construct` `index-selection` and `text` `xs[0]`. | Test (TC-2157) |
| FR-763-AC-4 | S2 over a state clause `reaches(self, self, Config::ConfigVersion::parent)` refuses with `construct` `qualified-reaches-edge` and `text` `Config::ConfigVersion::parent`, at the edge's span. | Test (TC-2158) |
| FR-763-AC-5 | For every `ConstructKind` variant, the refusal's message begins with the row's sentence filled from `text`; a construct whose source text is 200 bytes long carries a `text` of at most 123 bytes that ends with `...`. No refusal of either cause has an empty `text`. | Test (TC-2159) |
| FR-763-AC-6 | The `parse` outcome document of AC-2's unit holds one diagnostic with `code` `unsupported_construct`, `cause` `not-yet-implemented` (not `null`), `construct` `null-value`, `text` `null`, `declaration` the function's name and `profile` its profile identity, and the message of AC-5; the document's category is `unsupported`, exit 21. | Test (TC-2160) |

## Dependencies

- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md): the forms stage, its causes and their catalog codes.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the diagnostic members.
- [FR-341](FR-341-classify-a-spine-compile-result-for-a-refinement-gate.md): gate classification.
- FSpec FR-272: `unsupported_construct`/`not-yet-implemented` and its retained payload.
