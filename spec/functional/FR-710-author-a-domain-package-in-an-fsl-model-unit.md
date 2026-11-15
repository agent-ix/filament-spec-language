---
id: FR-710
title: "Author a domain package in an FSL model unit"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-002
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-208
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-152
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-610
    type: depends_on
---
# FR-710: Author a domain package in an FSL model unit

## Description

When an author writes a model unit, the compiler SHALL compile it to one
domain package document in the form FSpec FR-154 admits (an FCD semantic IR
2.0.0 domain package, serialized with RFC 8785), with every node, member,
`origin`, multiplicity and `constructs` entry that admission requires, and
SHALL let a package input supply the model unit in place of the document
under its identity.

A model unit is the language's own route to a domain model. It declares
everything a domain package holds: object types with fields, identity,
supertypes, relationships, operations, frames and contract clauses; value,
record, tuple, variant, option and map types; event types, state machines,
processes, persistence interfaces and namespaces; populations; dimensions and
units; and the systems Interface, Part, Port, Connection and Allocation
constructs (FR-208). A newcomer writes a model in the same notation as the
rules over it, in a few lines, and never writes or generates the JSON
document by hand. The document stays the one interchange form: model intake
(FR-056, FSpec FR-154) admits the compiled document exactly as it admits a
document lifted from spec artifacts, and nothing downstream knows which
route produced it.

## Inputs

- A model unit: UTF-8 source in the grammar below, with its source identity.
- Optionally, the identity a package input supplies it under.

## Outputs

- One domain package document whose `package` is the unit's `domain`
  identity, and whose bytes are the RFC 8785 serialization of that document.
- Or located refusals at the model unit's source spans and no document.

## Behavior

### Grammar

A model unit is written in the shared-grammar notation; `header`, `field`,
`type-ref`, `parameter`, `block`, `frame-clause`, `population-member`,
`record-decl`, `enum-decl`, `tuple-decl`, `union-decl`, `alias-decl`,
`dimension-decl` and `unit-decl` are the existing productions, and every
grammar word below is a reserved spelling of a model unit.

```ebnf
model-unit       = header, 'domain', string, ';', { domain-decl }, EOF ;
domain-decl      = domain-object | record-decl | enum-decl | tuple-decl
                 | union-decl | alias-decl | map-decl | dimension-decl
                 | unit-decl | event-decl | machine-decl | process-decl
                 | store-decl | namespace-decl | population-decl
                 | interface-decl | part-decl | port-decl
                 | connection-decl | allocation-decl ;
domain-object    = [ 'abstract' ], 'object', ident,
                   [ 'extends', ident, { ',', ident } ],
                   [ 'identity', '(', ident, { ',', ident }, ')' ],
                   '{', { object-body }, '}' ;
object-body      = domain-field | relationship | domain-operation
                 | invariant ;
domain-field     = ident, ':', type-ref, [ '?' ], [ multiplicity ],
                   [ 'subsets', ident ], [ 'redefines', ident, '::', ident ],
                   ';' ;
multiplicity     = '[', uint, ',', ( uint | '*' ), ']',
                   [ 'ordered' ], [ 'unique' ] ;
relationship     = 'relationship', ident, ':', ident, multiplicity,
                   [ 'inverse', ident, multiplicity ], ';' ;
domain-operation = 'operation', ident,
                   '(', [ parameter, { ',', parameter } ], ')',
                   [ ':', type-ref ],
                   [ 'redefines', ident, '::', ident ],
                   { frame-clause },
                   ( ';' | '{', { contract }, '}' ) ;
contract         = ( 'pre' | 'post' ), ident, block, ';' ;
invariant        = 'invariant', ident, block, ';' ;
map-decl         = 'map', ident, '=', 'Map', '<', type-ref, ',', type-ref,
                   '>', '[', uint, ',', ( uint | '*' ), ']', ';' ;
event-decl       = 'event', ident, '{', { domain-field | invariant },
                   'occurs', ident, ';', '}' ;
machine-decl     = 'machine', ident, [ 'extends', ident, { ',', ident } ],
                   '{', { domain-field | domain-operation | invariant },
                   'states', '(', ident, { ',', ident }, ')', ';',
                   { transition }, '}' ;
transition       = 'transition', ident, '->', ident, 'on', ident,
                   [ 'when', block ], [ 'emits', ident, { ',', ident } ], ';' ;
process-decl     = 'process', ident, [ 'extends', ident, { ',', ident } ],
                   '{', { domain-field | domain-operation | invariant },
                   { step }, '}' ;
step             = 'step', ident, ':', ident,
                   [ 'consumes', ident, { ',', ident } ],
                   [ 'emits', ident, { ',', ident } ], ';' ;
store-decl       = 'store', ident, 'persists', ident, { ',', ident },
                   '{', { domain-operation }, '}' ;
namespace-decl   = 'namespace', ident, '{',
                   [ 'members', ident, { ',', ident }, ';' ],
                   { 'term', ident, '=', string, ';' }, '}' ;
population-decl  = [ 'open' ], [ 'symmetric' ], 'population', ident, ':',
                   ident, { ',', ident }, ';' ;
interface-decl   = 'interface', ident, '{',
                   { domain-field | domain-operation }, '}' ;
part-decl        = 'part', ident, ':', ident, multiplicity, 'in', ident, ';' ;
port-decl        = 'port', ident, ':', ident, multiplicity,
                   ( 'in' | 'out' | 'inout' ), 'on', ident, ';' ;
connection-decl  = 'connection', ident, ident, '.', ident,
                   ( '->' | '<->' ), ident, '.', ident, ';' ;
allocation-decl  = 'allocate', ident, 'from', ident, [ '::', ident ],
                   'to', ident, ';' ;
```

S1 SHALL parse a model unit through the same entry point as any FSL source,
recognizing it by the `domain` declaration that follows its header, and SHALL
admit its concrete syntax tree under the same limits, rendering and
diagnostic rules (FR-001). A complete-V1 unit and a model unit never share a
source: `domain` after a `profile` or `model` declaration is
`invalid_syntax`/`unexpected-token` at `domain`.

A model unit names only its own declarations and native types: a type name
in a model unit is one segment. Clause bodies (`block`) are FSL expressions
over `self`, the operation's parameters, `result` and `pre(...)`, exactly as
a state clause on the declaring type reads them (FR-104).

### Compilation

The compiler SHALL map each declaration to the document node FSpec FR-154
admits for it, under the FR-208 meaning in this table, and SHALL write one
`constructs` entry for each kind it uses and none for a kind it does not use:

| Declaration | FR-208 meaning |
| --- | --- |
| `object` (with `relationship` members, a relationship of the document) | `filament.meaning.model.object-type/v1` |
| `type` alias of a native type | `filament.meaning.model.value-type/v1` |
| `record` | `filament.meaning.model.record-value-type/v1` |
| `tuple` | `filament.meaning.model.tuple-value-type/v1` |
| `enum`, `union` | `filament.meaning.model.variant-type/v1` |
| `event` | `filament.meaning.model.event-type/v1` |
| `machine` | `filament.meaning.model.state-machine/v1` |
| `process` | `filament.meaning.model.process/v1` |
| `store` | `filament.meaning.model.persistence-interface/v1` |
| `namespace` | `filament.meaning.model.namespace/v1` |
| `population` | `filament.meaning.model.population/v1` |
| `interface` | `filament.meaning.systems.interface/v1` |
| `part`, `port`, `connection`, `allocate` | `filament.meaning.systems.part/v1`, `port/v1`, `connection/v1`, `allocation/v1` |

`Option<T>` field types and `map` declarations become the document's option
and map value types, and `dimension` and `unit` its dimension and unit nodes.
Each declaration's IR node identity is `ix://<domain identity>/<name>`, each
member's `<owner identity>/<member name>`, and each relationship's
`ix://<domain identity>/relationship/<Owner>-<name>-<Target>`. Every type,
field, operation, parameter and enum case carries an `origin` naming the model
unit's source identity and the span of its declaration. Every field carries
an explicit multiplicity: `[1, 1]` when none is written, the presence
`optional` when `?` is written, and the written bounds and flags otherwise.
Every `pre`, `post` and `invariant` carries the clause language that FR-056
admits for an FSL clause and the block's expression text. A population
without `open` has extent `closed`. The document's `contractVersion` is
`2.0.0` and its `package` is the `domain` string.

The compiled bytes SHALL be a function of the model unit's source bytes and
source identity alone: compiling one source twice gives equal bytes.

### Refusals

The compiler SHALL check the model unit before it writes a document, and SHALL
refuse at the model unit's span, never at a document pointer:

- a model unit that does not parse: the `invalid_syntax` diagnostic at the
  offending token, as for any FSL source;
- a name that names no declaration of the unit (a supertype, field type,
  relationship target, frame entry, population member, transition state,
  trigger or emitted event, step event, `persists` entry, namespace member,
  part, port or allocation end): `missing_declaration`/`missing-name` at the
  name;
- two declarations, or two members of one declaration, sharing a name:
  `ambiguous_declaration`/`ambiguous-name` at the second;
- a `domain` identity that is reserved under FSpec FR-154 (`filament/native`,
  `filament/unit` or beneath them): `invalid_model_binding`/
  `malformed-declaration` at the string;
- a declaration FSpec FR-154 would refuse for its meaning (for example an
  operation on a `record`, a `supertypes` entry of another meaning, a map
  whose `keys` is not a scalar key type): the code and cause FR-154 gives for
  that node, at the declaration's span.

A document the compiler writes SHALL be admitted by FR-154 with no refusal;
an FR-154 refusal of a compiled document is a compiler defect, not a refusal
the author sees.

### Package input

A package input MAY supply, under a domain package identity, a model unit in
place of a document. The compiler SHALL compile it under this requirement,
and SHALL then admit the compiled document by FR-056 and FSpec FR-154 exactly
as a supplied document, so a model unit whose `domain` string differs from the
selection's identity is refused `invalid_model_binding`/`wrong-model-selection`
as FR-154 refuses a document whose identity differs. The command-line
`compile` and `run` requests and the library API accept the model unit the
same way, and the library API also returns the compiled document bytes so an
author can publish the document itself.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-710-AC-1 | The model unit `domain "example/shop"; object Order identity (id) { id: Int[0, 999]; total: Int[0, 100]; operation pay(): Boolean modifies self.total; } population orders: Order;` parses, compiles to a document that FSpec FR-154 admits with no refusal, with one object type `ix://example/shop/Order` (fields `id`, `total` with multiplicity `[1, 1]` and an `origin` each), one operation `pay` with its frame, one closed population `orders` and exactly the `constructs` entries for the object type and population kinds. A unit with `model M = "example/shop";` supplied that model unit checks `invariant I using P on M::Order at current { self.total >= 0 }`. Compiling the model unit twice gives equal bytes. | Test (TC-1942) |
| FR-710-AC-2 | A model unit with `object Node { relationship next: Node [0, 1]; }` and `population nodes: Node;` compiles to a document whose `Node` declares the relationship `ix://example/graph/relationship/Node-next-Node` with its ends and multiplicity, FR-154 admits it, and `reaches(a, b, next)` checks and evaluates over a snapshot of `nodes`. | Test (TC-1942) |
| FR-710-AC-3 | One model unit holding one declaration of every form in the grammar (object with relationship, contract clauses and invariant; alias, record, tuple, enum, union, option field and map; event; machine with states and a transition; process with a step; store; namespace; open and closed populations; dimension and unit; interface, part, port, connection and allocation) compiles to a document that FR-154 admits, with one node of each FR-208 meaning in the compilation table. | Test (TC-1942) |
| FR-710-AC-4 | Each refuses with no document, at the model unit's span: `domain` with no string `invalid_syntax`/`unexpected-token` at `;`; `domain` after a `profile` declaration `invalid_syntax`/`unexpected-token` at `domain`; `relationship next: Ghost [0, 1];` `missing_declaration`/`missing-name` at `Ghost`; two objects `Order` `ambiguous_declaration`/`ambiguous-name` at the second; `domain "filament/native/x";` `invalid_model_binding`/`malformed-declaration` at the string; an operation on a `record` with FR-154's `unsupported_construct`/`declaration-form` at the operation. | Test (TC-1943) |
| FR-710-AC-5 | A package input that supplies the AC-1 model unit under `example/shop` for `model M = "example/shop";` admits it as FR-154 admits the compiled document, and the checked package equals the one built from the compiled document supplied as bytes; supplied under `example/other` for `model M = "example/other";` it refuses `invalid_model_binding`/`wrong-model-selection`. | Test (TC-1943) |

## Dependencies

- [FR-056](FR-056-admit-domain-package-model-declarations.md): model
  intake of the compiled document.
- [FR-569](FR-569-declare-object-types-populations-and-operations-in-fsl-source.md):
  the `object` notation a model unit shares with a unit's source objects.
- [FR-104](FR-104-check-state-clauses.md): how clause bodies read `self`,
  parameters and `result`.
- FSpec FR-154, FR-208 and FR-152: the domain package document, its meanings
  and the systems constructs.
