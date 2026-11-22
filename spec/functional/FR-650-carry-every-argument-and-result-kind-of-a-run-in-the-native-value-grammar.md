---
id: FR-650
title: "Carry every argument and result kind of a run in the native value grammar"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-014
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-360
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-038
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-089
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-034
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-144
    type: references
  - target: ix://agent-ix/filament-specification/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-154
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-261
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: references
---
# FR-650: Carry every argument and result kind of a run in the native value grammar

## Description

A public run request supplies function arguments as JSON, and a public run
result reports the value a function returned as JSON. Both SHALL use FSpec
FR-181's tagged canonical values under "Canonical command boundary" below;
a Population argument retains the separate `{"population":P}` selection.
Every value of every declared type that has an FSpec FR-181 typed canonical
form can be passed as an argument and returned as a result. A completed
value can be passed back unchanged as an argument of a later run, so a
caller can chain runs, and consumers of the standard envelope (FSpec
FR-550) and an FSL public run document read the same values.

The **native run value form** below is FR-106's closed value grammar with
tuple, union-payload, identity-reference and population forms added. It is
the internal admission representation and the contract of the separately
exposed spine and clause readers. It SHALL NOT be accepted as an alternative
to canonical values at the public command boundary. Its correspondence to
FSpec FR-181 remains ADR-034 CS-2's one-to-one mapping.

The public canonical boundary applies to:

- each `value` of `call.arguments` in a native-run/1 request (FR-100);
- each `value` of the `arguments` of FR-312's `function` selection and of
  FR-651's `call` selection;
- the `value` of a completed `outcome` in `spine-run-result/1` (FR-100), and
  the `value` of a completed `execute` result in `filament-outcome/1`
  (FR-286).

## Inputs

- A JSON value at an argument position, and the declared type of the
  parameter it binds, from the compiled package.
- For a Reference or Population argument, the call's object world: the
  current snapshot the call names (`snapshot`, below).
- A completed S6a value and the selected function's declared result type.

## Outputs

- An admitted `filament_exact::Value` per parameter, or one refusal.
- The run value form of a completed result.

## Behavior

### Canonical command boundary

The public `fsl_command::command::run` family SHALL read function-call
arguments and write completed call values in FSpec FR-181's tagged canonical
form. Integers carry `{"type":"integer","value":"5"}`, Booleans
carry `{"type":"boolean","value":true}`, Text carries
`{"type":"text","value":"hello"}`, and collections carry their
`type` and `elements`. The command SHALL refuse the untagged native forms
and bare integer shorthand; it SHALL NOT retry them through another reader.

This boundary SHALL retain the declared type's existing admission: exact
integers have no machine-integer ceiling, rational `4/6` admits as `2/3`,
and an unordered supplied set is admitted before its canonical writer
orders its elements. Unknown members, domain and cardinality violations,
snapshot reference admission, and resource limits retain their existing
refusal or incomplete category. A Population argument retains its native
`{"population":P}` selection because a population has no FR-181 value
form; its returned members use the canonical set and reference forms.

Canonical reference argument conversion and execution SHALL borrow the same
supplied-value run, retaining its counters, original argument index and
preorder position. Reference catalog work and result-reference encoding SHALL
charge `work_units` on the normalization owner that admitted the actual
declaration, at `reference.catalog` or `reference.result`. Snapshot admission
SHALL retain each model's original owner; selecting a declaration SHALL NOT
substitute the first model's owner or create a fresh allowance. A denied
charge SHALL preserve its typed normalization refusal and charge point.
With no snapshot, canonical output SHALL NOT spend model-normalization work
or construct a reference catalog; its borrowed writer retains physical
allocation admission.

The native forms below remain the internal admission representation and
the contract of the separately exposed spine and clause readers. They are
not additional accepted forms at the public run command boundary. The
driver SHALL call `run_with_limits_and_overrides` (or its provider-aware
variant) so the existing verification-plan and multi-selection coordinators
receive their selections; a function-only preparation API does not execute
those coordinators.

### The run value form

For a declared type, the run value form is:

| Declared type | Form |
| --- | --- |
| `Boolean` | `{"boolean": true}` or `{"boolean": false}` |
| an integer type (`Int[lo, hi]`, `Integer`) | `{"integer": "<decimal>"}`, the exact value in FR-038's integer spelling at any magnitude: an optional leading `-`, no `+`, no leading zero, `0` for zero and never `-0` |
| `Rational`, `Rational[..]` | FR-106's Rational object `{"kind": "rational", "numerator": N, "denominator": D}` |
| `Decimal`, `Decimal[..]` | FR-106's Decimal object `{"kind": "decimal", "coefficient": C, "scale": S}` |
| `Float32`, `Float64` | FR-106's Float object `{"kind": "float32" or "float64", "bits": H}` |
| `String` | FR-106's String object `{"kind": "string", "text": T}` |
| `Text[min, max; profile]` | FR-106's Text object `{"kind": "text", "text": T}` |
| `UUID`, `Timestamp`, `Bytes`, `JsonObject` | FR-106's native payload objects |
| `Quantity<u>`, `Quantity<u, C>`, `Quantity<u>[min, max]` | FR-106's Quantity object, as "Quantity" below states |
| `Map<K, V>`, `Map<K, V>[min, max]` | `{"map": [{"key": K, "value": V}, ...]}`, as "Map" below states |
| `Option<T>` | `{"absent": {}}` or `{"present": V}` |
| `Sequence<T>`, `Set<T>`, `Bag<T>`, `OrderedSet<T>` | `{"sequence": [V, ...]}`, `{"set": [...]}`, `{"bag": [...]}`, `{"ordered-set": [...]}`, admitted under FSpec FR-144's "Supplied collection values" as FR-106 states |
| a record type | FR-106's record form `{"record": {"type": I, "fields": {...}}}` |
| a tuple type | `{"tuple": {"type": I, "components": [V, ...]}}` |
| an enumeration or variant type | FR-360's variant form `{"variant": {"type": I, "case": C}}` |
| a union case with components | `{"variant": {"type": I, "case": C, "components": [V, ...]}}` |
| `Reference<T>` | `{"reference": {"population": P, "key": K}}` or `{"reference": {"universe": U, "type": T, "object_identity": O}}` |
| a population | `{"population": P}` as an argument; as a result, the `set` form of its members' references |
| an alias | the form of its target type |

`V` is a value of the run value form for the element, field, component or
payload type. `I` names the declaration: the declared name as the unit writes
it for a declaration of the unit's own source (FR-569), and the `ix://`
declaration identity for a model declaration, by FR-106's naming rule for
populations and object types, spelled as "Identity spellings" below states;
the writer writes the same spelling admission accepts, so a written value
reads back unchanged.

The forms this requirement adds to FR-106's grammar are these:

- **Tuple.** The outer object has exactly the `tuple` tag; its payload has
  exactly `type` and `components`. `components` holds one value per declared
  component, in declaration order. FR-106's member rules apply: a missing
  payload member is `missing-member`, an extra one `unknown-member`, a
  `type` naming another declaration `wrong-value-kind`, and a component count
  other than the declared arity `invalid-value`.
- **Union payload.** A variant whose case declares components carries
  `components`, one value per declared component in declaration order; a
  case with no components carries none, so FR-360's closed form is unchanged
  for an enumeration. A missing `components` on a case that declares them is
  `missing-member`, `components` on a case that declares none
  `unknown-member`, and a wrong count `invalid-value`.
- **Identity reference.** The payload `{"universe": U, "type": T,
  "object_identity": O}` names an object by its FSpec FR-204 identity
  triple: `U` the UniverseId, `T` the most-specific EffectiveId and `O` the
  exact nonempty UTF-8 object identity, spelled as "Identity spellings"
  below states. It is the form a completed Reference result is written in,
  so a result can be passed back as an argument. The payload `{"population": P,
  "key": K}` names an object by its population and key in the call's
  snapshot, as FR-106 and FR-109 do.
- **Population argument.** `{"population": P}` names a population of the
  call's snapshot by FR-106's naming rule. FR-084 admission binds it with the
  parameter's declared maximum, and FR-089 records its identity in the call's
  object context. A population has no FSpec FR-181 form, so this form names
  a binding and carries no members.
- **Map.** A `Map<K, V>` or `Map<K, V>[min, max]` value is
  `{"map": [{"key": K, "value": V}, ...]}`, one entry object per map entry,
  each with exactly `key` and `value`, `K` a value of the run value form for
  the key type and `V` one for the value type. It is the native spelling of
  FSpec FR-181's `map` form. Admission checks the entry count against the
  declared bound before reading any entry, as FR-106 checks a collection's
  length (`cardinality_out_of_bound`/`below-minimum` or `above-maximum`);
  reads every entry in array order; refuses an entry that is not an object
  `wrong-value-kind` and a missing `key` or `value` `missing-member`, at
  `/map/<index>`, and another member `unknown-member` at
  `/map/<index>/<member>`; and refuses an entry whose key
  is the same key as an earlier entry's under FSpec FR-144's key equality
  `duplicate-member` at `/map/<index>/key`. The writer writes the entries
  ascending by the RFC 8785 bytes of each key's run value form, with a
  Decimal key normalized, as FR-106 orders a set, so a map has one written
  form whatever entry order was supplied. Any traversal of a Map value, a closure walk or a first-reference search, visits the entries' values in FSpec FR-144 canonical key order, mathematical for Int and Decimal keys; a Map key is always scalar (FSpec FR-144) and never holds a reference, as [filament-semantic-value FR-106](ix://agent-ix/filament-semantic-value/FR-106) states.
- **Quantity.** A `Quantity<u>` value, and a value of a carried quantity
  `Quantity<u, C>`, is FR-106's Quantity object `{"kind": "quantity",
  "unit": N, "magnitude": M}`. A `Quantity<u>[min, max]` (BoundedQuantity)
  value has the same form; a magnitude outside the declared inclusive
  bounds, compared exactly in the declared unit, is a value of the right kind
  outside the declared domain, `invalid-value` at `/magnitude`. `M` is read as
  FR-106 reads it (a Rational or a Decimal object, retained as the exact
  normalized rational) and is written as the FSpec FR-205 normalized Rational
  object, never a Decimal, so the written form is FSpec FR-181's one
  canonical quantity encoding. `N` is the unit name of "Identity spellings".

### Identity spellings

Each identity the run value form writes is a spelling of a structural
identity of FSpec FR-201, never a digest, a hash or a hex rendering of one:
a structural identity has no fixed-width text, and FSpec FR-201 forbids
deriving one from a hash. Each is derived from the compiled package's own
identity values, so the writer and admission use one function pair per
identity, and a written identity reads back to the identity it was written
from.

- **Universe.** `U` is the UniverseId's NodeId written as the JSON object
  `{"package": S, "path": [...], "ordinal": N}`: `package` the package
  string, `path` the array of path segments (a string, or an operand node's
  own node id object nested in its place), and `ordinal` a JSON integer in
  `0..=9007199254740991`. This is FSpec FR-181's `<node id>` for the
  reference's `universe`, the node of the declaration of the universe's least
  root type.
- **Most-specific type.** `T` is the EffectiveId written as the JSON object
  `{"owner_effective_type": E, "original": {"package": S, "node": S}}`, `E`
  `null` or another EffectiveId object, in FSpec FR-201's member structure.
  FSpec FR-181's `object_type` is the node id of the declaration that
  `original` names.
- **Object identity.** `O` is the identity's UTF-8 text as a JSON string;
  FSpec FR-181's `identity` is the lowercase hexadecimal of the same bytes.
- **Declaration names.** `I` is the declared name as the unit writes it for
  a record, tuple, enumeration or union the unit's own source declares
  (FR-569), with `::` between the segments of a qualified name; and, for one
  a selected model declares, its domain package declaration identity
  `ix://<package>/<artifact id>` exactly as the model document writes it
  (FSpec FR-154), such as `ix://acme/orders/Order`. A variant's `case` is
  the case's member name, never its `variant/<Name>-<value>` identity, and
  a record's `fields` keys are its field member names. Admission resolves a
  name with no `ix://` scheme only among the source declarations and an
  `ix://` name only among the model declarations (FR-106); a name that
  resolves to no declaration, or to another declaration than the slot's, is
  `wrong-value-kind` at `/record/type`, `/tuple/type` or `/variant/type`.
- **Unit names.** `N` is, for a unit the unit's own source declares, its
  declared name as the unit writes it (`ms`), and, for a unit a selected
  model declares, its domain package identity `ix://<package>/unit/<name>`
  (FSpec FR-154, such as `ix://acme/rpc/unit/ms`). A compound unit is FR-088's
  compound spelling `u1^e1*u2^e2`, in the UnitId's factor order with every
  exponent written, each factor a unit name by the rule above. The compiled
  package's unit registry SHALL expose one public pair, the unit name of a
  UnitId and the UnitId of a unit name, that both the run form's writer and
  its admission use; admission compares the UnitId the name reads to the
  slot's UnitId, and a name that reads to no unit or to another unit is
  FR-106's `wrong-value-kind` naming `unit`.

A string in place of an identity object (such as 64 hexadecimal
characters) is `wrong-value-kind` at the member; a missing or extra member
of an identity object is `missing-member` or `unknown-member`; a well-formed
universe that is not the universe of the parameter's declared object type is
FR-100's `foreign_reference`/`foreign-universe`.

### Depth and null

- The run value form has no depth limit. Admission reads an argument
  value at any nesting depth through the shared reader (FR-261), bounded
  only by the request's byte limit and the values-per-document setting, and
  the writer writes a completed value at any depth in full, without a
  recursion limit, truncation or a depth refusal. A result whose run value
  form nests more than 100 JSON levels is written and reads back unchanged.
- `null` is not a value of the run value form. A record field whose value is
  JSON `null` refuses `invalid_runtime_input`/`wrong-value-kind` at
  `/record/fields/<name>`, as does `null` at any other value position
  inside an argument; an optional field with no value is
  `{"absent": {}}`. The writer never writes `null` for a value: an absent
  optional field is `{"absent": {}}`, which is FSpec FR-181's `null` field
  entry and `option` `null` value.

### Arguments

A native-run/1 `call` object MAY carry `snapshot`: the file of one
`filament.state.snapshot/v1` document whose observation is `current`, given
relative to the request file's directory and read once under FR-100's
`dependent_bytes` limit. FR-106 admits it as FR-109's `Function` selection
admits its snapshot. Its complete object world and its populations are the
call's object context (FR-100 "Typed public call"). In FR-651's `call`
selection, `snapshot` names a key of the request's `observations` instead of
a file. A call with no `snapshot` has an empty object world.

At the request stage, a `value` SHALL be either a JSON integer in the signed
64-bit range or a JSON object. Any other JSON value (a string, a number with
a fraction or exponent, a JSON integer outside the signed 64-bit range,
`true`, `false`, `null` or an array) SHALL refuse `invalid-request` at stage
`request`, naming `/call/arguments/<index>/value`, exit 20. A JSON number is
not admitted beyond the signed 64-bit range because RFC 8785 reads a JSON
number as an IEEE double; an integer of any magnitude is written
`{"integer": "<decimal>"}`. A bare JSON `true` or `false` is not admitted
because the Boolean form is `{"boolean": ...}`.

A bare JSON integer is FR-098's canonical integer assignment, the form
FR-098's replay writes. It binds an integer parameter as that integer, and a
`Boolean` parameter as `false` for `0` and `true` for `1`; for any other
parameter type, or another integer at a `Boolean` parameter, it is a value
not of the parameter's declared type.

After the binding-name checks of FR-100 (unknown, duplicate and missing
parameters), admission SHALL read each argument against its parameter's
declared type, in declared parameter order, by FR-106's value rules
extended as above, and SHALL stop at the first defect. A defect SHALL
refuse at stage `call`, exit 20, with `details`
`{"position": P, "path": "<JSON pointer>"}`, where `P` is the parameter's
zero-based position and the pointer locates the defect inside the
argument's `value` (the empty pointer for the value itself):

| Defect | Code and cause |
| --- | --- |
| A form of another kind, a `type` naming another declaration, a payload that is not an object | `invalid_runtime_input`/`wrong-value-kind` |
| A required member missing | `invalid_runtime_input`/`missing-member` |
| A member the form does not define | `invalid_runtime_input`/`unknown-member` |
| A noncanonical payload text, a zero denominator, an undeclared case, a wrong component count, a value of the right kind outside the declared domain | `invalid_runtime_input`/`invalid-value` |
| A repeated member of a set or ordered set | `invalid_runtime_input`/`duplicate-member` |
| A collection length outside its declared bound | `cardinality_out_of_bound`/`below-minimum` or `above-maximum` |
| A Reference or Population argument in a call with no `snapshot` | `invalid_runtime_input`/`missing-member`, `details.path` the request pointer of the absent `snapshot` member (below) |
| A key or identity triple that resolves to no object of the object world | `dangling_reference`/`absent-target-in-complete-population` |
| A population the snapshot does not hold | `invalid_runtime_input`/`wrong-role-mapping` |

The defect of a Reference or Population argument in a call with no `snapshot`
is not inside the argument's value but in the call that lacks the member,
so its `details.path` is the JSON pointer, from the request object, of the
absent member: `/call/snapshot` in a native-run/1 request and
`/items/<index>/call/snapshot` for an FR-651 `call` item. It is the one
pointer of this table that is not relative to the argument's `value`, and
`details.position` still names the first such parameter.

A model-correspondence failure of a Reference or Population argument keeps
FR-100's `Admission` refusal rows. Every argument kind is carried by the run
value form, so a mistyped argument is invalid input, category refusal, exit
20. A parameter whose declared type has no form refuses as "Types with no form"
states.

### Results

The run SHALL write a completed value in the run value form of the selected
function's declared result type:

- a Boolean as `{"boolean": b}` and an integer as `{"integer": "<decimal>"}`;
- a Rational in FSpec FR-205's normalized form, a Decimal with its retained
  coefficient and scale, a Float with its exact bits, a String with its exact
  scalar sequence and a Text with its retained sequence under its profile;
- a Set or Bag with its elements ascending by the RFC 8785 bytes of each
  element's run value form, with every Decimal inside an element normalized,
  as FR-106 orders an admitted set; a Sequence and an OrderedSet in their own
  order;
- a Map with its entries ascending by the RFC 8785 bytes of each key's run
  value form, and a Quantity with its unit name and its normalized Rational
  magnitude;
- a record with one `fields` entry for every exposed effective field, an
  absent optional field as `{"absent": {}}`;
- a Reference in the identity-reference form, with the returned reference's
  own universe, most-specific type and object identity, never an input
  reference or a reminted identity;
- a population-typed result as `{"set": [...]}` of its members' identity
  references, which is FSpec FR-181's encoding of a population as a set of
  references.

The result types the run can write are every type the run can admit as an
argument, plus the population-typed result. A function whose declared result type has no form refuses as "Types with no
form" states; no other result type refuses.

### Types with no form

Every type FSL source can declare for a function has a run value form. The
one kind of type FSpec FR-181 gives no form is a type that holds a
population inside a collection, an option, a map or a record or tuple field
(such as `Sequence<Population<M::Thing>[1]>`), and it reaches a run in one
way only:

- **From FSL source** it cannot be spelled: S3 refuses its declaration
  wherever it appears, `ill_typed`/`operator-ineligible` (FSpec FR-144, a
  population is not a value and is no element type), category refusal,
  exit 20, at stage `check`. A population is a parameter or result type of
  FSL source only at the top level, where it has the population form.
- **In a selected checked package** (the request's `package` member, FR-100 and FR-945,
  such as a package another producer wrote from IR), a function may declare
  such a type, because FSpec FR-144 admits the type and refuses only its
  value. The run SHALL refuse it `unknown_required_feature`/
  `unsupported-feature` at stage `call`, category unsupported, exit 21,
  before any call (FSpec FR-144): with `details` `{"position": P}` for a
  parameter and `{"function": "<the function string>"}` for a result.

The run stage SHALL read each parameter and result type through one
exhaustive mapping from the checked `ValueType` to its run value form, and
this refusal is the mapping's only arm without a form; it is never a fallback
for a type the table gives a form, so a Quantity, a Map, a Reference or a
top-level population never refuses with it.

### Examples

These examples exercise the public command's canonical boundary. The native
forms above remain the separate internal reader contract. `Nc`, `Np` and
`Nt` below are the canonical declaration names of `Color`, `P` and `Tp`
from the compiled identity fixture, as written in the canonical results of
functions returning `Color::Green`, `P{x: 1, y: 2}` and `Tp(2, 1)`.

The **identity fixture** is this unit, each function returning its argument:

```fsl
language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
type Big = Int[0, 1267650600228229401496703205376];
type R = Rational[0, 9; 1, 9];
type D = Decimal[0, 1000; 0, 2; exact];
type F = Float64[nearest-even];
type T = Text[0, 8; nfc];
enum Color { Red, Green }
record P { x: Int[0, 9]; y: Int[0, 9]; }
tuple Tp(Int[0, 5], Int[0, 5]);
function id_bool using v(a: Boolean): Boolean pure { a }
function id_big using v(a: Big): Big pure { a }
function id_rat using v(a: R): R pure { a }
function id_dec using v(a: D): D pure { a }
function id_flt using v(a: F): F pure { a }
function id_str using v(a: String): String pure { a }
function id_txt using v(a: T): T pure { a }
function id_enum using v(a: Color): Color pure { a }
function id_rec using v(a: P): P pure { a }
function id_tup using v(a: Tp): Tp pure { a }
function id_seq using v(a: Sequence<Int[0, 9]>[0, 3]): Sequence<Int[0, 9]>[0, 3] pure { a }
function id_set using v(a: Set<Int[0, 9]>[0, 3]): Set<Int[0, 9]>[0, 3] pure { a }
function id_opt using v(a: Option<Int[0, 9]>): Option<Int[0, 9]> pure { a }
```

| Function | Argument `value` | Completed `value` |
| --- | --- | --- |
| `id_bool` | `{"type":"boolean","value":true}` | the same |
| `id_big` | `{"type":"integer","value":"1267650600228229401496703205376"}` | the same |
| `id_big` | `{"type":"integer","value":"7"}` | the same |
| `id_rat` | `{"type":"rational","numerator":"4","denominator":"6"}` | `{"type":"rational","numerator":"2","denominator":"3"}` |
| `id_dec` | `{"type":"decimal","coefficient":"150","scale":"2"}` | the same |
| `id_flt` | `{"type":"float64","bits":"3ff0000000000000"}` | the same |
| `id_str` | `{"type":"string","value":"abc"}` | the same |
| `id_txt` | `{"type":"text","value":"é"}` | `{"type":"text","value":"é"}` |
| `id_enum` | `{"type":"enum","name":Nc,"member":"Green"}` | the same |
| `id_rec` | `{"type":"record","name":Np,"fields":[{"name":"x","value":{"type":"integer","value":"1"}},{"name":"y","value":{"type":"integer","value":"2"}}]}` | the same |
| `id_tup` | `{"type":"tuple","name":Nt,"components":[{"type":"integer","value":"2"},{"type":"integer","value":"1"}]}` | the same |
| `id_seq` | `{"type":"sequence","elements":[{"type":"integer","value":"1"},{"type":"integer","value":"1"}]}` | the same |
| `id_set` | `{"type":"set","elements":[{"type":"integer","value":"3"},{"type":"integer","value":"1"}]}` | `{"type":"set","elements":[{"type":"integer","value":"1"},{"type":"integer","value":"3"}]}` |
| `id_opt` | `{"type":"option","value":{"type":"integer","value":"3"}}` | the same |
| `id_opt` | `{"type":"option","value":null}` | the same |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-650-AC-1 | Through the public `run` command, each row of the canonical identity-fixture table, as a native-run/1 `call` of its function with its argument `value`, returns one `spine-run-result/1` document whose `outcome` is `{"kind": "completed", "value": V}` with `V` the row's canonical completed value, exit 0. | Test (TC-1700) |
| FR-650-AC-2 | `id_big` called with `{"type":"integer","value":"9223372036854775808"}` completes with that same canonical value, exit 0. The `value`s `9223372036854775808` (a bare JSON integer above the signed 64-bit range), `1`, `"7"`, `1.5`, `true`, `null` and `[1]` each refuse `invalid-request` at stage `request`, exit 20, no result document, naming `/call/arguments/0/value`. | Test (TC-1701) |
| FR-650-AC-3 | Over the canonical identity-fixture table, each refuses at stage `call`, exit 20, no result document, with `details.position` 0 and the `details.path` given; codes not otherwise stated are `invalid_runtime_input`: `id_str` with `{"type":"text","value":"a"}`, `wrong-value-kind`, `""`; `id_rec` with `fields` lacking the `y` entry, `invalid-value`, `/fields`; `id_rec` with an extra third field entry named `z`, `invalid-value`, `/fields`; `id_enum` with member `Blue`, `invalid-value`, `/member`; `id_rat` with denominator `"0"`, `invalid-value`, `""`; `id_set` with `elements` containing integer `1` twice, `duplicate-member`, `/elements/1`; `id_seq` with four elements, `cardinality_out_of_bound`/`above-maximum`, `""`; `id_tup` with one component, `invalid-value`, `/components`; `id_bool` with `{"type":"integer","value":"2"}`, `wrong-value-kind`, `""`. | Test (TC-1702) |
| FR-650-AC-4 | Over FR-109-AC-4's `sameIdentity` unit as a native-run/1 `call` whose `snapshot` is the distinct-identities current snapshot: `a` and `b` are FR-181 canonical references to `root` and `child`, carrying structural `universe` and `object_type` node ids and `identity` `"726f6f74"` and `"6368696c64"` respectively; the call completes `{"type":"boolean","value":false}`, exit 0. The corresponding canonical reference to `ghost` (`identity` `"67686f7374"`) as `b` refuses `dangling_reference`/`absent-target-in-complete-population` at stage `call` with `details.position` 1, exit 20; the first request with `snapshot` removed refuses `invalid_runtime_input`/`missing-member` at stage `call`, `details.position` 0, `details.path` `/call/snapshot`, exit 20. A function over a Population parameter called with `{"population": P}` binds that population with the parameter's declared maximum and completes; with a population the snapshot does not hold it refuses `invalid_runtime_input`/`wrong-role-mapping` at stage `call`. | Test (TC-1703) |
| FR-650-AC-5 | TC-792's present lookup, run from a native-run/1 file request whose `call.snapshot` holds the looked-up object, completes with `{"type":"reference","universe":U,"object_type":T,"identity":O}`, where `U` and `T` are the structural declaration node ids and `O` is the lowercase hex encoding of the returned reference's exact identity bytes, exit 0; passing that completed value unchanged as the argument of a function `function same using v(r: Reference<T>): Reference<T> pure { r }` in the same request shape completes with the identical value. | Test (TC-1704) |
| FR-650-AC-6 | From source, a unit declaring `function gives using v(): Sequence<Population<Config::ConfigVersion>[1]>[0, 1] pure { [] }`, or `function takes using v(a: Sequence<Population<Config::ConfigVersion>[1]>[0, 1]): Boolean pure { true }`, over the ConfigVersion model refuses at S3 `ill_typed`/`operator-ineligible`, category refusal, exit 20, and reaches no call. A native-run/1 request whose `package` member selects the fixture package `tests/fixtures/population-collection.checked-package.json` (the compiled ConfigVersion package with the result type of its function `gives`, and the parameter type of its function `takes`, rewritten to that collection type, as an IR producer can write them) refuses `unknown_required_feature`/`unsupported-feature` at stage `call`, exit 21, before any call, with `details` `{"function": "gives"}` for `gives` and `{"position": 0}` for `takes`. A function whose parameter or result type is `Quantity<s>`, `Map<String, Int[0, 9]>[0, 2]`, `Reference<Config::ConfigVersion>` or a top-level population does not refuse `unknown_required_feature`. | Test (TC-1705) |
| FR-650-AC-16 | The run stage reads every parameter and result type through one exhaustive match over `ValueType` with no wildcard arm, each arm yielding the type's run value form or, only for a type holding a population below the top level, the `unknown_required_feature`/`unsupported-feature` refusal at stage `call`, exit 21, with `details` `{"position": P}` for a parameter and `{"function": "<the function string>"}` for a result, before any call. | Inspection (TC-2380) |
| FR-650-AC-17 | `function id_imap using v(a: Map<Integer, Population<Config::ConfigVersion>[4]>[0, 2]): Map<Integer, Population<Config::ConfigVersion>[4]>[0, 2] pure { a }` is called with two entries given in the order key 10 then key 9, each value an identity reference. A first-reference search of the Map visits the reference under key 9 before the reference under key 10 (mathematical key order, although the run form array sorts `{"integer":"10"}` before `{"integer":"9"}` by RFC 8785 bytes). | Test (TC-2950) |
| FR-650-AC-7 | The public run command's FR-312 `function` selection admits canonical arguments: FR-109-AC-4's `sameIdentity` with `a` and `b` both FR-181 canonical references to `child` reports `success`, `truth: true`, exit 0; a Boolean claim `function short using v(t: Text[0, 8; nfc]): Boolean pure { size(t) < 3 }` selected with `t` `{"type":"text","value":"ab"}` over the same snapshot reports `success`. | Test (TC-1706) |
| FR-650-AC-8 | The `execute` outcome (FR-286) of FR-100-AC-1's `seven` carries `result` `{"kind":"completed","value":{"type":"integer","value":"7"}}`, and of `id_rec` with the table's canonical record argument `result` `{"kind":"completed","value":V}` with `V` that record; each `value` equals the `spine-run-result/1` document's for the same request. | Test (TC-1707) |
| FR-650-AC-9 | Over FR-650-AC-5's `same` and the healthy current snapshot, a canonical reference to `child` completes as `{"type":"reference","universe":U,"object_type":T,"identity":"6368696c64"}`, where `U` and `T` are structural checked-package node ids, each with exactly `package`, `path` and `ordinal` (a JSON integer), naming the least root type's declaration and the object's most-specific type's declaration respectively; passed back unchanged it completes with the identical value. Replacing `universe` with a string of 64 lowercase hexadecimal characters refuses `invalid_runtime_input`/`wrong-value-kind`, message `reference conversion MalformedWire at /universe`, with `details` exactly `{"path":"/universe"}`; adding `x` to `object_type` refuses the same code and cause, message `reference conversion MalformedWire at /object_type`, with `details` exactly `{"path":"/object_type"}`. Each refusal is at stage `call`, exit 20, with no result document. | Test |
| FR-650-AC-10 | Over a unit declaring `dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;`, let `Ums` and `Us` be the canonical unit strings written by zero-argument functions returning `3 ms / 2` and `1 s`. Calling `function id_q using v(a: Quantity<ms>): Quantity<ms> pure { a }` with `{"type":"decimal","coefficient":"150","scale":"2","unit":Ums}` completes `{"type":"rational","numerator":"3","denominator":"2","unit":Ums}`, exit 0; substituting `Us` refuses `invalid_runtime_input`/`wrong-value-kind` at `/unit`, stage `call`, exit 20. For an already checked identity function whose parameter and result carry the bounded quantity type from `0 ms` through `10 ms`, the canonical rational form with `unit` `Ums`, denominator `"1"` and numerator `"10"` completes unchanged; numerator `"11"` refuses `invalid_runtime_input`/`invalid-value` at `/magnitude`, stage `call`, exit 20. | Test |
| FR-650-AC-11 | `function id_map using v(a: Map<String, Int[0, 9]>[0, 2]): Map<String, Int[0, 9]>[0, 2] pure { a }` called with `{"type":"map","entries":[{"key":{"type":"string","value":"b"},"value":{"type":"integer","value":"2"}},{"key":{"type":"string","value":"a"},"value":{"type":"integer","value":"1"}}]}` completes with the same entries ordered `a` then `b`, exit 0. It refuses, at stage `call`, exit 20: the key `a` given twice, `invalid_runtime_input`/`duplicate-member` at `/entries/1/key`; three entries, `cardinality_out_of_bound`/`above-maximum` at `""`; an entry without `value`, `invalid_runtime_input`/`missing-member` at `/entries/0`; an entry with a member `extra`, `invalid_runtime_input`/`unknown-member` at `/entries/0/extra`. A `text`-tagged key in this `Map<String, ...>` refuses `invalid_runtime_input`/`wrong-value-kind` at `/entries/0/key`. | Test |
| FR-650-AC-12 | Over the ConfigVersion model extended with variant type `ix://example/config-version/Tier` (cases `Gold`, `Silver`) and record value type `ix://example/config-version/Stamp` (one required `Integer` field `n`), let `Nt` and `Ns` be the canonical declaration-name strings written by zero-argument functions returning `Config::Tier::Gold` and `Config::Stamp{n: 3}`. Calling `id_tier` (`Config::Tier` to itself) with `{"type":"enum","name":Nt,"member":"Gold"}`, and `id_stamp` (`Config::Stamp` to itself) with `{"type":"record","name":Ns,"fields":[{"name":"n","value":{"type":"integer","value":"3"}}]}`, each completes with its argument unchanged, exit 0. `id_tier` with `name` `"Tier"` refuses `invalid_runtime_input`/`wrong-value-kind` at `/name`, and with `member` `"variant/Tier-Gold"` refuses `invalid_runtime_input`/`invalid-value` at `/member`, each at stage `call`, exit 20. | Test |
| FR-650-AC-13 | A function returning its argument of a type nesting `Sequence<...>[0, 1]` 60 levels deep around `Int[0, 1]`, called with `{"type":"integer","value":"1"}` nested in `{"type":"sequence","elements":[V]}` once per level, completes with the identical value, more than 100 JSON levels deep, exit 0, with no depth refusal. For `record P { x: Int[0, 9]; y: Int[0, 9]; }`, let `Np` be the canonical declaration name written by a function returning `P{x: 1, y: 2}`. Calling `id_rec` with `{"type":"record","name":Np,"fields":[{"name":"x","value":{"type":"integer","value":"1"}},{"name":"y","value":null}]}` refuses `invalid_runtime_input`/`wrong-value-kind` at `/fields/1/value`, stage `call`, exit 20. Identity functions over String, Bytes and JsonObject complete unchanged for `{"type":"string","value":"ready"}`, `{"type":"bytes","value":"00ff"}` and `{"type":"json_object","value":"{\"a\":1,\"b\":[true,null]}"}` respectively, including the empty String and Bytes and the JsonObject values `"x"` and `[]` in their canonical string payloads. A String given as `{"type":"text","value":"ready"}` refuses `invalid_runtime_input`/`wrong-value-kind` at `""`; Bytes or JsonObject with a `text` member in place of `value` refuses `invalid_runtime_input`/`unknown-member` at `/text`, stage `call`, exit 20. | Test |
| FR-650-AC-14 | Over the healthy current snapshot, `function members using v(p: Population<Config::ConfigVersion>[4]): Population<Config::ConfigVersion>[4] pure { p }` called with `{"population":P}` completes `{"type":"set","elements":[R1,R2]}`, exit 0. `R1` and `R2` are the FR-181 canonical reference forms of `root` and `child`, with `identity` `"726f6f74"` and `"6368696c64"`, ordered ascending by each reference's RFC 8785 bytes. | Test |
| FR-650-AC-15 | Over the healthy current snapshot, `sameIdentity` called with `a` a canonical reference whose `universe` is replaced by `{"package":"other/package","path":["ConfigVersion"],"ordinal":0}` refuses `foreign_reference`/`foreign-universe` at stage `call`, exit 20, message `reference conversion IncompatibleUniverse at /universe`, with `details` exactly `{"path":"/universe"}` and no result document. An FR-651 `items` request whose `call` selection of `sameIdentity` names the healthy current snapshot's observation key as `snapshot`, with `a` and `b` both canonical references to `child`, completes `{"kind":"completed","value":{"type":"boolean","value":true}}` in that item's `result.outcome`, exit 0. | Test |
| FR-650-AC-18 | Through the public run command, canonical Integer, Boolean, Text, Sequence and Set arguments complete with canonical values; a 23-digit integer remains exact, `4/6` completes as `2/3`, an empty sequence and one at its maximum retain order, and a set supplied as 3 then 1 completes as 1 then 3. A call with no arguments also writes its result canonically. | Test |
| FR-650-AC-19 | The public run command refuses an untagged integer, a bare integer, a tagged integer with noncanonical decimal spelling, an unknown member, and an out-of-domain integer, each with exit 20. Request and dependent-byte exhaustion remain incomplete with no completed value. | Test |
| FR-650-AC-20 | Genuine canonical argument membership and execution each fit a supplied admission-work limit independently, but their combined work on the same supplied run exceeds it and returns the typed supplied-value limit refusal. Reference codec conversion retains its separately cumulative conversion-work counter on that same run. | Test |
| FR-650-AC-21 | A genuine checked-call reference catalog exhausts its original normalization owner's remaining work allowance and returns `NormalizationIncomplete` at `reference.catalog`, retaining the configured limit, prior consumption and denied charge. | Test |
| FR-650-AC-22 | A checked-package call over a genuine admitted snapshot accepts a canonical reference and returns that same canonical reference; feeding the result into another package call completes unchanged after the original program file is removed. The catalog and writer use the same snapshot admission's retained normalization owners. | Test |
| FR-650-AC-23 | A call with no models and no snapshot completes with its canonical value when `model.work_units` is zero; canonical output does not introduce model work into that value-only call. | Test |

## Dependencies

- FR-100: the request envelope, the binding-name checks, the call stage, the
  `Admission` rows and the result document.
- FR-106: the value grammar, its member rules and the snapshot admission.
- FR-360: the variant form.
- FR-084 and FR-089: population binding and identity.
- FR-038: the integer spelling.
- FR-088: the compound unit spelling; FR-261: reading JSON at any depth.
- FSpec FR-181: the typed canonical forms the run value form maps onto;
  FSpec FR-144 for supplied collections; FSpec FR-550 for the standard
  envelope's values.
- FSpec FR-201: the structural identities the identity spellings write;
  FSpec FR-154: the domain package declaration identities.
- ADR-034 CS-1 and CS-2: FR-106's native spelling and its correspondence to
  FSpec FR-181.
