---
id: SR-17952655000001
title: "QSL-2178 code-review of frozen PR 3303"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2178 QSL-2717; frozen PR 3303"
review_set: subset
---

## Summary

Ticket: QSL-2178; linked QSL-2717. Independent frozen-head code review with the Rust lane folded in. Four concrete defects remain in ClockRef admission and route parity; hyper and reference-set production changes inspected without a new defect.

## Verdict

FAIL

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Clock-reference values still make their owner population required: parameter_populations treats every Reference leaf as an object reference, the object-field completeness traversal collects every raw reference, and read_context_populations does likewise. A valid place owned by an admitted object in an incomplete population therefore refuses incomplete-scope, contrary to the new explicit ClockRef completeness exemption. Preserve static type when computing required populations. | fsl-semantics/src/model/observation/document.rs:3830-3851,3919-3936 |
| FND-002 | high | A valid ClockRef in a read context is admitted by References::resolve but then check_context_closure searches its place/v1 identity in keys_by_population as an object key. With a complete owner population it always refuses dangling_reference. Exclude only already authenticated typed clock-place references from context closure, preserving ordinary dangling checks. | fsl-semantics/src/model/observation/document.rs:3718-3727,3790-3821 |
| FND-003 | high | BarePackage observation admission still passes plain model selections to StateModel/admit_current_snapshot, so ModelSelections has an empty clock_models slice. DeclaredClockPlaces is empty and valid stored ClockRefs refuse instead of matching the source route, which now uses graph.model_inputs with checked templates. The strict readback test never constructs or executes a BarePackage. Supply authenticated bytes-derived declared places and add actual source/Bare admission and evaluation parity evidence. | fsl-package/src/bare/observation.rs:76-82,134-144,194-203 |
| FND-004 | medium | DeclaredClockPlaces::new retains Map clock templates but drops the map flag, and contains accepts any matching numeric selector below its maximum. A forged place/v1 ID for a Map<Text, Clock> slot can therefore be admitted as ClockRef data, although Map places have no place ID. Domain::with_clock_places correctly skips these same map templates. Reject Map templates in admission and candidate closure as well. | fsl-semantics/src/model/clock_reference.rs:35-50,83-89 |

## Coverage

All 45 changed files inspected against base f1c43209be6b2344addb1048c353120a70249369 and frozen head 04c0d0b46a2c494965f2bc7c65042dac44f95954. Manifest and lock agree on IR 1cbd86a48f02e4623e558a3d3b23c066c7f280bf; no other pin delta or CI weakening. Separate producer-forward aliases do not pollute final reverse/canonical maps; substitutions compose in reverse application order. All ten reference-set fields sort before compaction; sequences/repeated operands remain ordered, and the three-clause source test has distinct IDs. Hyper source_unit is only on hyper claim-name rows and reached Local slots sort by byte name. Clock domains are indexed mixed-radix with actual owner/type/template identities and explicit per-subject maxima; ordinary untimed routes require selector authority and preserve generic errors. No new asynchronous tasks or unsafe code. Existing eager timed place slot_vectors allocation remains an inherited limitation, not a newly introduced algorithm. No new reproduction was executed. The three frozen-head logs each selected/passed exactly 1, 0 ignored: /tmp/qsl-third-rebase-clock-package.log, /tmp/qsl-third-rebase-protocol-clock-bound.log, /tmp/qsl-third-rebase-clock-refine-boundary.log. Earlier focused evidence and red control were inspected via supplied handoff; no aggregate gate authorized. Final analyze log includes the existing large-linker unwind warning; it is not suppressed. All build evidence is coder supplied. Source-only runtime plus strict readback does not establish Bare execution parity.

## Examined Scope

```yaml
scope:
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      A statically `Reference<Clock>`-typed value
      does not name a population member and adds no required population through
      its target; its declared-place validation is independent of population
      completeness.
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      `Reference<Clock>` target is checked against declared clock places
             under the same rule as an object field.
  - id: FR-531
    path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
    role: examined
    excerpt: >-
      The bare route SHALL give the same result, the same refusal and the same
      charge
        log as the in-process route for one package and one request.
  - id: FR-1040
    path: spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md
    role: examined
    excerpt: |-
      `ref` of a place under a `Map` value refuses `ill_typed`/`type-mismatch`
          at the `ref` (FR-230 "Place identity");
  - id: FR-1040-AC-3
    path: spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md
    role: examined
    excerpt: "In a refinement row over `calls = {c1, c2}`, `only(c in M::calls where
      c.n = 2: ref(c.x))` and `count(c in M::calls where ref(c.x) = self.r)`
      check, lower with the binder `c` itself (typed `Reference<Call>`) as owner
      term, and evaluate as FSpec FR-829-AC-2 states; the same `count` in a
      state invariant (FR-391) checks and lowers."
  - id: FR-1040-AC-8
    path: spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md
    role: examined
    excerpt: Over a domain package `acme/rpc` whose object type `Call` has clock
      fields `x` and `y` and a field `r` typed `ix://acme/rpc/ClockRef`, a type
      definition of `kind` `reference` with `target`
      `ix://filament/native/Clock`, intake gives `r` the type `Reference<Clock>`
      as a data member, and the postcondition `self.r = ref(self.x)` of an
      operation `point` whose frame names only `r` checks with no `clock-write`
      refusal and lowers the write to a `filament.op.clock_reference.ref`
      application typed by the `scalar_type`/`clock_reference` node, which also
      types the field; after `point` the state's `r` hold
  - id: FR-106-AC-72
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "Over [TC-464](../test-cases/TC-464-snapshots-and-invocations-admit.md)
      step 7's dense-time `Call` fixture, a raw snapshot storing the actual
      per-instance place identities of `c1.x` and `c2.x` in `r:
      Reference<Clock>`, `maybe: Option<Reference<Clock>>` and `refs:
      Sequence<Reference<Clock>>` admits, retaining those identities as data,
      the optional presence and the sequence order. Neither place identity is an
      object key, and neither target is resolved as a population member. Reading
      the admitted fields distinguishes `c1.x` from `c2.x`; the required
      evaluation of the stored `self.r` under FR-1040-"
  - id: FR-106-AC-73
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: Over [TC-464](../test-cases/TC-464-snapshots-and-invocations-admit.md)
      step 8's model variant without `x`, storing the formerly declared `c1.x`
      identity in `r`, in the present value of `maybe`, or at element 0 of
      `refs` refuses
      `dangling_reference`/`absent-target-in-complete-population`, naming the
      snapshot, owner population `calls`, object `c1`, the affected field and
      nested value path, and that undeclared target identity, with no admitted
      observation or truth value. A target's clock-reference static type or
      native Clock target alone does not make it declared.
  - id: Cargo.lock
    path: Cargo.lock
    role: examined
    excerpt: source =
      "git+https://github.com/agent-ix/filament-contract-ir?rev=1cbd86a48f02e4623e558a3d3b23c066c7f280bf#1cbd86a48f02e4623e558a3d3b23c066c7f280bf"
  - id: Cargo.toml
    path: Cargo.toml
    role: examined
    excerpt: filament-contract-model = { git =
      "https://github.com/agent-ix/filament-contract-ir", rev =
      "1cbd86a48f02e4623e558a3d3b23c066c7f280bf" }
  - id: fsl-analyze/src/analyze/timed_item.rs
    path: fsl-analyze/src/analyze/timed_item.rs
    role: examined
    excerpt: |2-
                      domains: Some(domains),
                      clock_references: Vec::new(),
  - id: fsl-analyze/src/zone_check/refine.rs
    path: fsl-analyze/src/zone_check/refine.rs
    role: examined
    excerpt: >-
      fn timed_subject_refusal(refusal: TimedSubjectRefusal) ->
      TimedRefinementRefusal {
          match refusal {
              // Keep the existing requires-bound/resource refusal envelope when
              // the profile-aware constructor performs the preflight.
              TimedSubj
  - id: fsl-analyze/tests/it/model_check.rs
    path: fsl-analyze/tests/it/model_check.rs
    role: examined
    excerpt: |2-
                  .as_ref()
                  .expect("generic unbounded domains")
  - id: fsl-analyze/tests/it/model_instances.rs
    path: fsl-analyze/tests/it/model_instances.rs
    role: examined
    excerpt: |2-
              .as_ref()
              .expect("generic unbounded domains")
              .as_ref()
              .expect("generic unbounded domains")
              .as_ref()
              .expect("generic unbounded domains")
  - id: fsl-analyze/tests/it/protocol_compensation.rs
    path: fsl-analyze/tests/it/protocol_compensation.rs
    role: examined
    excerpt: |2-
                  .as_ref()
                  .expect("generic unbounded domains")
              .as_ref()
              .expect("generic unbounded domains")
  - id: fsl-analyze/tests/it/protocol_system.rs
    path: fsl-analyze/tests/it/protocol_system.rs
    role: examined
    excerpt: |2-
          let named: Vec<_> = bound
              .domains
              .as_ref()
              .expect("generic unbounded domains")
              .iter()
              .map(|(key, _)| key.clone())
              .collect();
          let named: Vec<_> = bound
              .domains
              .as_ref()
              .exp
  - id: fsl-analyze/tests/it/timed_refinement.rs
    path: fsl-analyze/tests/it/timed_refinement.rs
    role: examined
    excerpt: |-
      /// The default timed profile supplies the selector bound for concrete
      /// ClockRef data before refinement's independent hidden-field refusal.
      #[trace("TC-3084", "FR-1040-AC-8")]
      #[test]
      fn clock_reference_selectors_use_the_timed_profile_before_refinement_gate
  - id: fsl-analyze/tests/it/zone_mitl.rs
    path: fsl-analyze/tests/it/zone_mitl.rs
    role: examined
    excerpt: |2-
          assert!(
              !roots
                  .domains
                  .as_ref()
                  .expect("generic unbounded domains")
                  .is_empty(),
              "{roots:?}"
          );
  - id: fsl-eval/src/simulation/mod.rs
    path: fsl-eval/src/simulation/mod.rs
    role: examined
    excerpt: pub use not_simulated::{ClockReferenceDomain, NotSimulated,
      RequiresBound, Root, BOUND_POSITIONS};
  - id: fsl-eval/src/simulation/model.rs
    path: fsl-eval/src/simulation/model.rs
    role: examined
    excerpt: >2-
          ) -> Option<domain::Domain> {
              self.field_domain_with_clock_selectors(type_key, field, references, None)
          }

          fn field_domain_with_clock_selectors(
              &self,
              type_key: &DeclarationKey,
              field: &StateField,
              reference
  - id: fsl-eval/src/simulation/model/application.rs
    path: fsl-eval/src/simulation/model/application.rs
    role: examined
    excerpt: >2-
                      // Protocol attempts have no timed-profile selector authority.
                      // check_protocol_bounds runs the model's requires-bound
                      // preflight before an attempt can expand frames.
                      clock_selectors: None,
  - id: fsl-eval/src/simulation/model/domain.rs
    path: fsl-eval/src/simulation/model/domain.rs
    role: examined
    excerpt: |-
      /// Selector maxima authenticated by a timed subject's live-clock bound.
      pub(super) type ClockSelectorMaxima = BTreeMap<NodeKey, Vec<u64>>;

      #[derive(Clone, Copy, Default)]
      pub(super) struct ClockDomains<'a> {
          pub(super) models: &'a [fsl_semantics::check::
  - id: fsl-eval/src/simulation/model/frames.rs
    path: fsl-eval/src/simulation/model/frames.rs
    role: examined
    excerpt: >2-
          FieldChange, ModelFinding, ModelSimState, ModelSystem, StepEffect,
      WrittenSlots, BROKEN,
          EXHAUSTED,
                      None => walk.value_domain(value_type, &references),
                      let domain = self.walk.value_domain(result_type, &self.walk.referenc
  - id: fsl-eval/src/simulation/model/input.rs
    path: fsl-eval/src/simulation/model/input.rs
    role: examined
    excerpt: "            .with_clock_places(self.timed_models)"
  - id: fsl-eval/src/simulation/model/timed.rs
    path: fsl-eval/src/simulation/model/timed.rs
    role: examined
    excerpt: >-
      use super::walk::{expand_enabled_with_clock_selectors, stop};

      /// The potential selector maxima under the timed subject's live-clock
      bound.

      /// This is the same per-element layout used for concrete clock
      placements.

      pub(super) fn clock_selector_maxima(
          tim
  - id: fsl-eval/src/simulation/model/walk.rs
    path: fsl-eval/src/simulation/model/walk.rs
    role: examined
    excerpt: "    /// Selector maxima from this timed subject's authenticated clock
      bound.

      \    pub(super) clock_selectors: Option<&'s
      super::domain::ClockSelectorMaxima>,

      \    pub(super) fn value_domain(

      \        &self,

      \        value_type: &filament_exact::ValueType,

      \        "
  - id: fsl-eval/src/simulation/not_simulated.rs
    path: fsl-eval/src/simulation/not_simulated.rs
    role: examined
    excerpt: >-
      /// A generated data member whose clock-reference domain needs a selector
      bound.

      #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]

      pub struct ClockReferenceDomain {
          /// The actual field or parameter type position.
          pub member: fsl_foundation::boun
  - id: fsl-eval/src/simulation/protocol.rs
    path: fsl-eval/src/simulation/protocol.rs
    role: examined
    excerpt: >2-
                  domains: Some(domains),
                  clock_references: Vec::new(),
          let ordinary = match check_requires_bound(&roots(&domains), model.types(), position_limit) {
              Ok(()) => None,
              Err(NotSimulated::RequiresBound(bound)) => bound.doma
  - id: fsl-eval/src/value/expression/evaluate.rs
    path: fsl-eval/src/value/expression/evaluate.rs
    role: examined
    excerpt: >2-
          FieldValue, ObjectReference, OptionValue, Quantity, UnionValue, Value,
      ValueType,
          if !fsl_semantics::model::clock_reference::is_clock_type(&clock_type) {
              return Err(invariant("place-reference-result-type-expected"));
          fsl_semantics::model::c
  - id: fsl-eval/tests/it/finite_simulation.rs
    path: fsl-eval/tests/it/finite_simulation.rs
    role: examined
    excerpt: |2-
              assert_eq!(
                  requires_bound
                      .domains
                      .as_ref()
                      .expect("generic unbounded domains")
                      .len(),
                  1
              );
                  .as_ref()
                  .expect("generic unbounded
  - id: fsl-eval/tests/it/model_simulation.rs
    path: fsl-eval/tests/it/model_simulation.rs
    role: examined
    excerpt: |2-
                  .as_ref()
                  .expect("generic unbounded domains")
  - id: fsl-eval/tests/it/refinement_row_clock_places.rs
    path: fsl-eval/tests/it/refinement_row_clock_places.rs
    role: examined
    excerpt: >2-
          format!(
              "{:?}",
              evaluate_row(&package, &checked.packages, row, receiver, &state())
          )
      }


      /// Evaluate a row against supplied state, keeping the checked package
      fixed

      /// while a reference field is updated.

      fn evaluate_row(
          package: 
  - id: fsl-eval/tests/it/timed_subject.rs
    path: fsl-eval/tests/it/timed_subject.rs
    role: examined
    excerpt: |-
      fn clock_reference_world(
          document: Json,
          unit: &str,
      ) -> (
          World,
          filament_exact::ObjectReference,
          filament_exact::ObjectReference,
      ) {
          clock_reference_world_with_profiles(document, "", unit)
      }

      fn clock_reference_world_with_profiles(
  - id: fsl-package/src/emit.rs
    path: fsl-package/src/emit.rs
    role: examined
    excerpt: |2-
              BodyTerm::Application(application) => {
                  let mut arguments = application
                      .collect::<Result<Vec<_>, _>>()?;
                  if application.operator == Operator::ProtocolControl {
                      if let Some(Member::ProfileOperator
  - id: fsl-package/src/emit/reference_sets.rs
    path: fsl-package/src/emit/reference_sets.rs
    role: examined
    excerpt: |-
      // SPDX-License-Identifier: AGPL-3.0-or-later
      //! FR-464 reference sets are ordered by structural checked node identity.

      use filament_contract_model::CheckedTerm;

      use super::{encoding, EmitRefusal};

      /// Order only the reference sets of the protocol layout. 
  - id: fsl-package/src/emit/tests.rs
    path: fsl-package/src/emit/tests.rs
    role: examined
    excerpt: mod reference_sets;
  - id: fsl-package/src/emit/tests/clock_places.rs
    path: fsl-package/src/emit/tests/clock_places.rs
    role: examined
    excerpt: >-
      /// Canonical reference fields and their data reads/writes retain the one

      /// clock_reference scalar, including ordinary presence and sequence
      wrappers.

      #[trace("TC-3084", "FR-1040-AC-8", "FSpec-FR-154-AC-53")]

      #[test]

      fn clock_reference_fields_emit_and_read_b
  - id: fsl-package/src/emit/tests/hyper_clause.rs
    path: fsl-package/src/emit/tests/hyper_clause.rs
    role: examined
    excerpt: >-
      
      /// FR-322-AC-511 "Hyper clause source unit": a hyper clause's
      `claim_names`

      /// row names the clause's own source unit, the one its
      `declaration_region`

      /// is under; every other row names none, and the hyper clause's

      /// `temporal_bindings` rows carry none.
  - id: fsl-package/src/emit/tests/protocol_requirements/fixture_w.rs
    path: fsl-package/src/emit/tests/protocol_requirements/fixture_w.rs
    role: examined
    excerpt: >2-
          let rows = crate::emit::temporal_binding_rows(&graph, &none, &none,
      &[], &[], &[])
              crate::emit::temporal_binding_rows(&graph, &none, &none, &[], &[], &selections),
  - id: fsl-package/src/emit/tests/reference_sets.rs
    path: fsl-package/src/emit/tests/reference_sets.rs
    role: examined
    excerpt: |-
      // SPDX-License-Identifier: AGPL-3.0-or-later
      //! Protocol sets through source checking and the actual package emitter.

      use super::*;

      #[trace("FSpec-FR-464-AC-3")]
      #[test]
      fn attempt_contract_sets_are_structural_across_source_permutations() {
          let declara
  - id: fsl-semantics/src/check/lowering/hyper.rs
    path: fsl-semantics/src/check/lowering/hyper.rs
    role: examined
    excerpt: "use filament_checked::node::NodeKind;

      \        // The atom slots the formula reaches: one `<p>@<t>` parameter
      node

      \        // per name, at levels `0..n` in ascending byte order of the name

      \        // (FSpec FR-402 \"Wrapper slot scope\"). A binder a leaf of the

      \ "
  - id: fsl-semantics/src/check/mod.rs
    path: fsl-semantics/src/check/mod.rs
    role: examined
    excerpt: "        .with_clock_places(&self.timed_models)"
  - id: fsl-semantics/src/model/clock_reference.rs
    path: fsl-semantics/src/model/clock_reference.rs
    role: examined
    excerpt: |-
      // SPDX-License-Identifier: AGPL-3.0-or-later
      //! The place-id codec shared by clock-reference evaluation and admission.

      use filament_exact::{NodeKey, ObjectId, ObjectReference, Value};

      /// Whether the target type is the native clock reference's exact target
  - id: fsl-semantics/src/model/intake.rs
    path: fsl-semantics/src/model/intake.rs
    role: examined
    excerpt: "        if type_value.get(\"scalar\").is_some() {

      \            return Err(ctx.malformed_in(\"scalar\", \"a native-target
      value type declares no scalar\"));

      \        }

      \        if let Some(constraint) =
      ctx.array_field(\"constraints\")?.first() {

      \            return Err(

      \ "
  - id: fsl-semantics/src/model/intake/model_unit.rs
    path: fsl-semantics/src/model/intake/model_unit.rs
    role: examined
    excerpt: "                if type_name == \"Reference\" &&
      tokens.get(index).is_some_and(|t| t.text == \"<\") {

      \                    let target = tokens.get(index + 1)?.text.clone();

      \                    if target == \"Clock\" && tokens.get(index + 2)?.text
      == \">\" {

      \           "
  - id: fsl-semantics/src/model/mod.rs
    path: fsl-semantics/src/model/mod.rs
    role: examined
    excerpt: pub mod clock_reference;
  - id: fsl-semantics/src/model/observation.rs
    path: fsl-semantics/src/model/observation.rs
    role: examined
    excerpt: >2-
          pub(crate) clock_places:
      std::sync::Arc<super::clock_reference::DeclaredClockPlaces>,
                  clock_places: std::sync::Arc::default(),
          clock_models: &'a [crate::check::TimedModel],
                  clock_models: &[],
          /// Supply the checked graph's d
  - id: fsl-semantics/src/model/observation/document.rs
    path: fsl-semantics/src/model/observation/document.rs
    role: examined
    excerpt: |2-
          /// Validated data references to declared clock places, never objects.
          clock_references: Vec<(String, ObjectReference)>,
                  clock_references: Vec::new(),
              if crate::model::clock_reference::is_clock_type(&declared) {
                  let dang
  - id: fsl-semantics/src/model/observation/document/tests/collection_elements.rs
    path: fsl-semantics/src/model/observation/document/tests/collection_elements.rs
    role: examined
    excerpt: "        clock_references: Vec::new(),"
  - id: fsl-semantics/src/model/observation/initial.rs
    path: fsl-semantics/src/model/observation/initial.rs
    role: examined
    excerpt: >2
          let references =
              crate::model::clock_reference::references_in(objects.iter().flat_map(|(_, slots)| {
                  slots.iter().filter_map(|(_, slot)| match slot {
                      FieldValue::Present(value) => Some(value),
                      _ => None,
  - id: fsl-semantics/tests/it/clock_places.rs
    path: fsl-semantics/tests/it/clock_places.rs
    role: examined
    excerpt: |-
      /// scalar.
      /// TC-3084's clock model with a canonical reference definition and a data
      /// field writable independently of the clock it names.
      fn clock_reference_document(target: &str) -> Vec<u8> {
          let mut document: Value =
              serde_json::from_slice(&
  - id: spec/functional/FR-106-admit-snapshots-and-invocations.md
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      A stored clock-place reference uses the same `reference` value form above:
      `population` names the owner's population and `key` carries the instance's
      clock-place identity, rather than an object key. When its static declared
      type is `Reference<Clock>` (the `sca
  - id: spec/test-cases/TC-464-snapshots-and-invocations-admit.md
    path: spec/test-cases/TC-464-snapshots-and-invocations-admit.md
    role: examined
    excerpt: "Scope: FR-106-AC-1, FR-106-AC-2, FR-106-AC-6, FR-106-AC-8,
      FR-106-AC-9,

      FR-106-AC-72, FR-106-AC-73.

      7. Compile the dense-time `Call` model of FR-1040 with clock fields `x`

      \   and `y`, data fields `r: Reference<Clock>`,

      \   `maybe: Option<Reference<Clock>>`,

      \   "
```


## Dispositions

Round 1 at 70f9b659638e36c7bd16754fe154b3fc716388ce.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Typed collection now distinguishes ClockRef leaves for field/context/parameter/result completeness and closure walks. Raw Clock inputs fail closed under the authorized missing-authority gate; this scoped fix does not establish the deferred external admission contract. |
| FND-002 | fixed | Read contexts retain object references only in closure/dependency lists; ClockRef values are refused by the raw gate before interpretation. The real context/parameter regression asserts the specific Unsupported reason, and the ordinary context ghost retains dangling_reference with field peer and object ghost. |
| FND-003 | deferred | Explicit architecture authorization accepts partial STEP1 only. Both raw source References::resolve and all three Bare observation APIs return typed Unsupported missing-route-neutral-place-identity, preventing the reproduced v1 ordinal alias. Actual supported Clock observation/evaluation parity is NOT fixed or claimed. QSL-2178 remains OPEN for the separately owned STEP2 route-neutral v2 contract and source/Bare parity; typed same-run Rust observations remain trusted under the required documented preconditions. |
| FND-004 | fixed | The common declared-place authority omits every Map-containing template. An active exact regression checks contains=false for a syntactically forged numeric slot on a real checked Map<Text, Clock> template and owner, independently of the raw Unsupported gate. |

## Round 1 Verification

Frozen disposition head: 70f9b659638e36c7bd16754fe154b3fc716388ce. Round: 1.

Explicit architecture ruling supplied by root authorizes STEP1 only: every raw transported Clock-reference observation on source and Bare routes refuses unknown_required_feature/unsupported-feature with reason missing-route-neutral-place-identity before interpreting v1 IDs. Only typed StateModel::observation/protocol_observation accept trusted states produced or already admitted in the same model run, with documented caller preconditions. No hostile Rust caller authentication, new provenance API, reader/pin/channel or v2 codec is authorized. QSL-2178 remains OPEN for source/Bare external route-neutral authority and map-keyed place IDs. AC72/73 remain unchanged and unmet.

Coder evidence inspected, no reviewer build: eight exact active tests each selected/passed 1 with 0 ignored (external-unsupported, map-forgery, internal-row, stored-row, ordinary-bare, timed-point, selector-bound, protocol-bound), logs /tmp/qsl-third-fix1-<name>.log. AC72/73 exacts each selected 1 and ignored 1, passed 0: /tmp/qsl-third-fix1-ac72-pending.log and /tmp/qsl-third-fix1-ac73-pending.log. Existing x-removed/y-retained red log demonstrates the old ordinal alias; it is not counted green. The stored-row exact now reaches trusted protocol_observation with actual generated c1/c2 references. Optional bootstrap starts absent; checked point and PointPresent restore the required successor reference set, exact reference values/profile 2/3 isolation/no resets/state-key assertions remain, and the actual optional member leaf is [0,4,0]. Protocol dependency edges exclude native Clock references only after observe_state validates them; ordinary missing-holder faults remain. IR manifest and lock unchanged at 1cbd86a48f02e4623e558a3d3b23c066c7f280bf. All fix commits and the full changed code/test/spec delta were inspected.

Scoped mergeability: ready under the explicit partial STEP1 authorization; no still-open finding in this round and no new finding. Original FAIL verdict/findings remain historical and immutable. Supported external Clock admission and full source/Bare parity remain deferred, not PASS.
