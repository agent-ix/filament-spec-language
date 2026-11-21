---
id: SR-20261121003316002
title: PR 3316 spec-review/base of model counterexample contract
type: SpecReview
analysis: base
scope: agent-ix/filament-spec-language; spec/functional/FR-267-write-run-results-as-native-run-result-2.md,
  spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md, spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md,
  spec/test-cases/TC-523-replay-a-model-counterexample.md, spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md
review_set: subset
---

## Summary

Ticket: QSL-2726. Checked changed requirement statements and four added criteria against the base checklist and TC procedures. Positive source/package model witnesses, canonical read/write parity, malformed wire and semantic replay controls have nonvacuous independent oracles. No new digest, file-tracking or pin requirement is introduced.

## Verdict

PASS. No defects found in the approved existing-model-arm contract. Implementation completion and global coverage are not assessed.

## Review context

Frozen candidate: 2e342571f9c18f125a46f232f8be1b8099432c77; five specification artifacts, +161/-12. Pinned FSpec context was read by git show at 247a394157b58843643a61dfde16280c25d733c0, including the actual backend-provider schema required interval and closed ModelStep/State/TransitionIdentity forms. Inspected existing request decoder, evaluate_model_trace and native model Undefined omission policy as grounding, without claiming they implement the proposed contract. No applicable AssuranceProfile exists in the local spec tree; direct review applies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-267-AC-30
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: FR-330-AC-19's model violations write canonical `native-run-result/2` documents
    and read back equal typed model-arm counterexamples, preserving initial index,
    typed binding, fairness, transition identities, state keys, loop boundary, interval
    and report `trace_position`; a later failing activation keeps its nonzero position.
    The initially terminal case round-trips its actual terminal key. A model `Undefined`
    document with no counterexample reads back with its reason and position; any carried
    undefined witness round-trips the model state-key envelope and actual truncated
    prefix. AC-13's snapsho
- id: FR-267-AC-31
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: The strict reader refuses AC-30's model document with `initial_state` null,
    a decimal string, negative or above the `Count` maximum; with null or malformed
    typed-reference binding; with a document or sample step in either prefix or loop;
    with a mixed step, unknown member, missing transition member or malformed base64url
    state key; or with `trace_position` unequal to the report position. It refuses
    a carried undefined model envelope with `document` or `sample` instead of `state_key`,
    and AC-13's observed document with a model step, non-null initial state or non-null
    binding. Removing `counterex
- id: FR-267:changed-1-1
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: '| `undefined` | `Undefined` with a kernel or family cause | `position`,
    `counterexample` (may be absent for a model trace''s `Undefined`, FR-330) | `violation`,
    `false`, `closed-scope` | `{"kind": "undefined", "reason": R}`, `R` the undefined
    value''s FR-100 reason |'
- id: FR-267:changed-2-1
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "  Outputs). For snapshot and record-sample traces it is written as\n \
    \ FSpec FR-364's observed arm (`trace: \"observed\"`,"
- id: FR-267:changed-3-1
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "  there. An observed run whose `Undefined` verdict has no recorded first\n\
    \  undefined reading at its position, or any `Completed(false)` violation\n  whose\
    \ counterexample is absent, is reported as an internal failure rather\n  than\
    \ written without one. The model `Undefined` exception below preserves\n  its\
    \ existing cause and position without requiring an observed witness."
- id: FR-267:changed-4-1
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "- For a `TraceInput::Model` result, `counterexample` SHALL use FSpec\n\
    \  FR-364's model arm and the existing native `interval` member. The pinned\n\
    \  backend-provider schema's `TemporalCounterexample`, `ModelStep`, `State`\n\
    \  and `TransitionIdentity` definitions specify the model members: non-null\n\
    \  `initial_state` in `Count` form; non-null `over_binding` holding the actual\n\
    \  typed reference for this authored clause; and closed steps\n  `{transition,\
    \ post_state}`, with `post_state` the closed `{key}` containing\n  unpadded base64url\
    \ canonical state-key octets. An ordinary transition has\n  exactly `ty"
- id: FR-267:changed-4-2
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "pe: \"transition\"`, `operation`, `receiver` and `arguments` in\n  their\
    \ existing typed forms; a terminal stutter has exactly\n  `{\"type\":\"terminal-stutter\"\
    }`. FR-330 supplies these members from the\n  verified taken behaviour, including\
    \ the actual key of an initially\n  terminal state; the writer SHALL NOT derive\
    \ them from invented observations\n  or unvalidated request keys. `trace_position`\
    \ retains the report's\n  `position`, never a model serializer's default. A carried\
    \ undefined model\n  envelope names `state_key` in place of `document` or `sample`\
    \ and retains\n  the actual truncated prefix, locus "
- id: FR-267:changed-4-3
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "and cause. A model trace's `Undefined`\n  MAY omit `counterexample` while\
    \ keeping its reason and position; a\n  `violated` row SHALL always carry one.\
    \ This authored-clause binding rule\n  does not change the null binding of FSpec's\
    \ generic deadlock arm, which is\n  not a temporal clause-run result.\n- The native\
    \ reader SHALL discriminate counterexample arms by `trace` and\n  restore the\
    \ equal typed payload. It SHALL refuse, naming the offending\n  member, a model\
    \ arm with null or non-`Count` `initial_state`, null or\n  malformed typed-reference\
    \ `over_binding`, a `document` or `sample` step,\n  a malfor"
- id: FR-267:changed-4-4
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "med transition or state key, or an undefined envelope carrying\n  `document`\
    \ or `sample`; an observed arm with a model step, non-null\n  `initial_state`\
    \ or `over_binding`, or an envelope carrying `state_key`;\n  and any mixed step\
    \ or extra member. Existing observed-step validation\n  remains unchanged. Structural\
    \ reading checks the closed typed shape;\n  request-bound initial-state membership,\
    \ binding/type compatibility,\n  enabledness, successor selection, fairness and\
    \ loop closure are checked\n  by FR-128 replay against the same admitted subject\
    \ and package."
- id: FR-267:changed-5-1
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: "  member the row does not list, or one it lists absent or `null` except\n\
    \  the expressly permitted model `Undefined` counterexample omission; a"
- id: FR-267:changed-6-1
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: '| Temporal | `counterexample` | FSpec FR-364, FR-330, FR-128, FR-331 |
    Observed- or model-arm `TemporalCounterexample` the report holds, selected by
    trace input | Restore the closed typed arm; replay authenticates subject semantics
    |'
- id: FR-330-AC-19
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: 'AC-16''s `Model` violation through source and checked-package execution
    carries equal model-arm counterexamples: non-null initial index `0`, the actual
    typed `over` reference, checked fairness and the validated transition/post-state
    sequence with its original loop boundary; `trace_position` equals the report position
    and `decisive.position` remains 2. A clause whose first failing activation is
    later than 0 keeps that later `trace_position`. A terminal initial state with
    no prefix steps carries one terminal-stutter loop step keyed by that actual initial
    state. A model `Undefined` run preserves i'
- id: FR-330-AC-20
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: With the same checked package, admitted initial snapshot, universes and
    request bytes available after source files are deleted, AC-19's source and checked-package
    model witnesses reproduce through FR-128's model replay contract with equal evaluated
    values and positions. Over FR-128-AC-1's branching subject, the recorded post-state
    key selects the actual taken successor. A different valid state key that is not
    a successor of the recorded transition, a disabled transition, an object binding
    outside the admitted subject, an unfair loop and a loop that does not close each
    refuse replay; structural
- id: FR-330:changed-1-1
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: '`TemporalCounterexample`, subject to the model `Undefined` exception below.

    For snapshot and record-sample traces this is

    FSpec FR-364''s observed arm, which the entry'
- id: FR-330:changed-2-1
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: 'For `TraceInput::Model`, a `Completed(false)` violation SHALL instead
    carry

    FSpec FR-364''s existing model arm. The entry SHALL retain the behaviour it

    actually re-executed and validated under FR-329: the transition identity and

    canonical post-state key of each taken step, the prefix/loop boundary, the

    checked clause''s fairness set and the actual typed reference bound to `over`.

    Its non-null `initial_state` SHALL select the admitted initial state in the

    request''s ordered initial-state list; for this input''s single `initial`

    snapshot that index is `0`. Its `trace_position` SHALL equal the report'
- id: FR-330:changed-2-2
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: '''s

    failing activation position, including when that position is not zero, and

    SHALL NOT be replaced by the decisive atom''s position or a serializer''s

    default position. A terminal-stutter loop SHALL carry the canonical key of

    the actual replayed terminal state, including when the initial state is

    terminal and the prefix is empty. A supplied terminal-stutter key that the

    input decoder did not validate SHALL NOT supply that key. The entry SHALL

    create no observed document or document identity for a reconstructed model

    state. The native `interval` member retains FR-267''s existing contract.'
- id: FR-330:changed-3-1
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: 'A model trace''s `Undefined` result retains its cause, position and exit
    10

    and MAY omit a counterexample; this exception does not permit omission from

    a `Completed(false)` violation. If the entry carries an undefined model

    counterexample, it SHALL use the model arm, the actual transitions reaching

    the failing position as its finite prefix, an empty loop and the envelope

    `where` containing that position''s actual `state_key` and undefined locus,

    with the recorded cause. It SHALL NOT require or invent an observed identity.

    The observed-arm construction and FR-331 replay route remain unchanged.'
- id: TC-1160:changed-1-1
  path: spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md
  role: examined
  excerpt: FR-327-AC-6, FR-330-AC-7, FR-267-AC-30 and FR-267-AC-31.
- id: TC-1160:changed-2-1
  path: spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md
  role: examined
  excerpt: "9. Write TC-840 step 7's model reports from both routes, read each through\n\
    \   the native strict reader and compare the typed payloads and RFC 8785\n   re-encoded\
    \ bytes. Include the later failing activation and initially\n   terminal case.\
    \ Write TC-840 step 8's model undefined report without a\n   counterexample; when\
    \ an undefined model witness is carried, also read\n   and re-encode its finite\
    \ prefix and state-key envelope. Keep step 2 and\n   FR-330-AC-18's sample witness\
    \ as observed-arm controls.\n10. Independently apply each malformed edit of FR-267-AC-31\
    \ to step 9's\n    model documents and the o"
- id: TC-1160:changed-2-2
  path: spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md
  role: examined
  excerpt: "bserved control; strictly read every edited\n    document. Test document\
    \ and sample steps in both prefix and loop, mixed\n    step members, null and\
    \ mistyped initial indexes and bindings, malformed\n    state keys and transition\
    \ members, misplaced undefined identity members,\n    changed `trace_position`,\
    \ and omission of a violated row's witness.\n    Keep well-shaped but semantically\
    \ foreign keys and bindings for TC-523's\n    replay checks rather than treating\
    \ structural read as subject validation."
- id: TC-1160:changed-3-1
  path: spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md
  role: examined
  excerpt: "- Step 9: equal typed model payloads and canonical bytes retain the actual\n\
    \  initial index, binding, fairness, taken transitions, post-state keys,\n  prefix/loop\
    \ boundary, interval and report position. No-witness model\n  undefined retains\
    \ its reason and position; a carried witness retains the\n  actual finite prefix\
    \ and state-key envelope. The observed controls retain\n  their existing closed\
    \ observed shapes.\n- Step 10: every malformed document refuses with a located\
    \ diagnostic naming\n  the offending member. Removing a violated row's counterexample\
    \ refuses;\n  accepting the existing model undefine"
- id: TC-1160:changed-3-2
  path: spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md
  role: examined
  excerpt: "d omission does not make that witness\n  optional. Semantic replay controls\
    \ remain separately asserted by TC-523."
- id: TC-523:changed-1-1
  path: spec/test-cases/TC-523-replay-a-model-counterexample.md
  role: examined
  excerpt: "  - target: ix://agent-ix/filament-spec-language/FR-330\n    type: verifies"
- id: TC-523:changed-2-1
  path: spec/test-cases/TC-523-replay-a-model-counterexample.md
  role: examined
  excerpt: 'Scope: FR-128-AC-1 to FR-128-AC-4 and FR-330-AC-20.'
- id: TC-523:changed-3-1
  path: spec/test-cases/TC-523-replay-a-model-counterexample.md
  role: examined
  excerpt: "5. Take TC-840 step 7's source and checked-package model-arm witnesses.\n\
    \   Keep the checked package, original admitted initial snapshot, universe\n \
    \  inputs, authoritative byte provision and request identity; delete the\n   original\
    \ source files. Replay the witnesses through FR-128's model route\n   using those\
    \ retained bytes, including its existing package/source\n   authentication. Repeat\
    \ the branching case with the actually taken\n   successor's key and the initially\
    \ terminal empty-prefix case.\n6. Independently replace a recorded post-state\
    \ with a well-shaped key of a\n   state that is not a succ"
- id: TC-523:changed-3-2
  path: spec/test-cases/TC-523-replay-a-model-counterexample.md
  role: examined
  excerpt: "essor of that transition, replace a transition\n   with a disabled transition,\
    \ replace the binding with a typed reference\n   outside the admitted subject,\
    \ break loop closure and supply an unfair\n   loop. Preserve the rest of each\
    \ request's identity and inputs. Use the\n   structural malformed-arm controls\
    \ from TC-1160 separately."
- id: TC-523:changed-4-1
  path: spec/test-cases/TC-523-replay-a-model-counterexample.md
  role: examined
  excerpt: 'Tag the tests `#[trace("TC-523", "FR-128-AC-n")]` and the added model-run

    replay checks `#[trace("TC-523", "FR-330-AC-20")]`.'
- id: TC-523:changed-5-1
  path: spec/test-cases/TC-523-replay-a-model-counterexample.md
  role: examined
  excerpt: "- Step 5: replay reproduces the actual model witness from retained bytes,\n\
    \  with equal evaluated values and trace positions for source and package\n  routes,\
    \ even with no source file remaining. The branch key selects the\n  actual successor;\
    \ the initially terminal case replays its actual terminal\n  key without requiring\
    \ a fabricated observed snapshot.\n- Step 6: every semantic defect refuses replay\
    \ with no reproduced result.\n  A native reader's acceptance of a well-shaped\
    \ state key or typed binding\n  does not count as successful model re-execution\
    \ or subject authentication."
- id: TC-840:changed-1-1
  path: spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: 'Scope: FR-330-AC-1 to FR-330-AC-4, FR-330-AC-12 to FR-330-AC-17 and

    FR-330-AC-19.'
- id: TC-840:changed-2-1
  path: spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: "7. Run AC-16's `Model` trace through source and checked-package execution.\n\
    \   Compare the carried counterexamples with the actual states and transition\n\
    \   identities reconstructed by model replay, including the typed `over`\n   reference\
    \ and fairness. Repeat with an `on each` clause whose first\n   failing activation\
    \ is after position 0, and with a terminal initial state,\n   no prefix steps\
    \ and a one-step terminal-stutter loop. For the terminal\n   case, supply a different\
    \ syntactically valid key on the request stutter;\n   if admission accepts it\
    \ without validating that key, check that the\n   out"
- id: TC-840:changed-2-2
  path: spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: "put uses the actual replayed terminal key.\n8. Run a model trace whose\
    \ atom evaluates undefined at a reconstructed state.\n   Check the cause, position\
    \ and exit with no carried counterexample. If the\n   route carries one, check\
    \ its finite model prefix and failing state key;\n   repeat through source and\
    \ checked-package execution. Retain steps 1 and\n   6 and FR-330-AC-18 as snapshot\
    \ and record-sample controls."
- id: TC-840:changed-3-1
  path: spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: "- Step 7: both routes report a violation with equal model-arm witnesses,\n\
    \  initial index 0 and non-null actual typed binding, the validated taken\n  steps\
    \ and original loop boundary. `trace_position` equals the failing\n  activation,\
    \ while `decisive.position` remains the decisive atom's position.\n  The later\
    \ activation keeps its nonzero position. The terminal initial case\n  carries\
    \ one stutter step with the actual initial state's canonical key,\n  including\
    \ when a supplied unchecked stutter key differs. No reconstructed\n  state receives\
    \ an invented document identity.\n- Step 8: cause, position and"
- id: TC-840:changed-3-2
  path: spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: " exit 10 survive without an observed witness.\n  Any carried undefined\
    \ witness uses `trace: \"model\"`, the actual prefix\n  reaching the failing state,\
    \ an empty loop and `where.state_key` of that\n  state with its recorded locus\
    \ and cause. Snapshot and record-sample\n  controls keep their existing observed-arm\
    \ forms and outcomes."
- id: FR-128
  path: spec/functional/FR-128-replay-a-model-counterexample.md
  role: context_only
  excerpt: 'FSL SHALL replay a model counterexample (FR-126) through the layer-6 replay

    facade (FR-098) at ADR-011 E9: `fsl_replay::replay_model_trace` takes

    FR-098''s request with the subject''s inputs and a

    `WitnessEnvelope<TemporalCounterexample>` whose source is

    `ReplaySource::ModelTrace`. It recompiles the package, re-admits the subject,

    re-executes each step through FR-120''s `ModelSystem` with FR-101 `replay`,

    checks the lasso against the fairness set, and evaluates the formula over

    the replayed trace by FR-125, or for a deadlock counterexample checks the

    final state (ADR-018 CX-3, DL-5). It settles a'
- id: FR-329
  path: spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md
  role: context_only
  excerpt: 'When the TemporalTrace evaluator (FR-327) evaluates a clause under

    `filament.temporal.infinite-trace/v1` over a lasso, it SHALL first check the

    lasso against the clause''s fairness set, refuse a lasso that is not an

    admitted trace, and evaluate a fair lasso exactly (ADR-014 §5 A-4). It SHALL

    read positions past the represented lasso by ADR-018 SM-8, including past

    operators and interval operators (IV-2). This lasso evaluation is the

    semantics that the model checker''s refutations are replayed through

    (ADR-018 SM-1, SM-7). Under a bounded profile, the evaluator SHALL read a

    lasso in one case only'
```

## Validation

The five changed artifacts passed explicit spec-artifacts-iso validation with exit 0; 5/5 grammar-clean and zero grammar findings. git diff --check passed. Current installed tools: quire 0.52.0 (engine 0.62.2), quoin 0.36.8, spec-artifacts-process@v0.36.0. This differs from the session release notice and was reported to the lead; no older binary or cached contract was substituted.
