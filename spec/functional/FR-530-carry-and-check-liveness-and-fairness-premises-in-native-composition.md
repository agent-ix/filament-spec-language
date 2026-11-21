---
id: FR-530
title: "Carry and check liveness dependencies and fairness premises in native composition"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-031
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-529
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-489
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-491
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-492
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-348
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-176
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-380
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-177
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---
# FR-530: Carry and check liveness dependencies and fairness premises in native composition

## Description

FSL SHALL compile, emit and read back FSpec FR-491's `composition-decl` over
checked component packages with FSpec FR-489's four premise lists kept
separate, and SHALL settle an `assume-guarantee` composition item from FSpec
FR-492's settlement request with FR-489's premise discharge record, in the
`fsl-replay` `planner` module FR-529 places, and every other rule, and an
`assume-guarantee` declaration selected by name through analyze (FR-1044), stay
FSpec FR-348's routed `composition` item.

## Inputs

- A composing unit that imports the component checked packages and declares
  a `composition-decl` (FSpec FR-491).
- For settlement: the composing checked package, its component packages, and
  one `filament.planner-settlement/v1` request (FSpec FR-492) carrying the
  slots, the pool of supplied component guarantee results and any discharge
  records.

## Outputs

- At S2 to S4: the checked declaration, its `correspondence`/
  `component_composition` node and its derived item and slots; or FSpec
  FR-491's located refusals.
- At settlement: FSpec FR-492's planner result with FR-489's premise
  discharge record, or its `invalid_runtime_input` refusal.

## Behavior

### Authoring and compile

The declaration is a `Relation` family declaration (ADR-012 §3), built at S2,
checked at S3 by FSpec FR-491 "Static checks", and emitted and read back at
S4; its targets in component packages resolve through the S4 dependency
closure, as a refinement's abstract package does (ADR-020 RM-1). S3 builds the
dependency graph and finds its cycles (strongly connected components) per
FSpec FR-491 "Dependency graph and cycles". Under `assume-guarantee` it
derives one planner-local item and its slots, and, when an analyze request
selects the declaration by name (FR-1044), the one routed FR-331 item of the
other rules; under any other rule it derives one FR-331 `composition`
requirement record whose `premises` and `assumptions` members `fsl-route`
writes from the slots and the declaration (FSpec FR-491 "Assumption
binding"), and `route` negotiates it as FSpec FR-348 states, with
`unsupported` when no composition-capable backend is registered
(FR-348-AC-4). The two entries stay separate: planner settlement reads FSpec
FR-492's request and never an analyze item, and the routed item never reaches
the planner.

### Owning layer and input type

Settlement is FR-529's `fsl_replay::planner` module, for the reasons and
under the layer allow-list FR-529 states. `read_settlement_request` reads a
composition request into the same `SettlementRequest`, whose pool holds one
`PremiseResult` per supplied component guarantee and whose discharge records
pair a slot index with a pool index; FSpec FR-492 "Admission" applies,
including step 7 for a discharge record.

### What is reused from TL-9 and what FSL adds

FSL holds no native implementation of TL-9's assume-guarantee algorithm,
fixed point or protocol refinement, and adds none:

| Shape | Reused, as an input or a route | FSL addition |
| --- | --- | --- |
| Composition negotiation, every rule when selected through analyze | FSpec FR-348's `composition` item, routed unchanged to a registered provider (FR-635, FR-1044); its result is read | the `premises` and `assumptions` members the request writer fills |
| Acyclic discharge order | FSpec FR-176 "Discharge fixed point": acyclic nodes discharge in topological order | `planner` discharges slots in that order |
| Fixed point over a cycle | none: no requirement defines the lattice declaration (FSpec FR-489 "Cycles") | the cycle detection and the typed `unsupported` answer; no `fixed-point` slot |
| Protocol refinement and refinement transfer | FSpec FR-177 and FR-380 results, read as premise results | none |
| Fairness implication | FSpec FR-380 "Fairness implication" | its application to each fairness premise |
| Premise transport and discharge record | none | the declaration, the four lists, the slots, admission, the discharge matching and the record |

### Discharge

`planner` discharges each slot by FSpec FR-489 "Implication": a supplied
guarantee discharges a `rely` or `liveness-dependency` slot only when its
request item's subject is the slot's peer subject, its claim node is the
peer guarantee clause, FR-363 gives it the slot's form, its result is
`proved`, and each of its fairness constraints is implied by a `supplies`
constraint. No other entailment is decided: a different clause, a weaker
claim, a guarantee of another kind, or a same-named guarantee of another
package leaves the slot undischarged. A component-guarantee slot is
discharged by a `proved` result of that guarantee's item checked under
exactly its FR-176 assumption set (the peer guarantees its `rely` and
`depends` members name), which is part of its slot key as the member
`assumptions`, the strictly ascending `slots` indices of those peer
`component-guarantee` slots, empty where it assumes none and under every rule
but `assume-guarantee` (FSpec FR-491 "Assumption binding", FSpec FR-492
"Request"); the guarantee's own item carries no assumptions. A result checked
under other assumptions or none is refused at admission, and
a failed component check leaves the slot undischarged. A discharge record that names an existing pool
entry over the right subject is used only after the same check; an
`inconclusive` referenced result discharges nothing.

### Cycles

A well-formed declaration under any of the four rules whose dependency graph
holds a cycle settles its composition item `unsupported`, basis `unavailable`,
with the diagnostic `unsupported_projection`/`unsupported-requested-capability`
whose `detail` names the kind `composition`, the form `fixed-point` and the
cycle's guarantees in edge order (FSpec FR-489 "Cycles"). The planner gives
this answer before it reads any slot, and a routed item gives it before the item
is written to a provider, with a provider registered or not, so no pool,
`discharges` entry or provider result can make it `proved` and no guarantee of
the cycle is available as an assumption. FSL derives no `fixed-point` slot and
adds no lattice declaration, profile grammar or waiver. A declaration that is
not well formed keeps its own located refusal, distinct from this answer, and
the package's other items settle unchanged.

### Settlement and outcomes

After the cycle answer above, which comes first, and before the slots, the
planner SHALL decide an interface mismatch by FSpec FR-492 "Interface
mismatch", from the declaration alone and never from a supplied result. The
order is reason-major: every `absent` mismatch over the `rely` and
`liveness-dependency` slots in ascending slot order, and only when there is none
every `weaker` mismatch over the same slots in the same order. The first mismatch
found SHALL settle the item `refuted`, basis `decisive-counterexample`, with
`mismatch` `{component, interface, premise, reason}`, no cause, certification or
conclusion, and `slots` the complete slot outcome list in slot order. A declared
interface that is present and not weaker, whose premise a supplied result does
not discharge (absent, `tested`, `inconclusive`, another subject, kind or
clause, or not `proved`), is never a mismatch.
The item otherwise settles by FSpec FR-492 "Decision order", writing FSpec FR-489's
premise discharge record: per slot, the discharging pool entry or supplied
constraint, the subject, the relation (`identity`, `fairness-implication` or
`assumption`), its certification and extent. A premise
discharged by a `trusted` plugin result settles `proved` with certification
`trusted`; a `rely` or liveness-dependency premise whose only discharger is
an environment record is `inconclusive`, `premise-unproved` with `result`
`assumption`, while an `environment` premise itself settles `supplied` and
is listed as an assumption. Unmet premise, trusted
assumption, declared interface mismatch, unsupported rule, cycle, failed check
and exhausted budget keep their distinct outcomes, trust basis and extent; the request's unrelated supported
claims settle unchanged. Backend execution stays with the TL-9 providers,
and Quoin consumes the result without validating the theorem anew.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-530-AC-1 | Compiling, emitting and reading back a `composition-decl` keeps the component subjects, the rule and the `environment`, `relies`, `liveness_dependencies` and `fairness` lists separate, with each slot's source span its member; a Boolean success flag in place of the slots is refused `invalid_runtime_input`/`wrong-value-kind`. | Test (TC-1330) |
| FR-530-AC-2 | The discharge record names, for each slot, its discharger; a supplied guarantee over another subject, a weaker clause, a guarantee of another kind and a same-named guarantee of another package each leave the slot undischarged and, the declared interface being present and not weaker, the item `inconclusive`, `premise-unproved`, naming it and never `refuted`. | Test (TC-1330) |
| FR-530-AC-3 | A `Producer`/`Consumer` fixture whose relies, dependencies and fairness are each discharged settles `proved`, `decisive-witness`, under `assume-guarantee` with the discharge record. | Test (TC-1330) |
| FR-530-AC-4 | A mutual liveness dependency in a well-formed declaration under each of the four rules settles the composition item `unsupported`, basis `unavailable`, `unsupported_projection`/`unsupported-requested-capability` naming the kind `composition`, the form `fixed-point` and both guarantees in edge order, before any slot is read and before any planner or provider settlement runs, with or without a registered provider; no guarantee of the pair is used as an assumption and no pool, discharge entry or provider result makes it `proved`. | Test (TC-1330) |
| FR-530-AC-5 | **Withdrawn (QSL-2739).** The criterion settled a mutual dependency under a profile declaring FR-176's lattice with a `fixed-point` slot; FSpec FR-489 defines no way a profile declares the lattice and withdrew its matching criterion, so no source unit can select one and no `fixed-point` slot is derived. AC-4 holds for every cycle under all four rules; no replacement. | Withdrawn |
| FR-530-AC-6 | A `trusted` plugin discharger, an assumption-only premise, an unsupported rule (`interface` with no composition-capable backend), a failed component check and an exhausted `max_work` settle `proved`/`trusted`, `inconclusive`/`premise-unproved`, `unsupported`, `inconclusive`/`premise-unproved` and `incomplete`, each with its own trust basis and extent. | Test (TC-1330) |
| FR-530-AC-7 | A discharge record naming no pool entry, or a pool entry over another subject, is refused `invalid_runtime_input`/`invalid-value` at the record; one naming an `inconclusive` result discharges nothing. | Test (TC-1330) |
| FR-530-AC-8 | Admission refuses a forged or reordered slot and a premise result whose obligation identity, computed with the slot's declaration-derived `assumptions`, is not its slot's; an unrelated supported item of the same package settles unchanged in every case. | Test (TC-1330) |
| FR-530-AC-9 | A composition whose `rely` slot names a peer clause that no declared `guarantee` covers, and one whose peer guarantee is conditional on a fairness constraint no `supplies` constraint implies, settle `refuted`, `decisive-counterexample`, with `mismatch` `{component, interface, premise, reason}` naming the peer component, its clause node, the slot and `absent` or `weaker`, no `cause`, certification or conclusion, and `slots` listing every slot outcome in slot order. A `weaker` mismatch at slot 3 and an `absent` one at slot 4 settle with the slot 4 `absent` mismatch; two `absent` slots, the lower; a supplied result that is absent, `tested`, `inconclusive`, over another subject or of another clause, with the interface declared and not weaker, settles `inconclusive`, `premise-unproved`, never `refuted`. | Test (TC-1330) |
| FR-530-AC-10 | A `component-guarantee` slot key carries `assumptions`, the strictly ascending peer slot indices from the declaration (empty under every rule but `assume-guarantee`), and a premise result for the same guarantee checked under other assumptions or none is refused `invalid_runtime_input`/`invalid-value` at `/slots/<i>/result`. | Test (TC-1330) |

## Dependencies

- FSpec FR-331 (the `assumptions` member, the `interface-mismatch` counterexample
  and the producer-only `provider-answer-unconfirmed` cause); FR-1044 owns their
  analyze route.
- FSpec FR-489, FR-491 and FR-492 (with their TC-983, TC-985 and TC-986);
  FSpec FR-348, FR-176, FR-177 and FR-380, reused as the table above states.
- [FR-529](FR-529-settle-a-temporal-guarantee-of-implementation-rust-natively.md):
  the `planner` module, its input type and the closed causes.
- This requirement adds no parallel evidence store and no theorem import.

## References

- Linear QSL-1542; TL-9.
