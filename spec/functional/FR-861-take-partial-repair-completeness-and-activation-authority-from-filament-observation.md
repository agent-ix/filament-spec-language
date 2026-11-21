---
id: FR-861
title: "Take the partial-value, repair, completeness and activation authority rules from filament-observation"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-007
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-008
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-010
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-013
    type: depends_on
---
# FR-861: Take the partial-value, repair, completeness and activation authority rules from filament-observation

## Description

`filament-observation` owns the observation-authority rules a temporal
monitor relies on: partial values and the order of uncertain event-time
intervals (its FR-007), activation-scope, progress and completeness
authority (its FR-008), and the affected-region plan of a revision repair
(its FR-010). FSL SHALL hold no implementation of those four rule sets of
its own; where it applies one, the implementation SHALL be
`filament-observation`'s. `fsl-eval` SHALL call the owner's `partial`,
`completeness` and `activation` authority modules where it applies those rules, as
`fsl-analyze` already calls the owner's `clock` and `progress` modules
(FR-251), and SHALL hold no second copy of them. `fsl-eval` applies no
affected-region planning (Behavior), so it holds no repair rule and calls
no `repair` module.

The temporal meaning stays FSL's. FR-501 (finite-window evaluation and its
counter cover), FR-502 (the progress ledger and its contradictions) and
FR-524 (instances, captures and activation dispositions) state what the
evaluator reports, and the owner's rules are the means it uses to decide the
observation-side facts those requirements read. FR-526 (the result chain)
reads no owner answer: it supersedes exactly the predecessor a request
names, which is why no affected-region plan is needed (Behavior).

### Decision: one owner, in filament-observation

Two implementations of the same authority rules drift, and an observation
fact accepted by one and refused by the other gives two answers for one
trace. The rules belong to the observation authority, which already
publishes them as validated contracts
(`filament.observation.partial-fact/v1`,
`filament.observation.completeness-assertion/v1`,
`filament.observation.activation-scope-authority/v1`,
`filament.observation.repair-plan/v1`), and FSL already depends on its
`clock` and `progress` modules. Keeping the owner there and deleting FSL's
copy leaves one implementation that every consumer shares. The edge it
needs, `fsl-eval` to `filament-observation`, adds no cycle:
`filament-observation` depends only on `fsl-foundation`, `filament-exact`,
`quire-canonical` and serialization crates, each below layer 5 (ADR-011).
FSL's crates are pre-release, so the superseded copy is deleted with no
shim and no re-export.

## Inputs

The observation facts the evaluator reads: a trace's stamps and cover
(FR-501), its progress and completeness assertions with the ledger entry
they revise (FR-502), and its deliveries, origins and capture facts
(FR-524).

## Outputs

The evaluator's reports, unchanged: FR-501's, FR-502's and FR-524's.

## Behavior

- `fsl-eval` SHALL name `filament-observation` in its `[dependencies]`
  (ADR-011's X-8 row lists it), at `branch = "main"`, through the workspace
  patch that keeps one `fsl-foundation` in the graph.
- Where the evaluator orders two uncertain event-time intervals or derives
  a partial value, it SHALL use the owner's `partial` rules.
- Where it accepts, revises or refuses a completeness assertion, it SHALL
  use the owner's `completeness` rules. The progress ledger (FR-502) takes
  every completeness answer from the owner's history-free revision check,
  `completeness::check_revision` (filament-observation FR-013), over what
  the ledger entry already retains: its assertion and its covered set with
  fingerprints. FR-944 "Completeness" states how the retained and asserted
  progress and the trace's positions map onto that check's arguments.
  `evaluate_temporal_evidence` (FR-516), the spine's batch run (FR-330) and
  carried evaluation (FR-820) all admit progress through that one ledger, so
  all three get the owner's answer from one function over one data source.
- Where it decides an activation scope's disposition and execution and
  evidence states, it SHALL use the owner's `activation` rules.
- `fsl-eval` plans no affected region of a revision. FR-526 supersedes
  exactly the predecessor the request names and recomputes every result
  from the request's own inputs, so the set of results a revision
  invalidates is that one predecessor by construction, and no rule decides
  it. Any affected-region planner FSL ever adds SHALL be the owner's
  `repair::plan` (filament-observation FR-010), and no FSL crate SHALL hold
  a planner of its own.
- No FSL crate SHALL hold a second implementation of any of the four rule
  sets: no type or function that restates the owner's partial-value
  algebra, completeness revision rule, activation-authority states or
  affected-region planning.
- The ledger's other progress checks stay FSL's and are outside the four
  rule sets: the watermark check (a regressing watermark, FR-502), which
  filament-observation FR-013 names the progress owner's rule, and the two
  closure checks (a retained `Closed` decision scope asserted `Open`, and a
  retained `Closed` surrounding execution asserted `Open`), which are
  FR-502's own rules over FSL's closures. The ledger runs the watermark
  check first, then `check_revision`, then the closure checks.
- Agreement tests SHALL run the evaluator's answer and the owner's on the
  same cases; a disagreement fails naming the case and both answers.
- Every existing temporal test in `fsl-eval` and `fsl-analyze` SHALL pass
  unchanged, except the tests of the criteria this requirement replaces:
  the former FR-944-AC-4 and FR-944-AC-16 selection and `revise` tests, the
  former FR-861-AC-4 tracked-exception test, and the repair tests of the
  former FR-861-AC-3 and FR-944-AC-3.
- Every FR-502, FR-820 and FR-993 test SHALL pass unchanged, except
  `a_retained_identity_at_one_stamp_does_not_cover_it_at_another_stamp`,
  which asserted stamp-keyed membership contrary to FR-502's "Ordering and
  membership use the identity only".
- That test SHALL assert that the identity is held and the request
  accepted.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-861-AC-1 | On FR-501-AC-13's counter-cover cases (`Counter { uncertainty: 1 }`, request at tick 0, `Complete { through: 3002 }`, watermarks 3001 and 3002), the interval order and partial value the evaluator uses equal those the owner's `partial` module derives for the same stamps. | Test (TC-2442) |
| FR-861-AC-2 | On FR-502-AC-2's cases (`Complete { through: 100 }` retained, then `Complete { through: 60 }`, `Incomplete` and an uncovered position stamped 80), mapped as FR-944 "Completeness" states, every acceptance and refusal of the evaluator's ledger is the answer of the owner's `completeness::check_revision` (filament-observation FR-013), with the refusal's typed cause read from that answer. | Test (TC-2442) |
| FR-861-AC-3 | On FR-524-AC-27 to AC-29's scopes, the owner's `activation` module gives the dispositions the evaluator reports. | Test (TC-2442) |
| FR-861-AC-4 | `fsl-eval`'s manifest names `filament-observation`, its source calls the owner's `partial`, `completeness` and `activation` modules, the progress ledger's source calls `completeness::check_revision`, and no FSL crate defines a type or function carrying one of the four rule sets of its own: a source scan of FSL's shipped source finds no function with two parameters of type `ProgressCompleteness` or comparing two `ProgressCompleteness` values, no use of `filament_observation::authority::repair`, and no item named for an affected-region plan, and fails naming the file and item of any it finds; a disagreement case of AC-1 to AC-3 fails its test naming the case and both answers instead of passing on either one. | Test (TC-2442) |

## Dependencies

- ADR-011 (the crate graph and its X-8 row), FR-251 (the existing use of the
  owner's `clock` and `progress`).
- FR-501, FR-502, FR-524: the evaluator behavior the owner's rules serve.
- FR-526: the result chain, which supersedes exactly the named predecessor,
  so no affected-region plan is needed.
- filament-observation FR-007, FR-008, FR-010, FR-013: the owned rules.
