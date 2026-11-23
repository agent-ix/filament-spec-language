---
id: SR-2897001
title: "QSL-2897 part 1 code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2897; PR #3555 head 56853164ff1186c7a11e126a67833c883fc99cfd vs merge base 7ca2bd11ab9216e66dd595cf66878569d4032dfd; fsl-semantics/src/model/observation.rs, fsl-semantics/src/model/observation/document.rs, fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/clock_domain.rs, fsl-eval/src/simulation/model/domain.rs, fsl-eval/src/simulation/model/frames.rs, fsl-eval/src/simulation/model/input.rs, fsl-eval/tests/it/clock_reference_v2.rs, fsl-eval/tests/it/timed_subject.rs, fsl-analyze/tests/it/timed_refinement.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-106", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
---

## Summary

Ticket: QSL-2897 (part 1: Source-route Clock DATA admission and Clock-reference domain generation; the Bare route waits on QSL-2776). Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD`.

The main path is sound: `resolve_clock` decodes `place/v2`, checks owner, scope and declared template, and the field-locus fix gives typed steps. `Domain::of` takes clock candidates from the model system, and `clock_domain.rs` builds declared-place candidates and then appends the initial-snapshot seeds in a fixed order. Focused tests pass on titan: `fsl-eval --test it clock_reference` gives 4 passed and 6 ignored. Each of the six ignored tests, run with `--ignored`, gets past its Source half and fails only at the Bare call (`missing-type-representation`).

Defects: invalid selectors refuse with the wrong cause. The incomplete-population tolerance path cannot work as written. Removing the global refusal leaves the secondary enumeration routes with empty Clock domains. Candidate generation is unbounded eager work. The package route enumerates without a declaration table and does not refuse.

## Verdict

**NOT MERGEABLE as is.** Fix FND-001, FND-003 and FND-004 before merge. FND-002 and FND-005 are real but narrower; fix them in this PR or record the deferral. FND-006 and FND-007 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An identity whose selectors are invalid for the declared template (index at or above the declared maximum, wrong path position, missing or extra index) fails `is_declared_identity` and refuses `dangling_reference`/`absent-target-in-complete-population`; FR-106-AC-80 requires `invalid_runtime_input`/`invalid-value`. | fsl-semantics/src/model/observation/document.rs:1517 |
| FND-002 | medium | Incomplete-population tolerance builds the reference with `object_reference(views, clock_reference_target(), key)`, which resolves a universe for the native Clock type (no model view declares it) and maps the fault to `invalid-value`, so an unlisted owner is refused rather than tolerated. Had it resolved, the universe would not be the owner's (unlike `encode_identity`), and scope and template checks are skipped. No test covers this branch. | fsl-semantics/src/model/observation/document.rs:1502 |
| FND-003 | high | With `check_clock_reference_enumeration` removed, models with Clock-reference members reach the routes that call `Domain::of` directly with owner objects or `&[]`: `complete_open_fields` and `domain_values` (refine.rs), symbolic field, parameter and result domains, and probabilistic parameter vectors. For `Reference<Clock>` these now return `Some` of the clock references found in that list, usually empty, instead of `None`. A hidden or parameter Clock reference then ranges over no values, which silently loses states or vectors, against FR-106's "exactly the canonical place/v2 identities". | fsl-eval/src/simulation/model/refine.rs:372, fsl-eval/src/simulation/model/refine.rs:220, fsl-eval/src/simulation/model/symbolic.rs:280, fsl-eval/src/simulation/model/probabilistic.rs:687, fsl-eval/src/simulation/model/domain.rs:534 |
| FND-004 | medium | `build_clock_reference_candidates` builds every selector tuple (`product`) for every admitted owner and declared place as an eager `Vec` before any domain-size, value-bit or candidate budget applies. A declared maximum such as 1_000_000, or two nested bounded slots, allocates or loops without bound. | fsl-eval/src/simulation/model/clock_domain.rs:92, fsl-eval/src/simulation/model/clock_domain.rs:172 |
| FND-005 | medium | The package route sets `clock_declarations: None`, and `ModelSystem::new` no longer refuses, so its Clock domains hold only snapshot seeds and no declared places. Today the Bare `missing-type-representation` refusal hides this, but once QSL-2776 lands the route would enumerate an incomplete domain silently. FR-106 requires a typed refusal when owner-population authority is missing. | fsl-eval/src/simulation/model/input.rs:1036, fsl-eval/src/simulation/model/clock_domain.rs:59 |
| FND-006 | low | `clock_candidates` caches by `selectors.is_some()` only, so a second `TimedSystem` with a different `max_clocks` over the same `ModelSystem` reuses the first selector set. A declared maximum that does not parse as `u64` silently drops the template. | fsl-eval/src/simulation/model.rs:825, fsl-eval/src/simulation/model/clock_domain.rs:43, fsl-eval/src/simulation/model/clock_domain.rs:159 |
| FND-007 | low | The comment "The typed steps stay as the kernel gave them" is now false: the steps are prefixed with the snapshot field steps. This prefix applies to every object-field refusal, not only Clock ones. | fsl-semantics/src/model/observation/document.rs:3717 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | `ModelSystem::values_of` enumerates a composite or enum type through `composite_values` over the owner references only, without clock candidates. Protocol event, finish, channel, role and compensation record types that hold a `Reference<Clock>` now enumerate an empty or partial set, which silently drops events and records. On main they were refused (`Domain::of` was `None`, so `Space::of` was `None`, so the fault refusal fired). Because clock leaves now count as enumerable, `enumerable_in` / `unenumerable_paths` no longer make the protocol bound check (protocol.rs:879) refuse either. | fsl-eval/src/simulation/model/application.rs:116 |
| FND-009 | medium | Model-based-test parameter domains use `Domain::of_bounded(value_type, &[], ..)`. An `Option<Reference<Clock>>` parameter now enumerates only absence, and a record parameter with a Clock-reference leaf loses its present values. On main both refused `ValueDomain`. | fsl-eval/src/simulation/model/model_based_test.rs:1442 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `has_valid_selectors` is split out of `is_declared_identity`; an invalid selector now refuses `invalid_runtime_input`/`invalid-value` after the template check. `invalid_clock_selectors_refuse_invalid_value_on_source` passes on titan at 0b4a656a. |
| FND-002 | fixed | The unlisted owner is typed by the population's member types and goes through the same scope, template and selector checks and `encode_identity` as a listed owner. `an_unlisted_clock_owner_is_dangling_when_complete_and_tolerated_when_incomplete` passes. |
| FND-003 | fixed | `domain_values`, `complete_open_fields` and the symbolic field, parameter and result domains use `domain_with_clocks`, which returns real candidates or `None`. Probabilistic reward parameters return `None` for a Clock reference, which matches main. Two remaining routes are filed as FND-008 and FND-009. |
| FND-004 | fixed | The product size is charged against `max_candidates` before any identity is built, and over-limit returns `ClockDomainRefusal::TooMany`. `clock_reference_candidates_are_bounded_and_need_declaration_authority` passes. |
| FND-005 | fixed | With no table, `MissingAuthority` is returned and `check_model_bounds` gives `RequiresBound` with empty-slot `ClockReferenceDomain` sites; tested. |
| FND-006 | fixed | The cache is keyed by `Option<ClockSelectorMaxima>`, and an unparseable maximum is the typed `UnparseableMaximum`. It is untested because it needs a declared maximum above u64; accepted as a typed refusal. |
| FND-007 | fixed | The comment is rewritten to describe the field steps followed by the kernel steps. |
| FND-008 | fixed | `values_of` adds the Clock candidates to the objects it enumerates composites over, or returns `None`; `enumerates` and `unenumerable_paths` report the Clock leaves unenumerable when the candidates cannot be built. `a_protocol_event_record_ranges_a_clock_reference_over_the_declared_places` asserts two `Read` events (x and y) and `UnenumerableRecord` without a declaration table; both assertions fail on the reverted code. |
| FND-009 | fixed | A model-based parameter holding a `Reference<Clock>` is refused `invalid_model_binding`/`malformed-declaration`. `a_model_based_parameter_holding_a_clock_reference_is_refused` fails on the reverted code (a run over absence alone). |
