---
id: TC-3036
title: "The evaluator derives every observation admission selection"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-944
    type: verifies
---
# TC-3036: The evaluator derives every observation admission selection

## Description

Verify that `fsl-eval` builds the filament-observation admission request from
the checked package, the clause occurrence, the trace with its clock profile
and the caller-owned producer bundle passed to it alone, that the owner admits it,
that an owner refusal of that request is an internal failure and an owner
maximum a limit, and that the package selection and its in-memory digest
are equal on the source and bare-package routes. Scope: FR-944-AC-8,
FR-944-AC-9, FR-944-AC-10, FR-944-AC-11, FR-944-AC-12, FR-944-AC-13, FR-944-AC-15 and
the equal-content, new-identity case of FR-944-AC-4.

## Test Procedure

1. Build the admission request for FR-501-AC-13's counter-cover request under
   `ms`, with one position carrying arrival 2, one an order key and one an
   interval stamp; call `admit`; build the request a second time; vary one
   valuation, one order key and one interval, each alone; add a second
   observation equal to the first in content, stamp, key, interval and
   arrival under another FR-106 identity; inspect a position without an
   arrival. Build the request for an event-position clause.
2. Build the `fsl-eval` producer bundle with its constructor, admit it once as
   the caller, pass it by reference to two runs and compare their requests'
   producer selections; scan `fsl-eval`'s shipped source, and a fixture source holding
   `static B: OnceLock<AdmittedStaticBundle>`, for a static, lazy,
   thread-local or locked holder of `StaticProducerBundle` or
   `AdmittedStaticBundle`;
   mutate a derived request (the binding source, the population identity, one
   record identity), each alone, before `admit` and run it through the
   evaluator; run a trace of `max_population_entries + 1` positions and a
   trace with one position whose value text is one byte over
   `max_string_bytes`.
3. Compile one package, evaluate it on the source route, emit it
   canonically, read the bytes back and evaluate on the bare-package route;
   feed the reader a reordered, whitespace-padded copy; inspect every
   document and file the run writes.
4. With a ledger retaining `Complete { through: T }` that covers one
   position, deliver a position equal to it in content, stamp, order key,
   interval and arrival under a new FR-106 identity, then the same position
   under its covered FR-106 identity.
5. Run FR-516-AC-9's `EvidenceRef` request through
   `evaluate_temporal_evidence` with a `Value` sample of 1/3 and one of
   18446744073709551616/3, a division-by-zero `Undefined` valuation, an
   `InsufficientData { count: 1, min_count: 2 }` sample and an
   instance-bound valuation under the trigger identity bytes `0x00 0xff`; vary one content member, one
   valuation, one sample, one instance valuation and one label, each alone;
   add a position equal to another in all but its `EvidenceRef` identity;
   repeat step 4 with that position. Inspect a spine position of step 1.
6. Over the three positions of FR-327-AC-5's closed trace, supply the owner
   the valuations of its first clause's atom `p`: `true`, `true` and kernel
   `division-by-zero` `Undefined` at position 2; derive the partial facts.
7. Admit FR-501-AC-13's request (clock unit `ms`) and an event-position
   request; derive every partial fact; read the FSL report.

## Expected Results

- Step 1: every member equals FR-944-AC-8's value, each record value is
  the seven-member object with `aggregate_samples`,
  `instance_aggregate_samples`, `instance_valuations`, `interval`,
  `observation` (the document without `identity`), `order_key` and
  `valuations` (read from the document), and `admit` returns `Available`; the two builds are equal; each variation changes exactly that
  record's identity; no record value carries an FR-106 identity; the equal
  observation admits as the same single record and member, and reports
  about it name both FR-106 identities; the arrival-less position carries
  its event time as ingestion (interim rule) and no report carries it; the
  event-position request uses `EventPosition` anchors and range, owner unit
  `position/4`, and is `Available`.
- Step 2: the bundle members and digests equal FR-944-AC-9's; each request's
  `producer` is a clone equal to the passed bundle; the scan finds no holder
  in `fsl-eval` and reports the fixture's line; each mutation ends `runtime_invariant`/
  `established-invariant-broken`, invariant `observation-admission`, with the
  owner's refusal cause, exit `30`, no clause outcome; the two over-maximum
  traces refuse `stage_limit_exceeded` with `node-count-exceeded` and
  `input-bytes-exceeded`, the owner maximum as `bound`, and the
  `filament_observation::authority::Limits` member as `setting`.
- Step 3: `canonical_bytes()` equals the emitted and consumed bytes exactly
  on both routes; equal `package` selections (revision
  `filament.checked-package/v2`, equal digest) and `Context` authorities;
  the non-canonical copy is refused `noncanonical_wire` and admits nothing;
  no document or file FSL writes carries the digest.
- Step 4: the position under the new FR-106 identity is refused
  `UncoveredMember` by `check_revision`, a contradiction on `Completeness`,
  `invalid_runtime_input`/`invalid-value`, with the clock name, the retained
  assertion and the position, the ledger unchanged, although it shares the
  covered position's owner record identity; the position under its covered
  identity is accepted.
- Step 5: each value is the stated seven-member object with the label-free
  four-member `observation` in lowercase hexadecimal, the rationals with
  unbounded components, `{"cause": "kernel", "reason": "division-by-zero"}`,
  `{"cause": "insufficient-data", "count": "1", "min_count": "2"}`, the
  instance valuation keyed `"00ff"`, and `admit` returns `Available`; each content,
  valuation or sample change alone changes exactly that record's identity
  and a label change does not; the equal positions share one record whose
  reports name both identities; the position under a new identity at or before
  the retained `through` is refused `UncoveredMember`; the spine position retains its FR-106 document and its
  `observation` is that document without `identity`.
- Step 6: the undefined position's record value carries
  `{"undefined": {"cause": "kernel", "reason": "division-by-zero"}}` for
  `p`, no partial fact is derived for `p` there, the evaluator reports its
  own undefined outcome unchanged, no owner answer for it is `Missing`,
  `Conflicting` or `Indeterminate`, no owner read consumes `p`'s possibility
  set there, and the clause outcome is
  `Undefined{where: 2, cause: division-by-zero}`; positions 0 and 1 have
  their facts read back.
- Step 7: binding, record and interval units are all `ms/4` (all
  `position/4` for the event-position request) and every `partial::derive`
  succeeds; the FSL report names `ms`.
