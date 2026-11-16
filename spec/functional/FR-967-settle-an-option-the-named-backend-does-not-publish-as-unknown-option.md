---
id: FR-967
title: "Settle an option the named backend does not publish as unknown-option on every provider-routing entry"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-960
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-961
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-779
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-272
    type: depends_on
---
# FR-967: Settle an option the named backend does not publish as unknown-option on every provider-routing entry

## Description

An item that names a backend may carry `options`, the backend's option names
and string values (FSpec FR-196, FR-331). Every provider-routing entry
(FR-635 "One rule for every provider-routing entry": `analyze`, `prove`,
`monitor`, the run's requirement items of FR-961, `replay` and `generate`)
SHALL settle an item whose `options` hold a name the named backend's manifest
does not publish as `invalid-request` with the diagnostic
`invalid_capability`/`unknown-option` (FSpec FR-779, FR-272).

The option is checked against a backend's published capability: the
manifest's `options`, which the registry snapshot holds. A name outside them
is a request for a capability the backend does not offer, the same class of
defect as a named backend the registry does not hold (`unknown-backend`), so
it carries an `invalid_capability` cause and the `invalid-request`
disposition. It is not a runtime input defect: no value the item evaluates is
wrong, so `invalid_runtime_input`/`invalid-value` would misclassify it.

## Inputs

The item's named backend and `options`; the registry snapshot.

## Outputs

The item's `invalid-request` settlement, or no change to its routing.

## Behavior

- The entry SHALL check an item's options after FSpec FR-290's kind
  admission (`absent-kind`, `unknown-kind`) and its `inapplicable-kind` rule,
  and after resolving its named backend, and before computing its
  candidates, so each item has exactly one outcome: an item those rules
  settle is not checked for options. An item whose named backend the
  registry does not hold settles `unknown-backend` (FR-075) and its options
  are not checked.
- When a named option is not one the named backend's manifest publishes, the
  entry SHALL settle the item `invalid-request` with
  `invalid_capability`/`unknown-option`, payload `request_index` (the item's
  FSpec FR-331 request index), `backend` (the named backend) and `option`
  (the least unpublished name in byte order); it SHALL NOT negotiate the item,
  send it to a provider or run an engine for it.
- The refusal SHALL settle only that item: other items of the request, and
  the value of a run's call (FR-961), are unchanged.
- An item whose every option the backend publishes carries the options into
  its FSpec FR-331 request item unchanged.
- An item's exit and category are those of an `invalid-request` item (exit
  20), as for `unknown-backend`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-967-AC-1 | An `analyze` request over the Counter model with two items, the first naming `backend` `en-1` and `options` `{"zeta": "1", "alpha": "2"}` (names EN-1's manifest does not publish) and the second naming `en-1` with no options, settles the first `invalid-request` `invalid_capability`/`unknown-option` with `request_index` 0, `backend` `en-1` and `option` `alpha`, exit 20, never `invalid_runtime_input`, and runs EN-1 for the second only, whose record is byte-equal to the same request without the first item. | Test (TC-2724) |
| FR-967-AC-2 | An `analyze` item naming `backend` `fsl.absent` with `options` `{"k": "v"}` settles `unknown-backend` naming `fsl.absent` and not `unknown-option`; with the workspace test plugin registered as a process provider whose manifest publishes the option `depth`, an item naming it with `options` `{"depth": "3"}` reaches the provider with that option in its FSpec FR-331 request item, and `{"depth": "3", "width": "1"}` settles `unknown-option` with `option` `width` and sends the provider no request. | Test (TC-2724) |

## Dependencies

- FR-635: the provider-routing entries. FR-960, FR-961: the run's items.
- FR-075: the named-backend candidate rule.
- FSpec FR-779: the standard's unknown-option rule; FSpec FR-272: the cause.
