---
id: "SR-17952730000002"
title: "QSL-2738 gap-analysis review"
type: "SpecReview"
analysis: "gap-analysis"
scope: "agent-ix/filament-spec-language; QSL-2738; spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md; spec/test-cases/TC-3013-systems-part-port-and-allocation-records-are-admitted.md; source systems compiler and protocol binding"
review_set: "subset"
---

## Summary

Ticket: QSL-2738. Frozen head 4e05bb700c82895b0791ca819f7c283f82cfe6cf; reviewed against incorporated main 7bb1249428 (merge b0143e7912). Exactly the supplied worktree was inspected; no source edits, builds, Cargo commands, checkouts, pushes or extra worktrees. Quoin 0.36.7, Quire 0.52.0 (engine 0.62.2), spec-artifacts-process@v0.29.2. No applicable AssuranceProfile exists (no type: AssuranceProfile document in spec/).

## Verdict

FAIL (repository coverage); changed slice clean. No new scoped merge-blocking defect found. Aggregate gate completion remains the lead's responsibility.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Existing FR-001-AC-7 has no computed test binder; repository-wide criterion coverage therefore does not pass. | spec/functional/FR-001-read-exact-source.md:273 |
| FND-002 | high | Existing FR-1021-AC-3 is bound only to an ignored todo! test; its behavior remains unverified under tracked QSL-1978. | fsl-analyze/tests/it/hybrid_requests.rs:1162 |

## Coverage

Repository matrix computation completed (exit 0): 6,136 criteria: 4,808 tagged, 104 method-without-symbol, 13 retired, 1,129 untagged and 82 tagged-by-ignored-test. FR-710-AC-11/12 and all newly tagged binding units are tagged. FND-001 records one concrete unchanged preexisting untagged criterion; FND-002 records an unchanged ignored todo! test explicitly tracked by QSL-1978; the remaining aggregate counts are not claimed individually adjudicated. The dispatch explicitly accepts the preexisting repository assurance backlog without changes for this scoped PR only. The global audit remains FAIL, independent of the clean changed slice. Plan completion: not assessed.

Reverse-gap/stub inspection scanned Rust files across the repository, including root, fsl-* crates, xtask and tools (repository-surface-scan.txt: 10,831 declaration/stub pattern matches, not a count of distinct semantically traced behaviors). Of 32 todo!/unimplemented! hits, 31 are documentation examples or fixture strings, and one is the explicitly ignored tracked test in FND-002. No newly introduced executable stub was found. The compiler, binder and diagnostics behavior examined in this diff map to FR-710, FR-086, FR-897, FR-001/002 and FR-027. Nine changed production files and one changed test file were read; the four tests use production seams and discriminating field/code/cause/span assertions, with no mocked compiler or hollow assertions. Changed-slice unowned behaviors: 0 found; changed-slice source stubs: 0; changed-slice test stubs: 0. Full semantic ownership of every public symbol in the repository and every existing test was not adjudicated, so no repository-wide reverse-gap/stub PASS is claimed. Optional gap semantic fan-out was not invoked; intent/test/code comparison needed by code review was performed directly for the changed slice. No formal coverage percentage was inferred from tags or logs.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-710-AC-11",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "examined",
      "excerpt": "TC-3013's source model compiles and its selected model records retain Parts `SysPump` and `SysTank`, their owner `Sys`, declared types and multiplicity, Ports `PumpOut` and `TankIn` with their owning Parts, directions, interface `Flow` and multiplicity, and Connection `Pipe` whose ends are those two Port keys, with source-to-target direction and `[1, 1]` unordered unique end multiplicities. Replacing `->` with `<->` and both Port directions with `inout` produces a bidirectional Connection admitted by FR-086."
    },
    {
      "id": "FR-710-AC-12",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "examined",
      "excerpt": "In TC-3013's source model, replacing `SysPump.PumpOut` by `Missing.PumpOut` or `SysPump.Missing` refuses `missing_declaration`/`missing-name` at `Missing`; `Sys.PumpOut` refuses `invalid_model_binding`/`wrong-export` at `Sys` requiring Part; `SysPump.Flow` refuses the same at `Flow` requiring Port; and `SysTank.PumpOut` refuses `invalid_model_binding`/`malformed-declaration` at `SysTank`, retaining `PumpOut`, written owner `SysTank` and declared owner `SysPump`. The equivalent mutations of the target end refuse at that end. Each refusal is located in the model source and produces no document."
    },
    {
      "id": "FR-710",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "examined",
      "excerpt": "When compiling a `connection C p.outlet -> q.inlet;` or its `<->` form,\nthe compiler SHALL resolve `p` and `q` as Parts and `outlet` and `inlet` as\nPorts of this model unit, checking that each Port's declared owning Part is\nthe Part written before its dot. The qualifier is an ownership assertion;\nit does not create a nested declaration identity. The compiled connection's\nends name the Ports' own `ix://<domain>/<port name>` identities. `->` writes\n`source-to-target`, and `<->` writes `bidirectional`; each end has\nmultiplicity `[1, 1]`, unordered and unique."
    },
    {
      "id": "FR-710",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "examined",
      "excerpt": "If a qualifier or Port name is absent, then the compiler SHALL refuse\n`missing_declaration`/`missing-name` at that name. If a qualifier names a\ndeclaration other than a Part, or an end names a declaration other than a\nPort, then the compiler SHALL refuse `invalid_model_binding`/`wrong-export`\nat that name, retaining required and actual kinds. If a Port belongs to a\ndifferent Part than the written qualifier, then the compiler SHALL refuse\n`invalid_model_binding`/`malformed-declaration` at the qualifier, retaining\nthe Port and the written and declared owning Part keys, with no document."
    },
    {
      "id": "FR-710",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "examined",
      "excerpt": "The model-unit compiler SHALL preserve the Part's owner, declared type and\nmultiplicity, and the Port's owner, direction, interface type and\nmultiplicity, for the unchanged classification and connection admission\nrules of [FR-086](FR-086-bind-systems-model-references.md). A consuming unit\nselects the compiled model with `model M = \"<domain>\";` and names its Ports\nand Connections as `M::outlet` and `M::C`, under\n[FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md) and\n[FR-897](FR-897-carry-a-protocol-channel-over-a-systems-connection.md).\nThis adds no systems declarations to a con"
    },
    {
      "id": "TC-3013",
      "path": "spec/test-cases/TC-3013-systems-part-port-and-allocation-records-are-admitted.md",
      "role": "examined",
      "excerpt": "Verify that model intake and the assembler admit a domain package's systems\npart, port and allocation records as model structure, that intake reads each\nwith its own fields, and that a unit protocol can bind a role to the admitted\nPort. The source-model variant follows Parts and Ports into a Connection\nused by a protocol channel. Scope: FR-103-AC-11, FR-086-AC-2,\nFR-897-AC-1, FR-897-AC-3, FR-897-AC-5, FR-710-AC-11 and FR-710-AC-12."
    },
    {
      "id": "FR-086-AC-2",
      "path": "spec/functional/FR-086-bind-systems-model-references.md",
      "role": "examined",
      "excerpt": "Given a Connection-classified relationship whose port-direction condition and whose per-end multiplicity condition both fail while the interface-type condition holds, `check_connection` reports both failing conditions (naming their own cause) and does not report the interface-type condition as failing; a relationship satisfying all three conditions is admitted."
    },
    {
      "id": "FR-897-AC-1",
      "path": "spec/functional/FR-897-carry-a-protocol-channel-over-a-systems-connection.md",
      "role": "examined",
      "excerpt": "Over FR-383-AC-62's model (`Sys` owning `pump_out` (out) and `tank_in` (in), connection `link` from `pump_out` to `tank_in`, source-to-target), `relationship Link = M::link;` with `channel Feed from Outlet to Inlet via Link carries Flow ...` checks and compiles. The channel node's `connection` argument references `link`'s relationship node, which its dependencies list, and the package is admitted."
    },
    {
      "id": "FR-897-AC-3",
      "path": "spec/functional/FR-897-carry-a-protocol-channel-over-a-systems-connection.md",
      "role": "examined",
      "excerpt": "`channel Back from Inlet to Outlet via Link` (the reverse of a source-to-target connection) refuses `invalid_model_binding`/`malformed-declaration` at `Link`, retaining the expected and actual ports. A `from` role on the component `Sys` refuses at that role's name."
    },
    {
      "id": "FR-897-AC-5",
      "path": "spec/functional/FR-897-carry-a-protocol-channel-over-a-systems-connection.md",
      "role": "examined",
      "excerpt": "`related by Link (a, b)` on a `send` refuses `ill_typed`/`operator-ineligible` at `Link` with the message naming `via`. `role R on M::link` still refuses as FSpec FR-465-AC-12 states."
    },
    {
      "id": "FR-001-AC-7",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key."
    },
    {
      "id": "FR-1021-AC-3",
      "path": "spec/functional/FR-1021-plan-over-a-hybrid-model.md",
      "role": "examined",
      "excerpt": "The `Fill`/`Drain` validation item for dwell 8 reaches the reference provider with its dwell and horizon as `{\"numerator\": \"8\", \"denominator\": \"1\"}`-form `Rational` objects, and a fixture provider whose answer writes a trace bound as `\"8\"` makes that item refuse `invalid_result`/`wrong-value-kind` at the member, so the candidate is not validated."
    }
  ],
  "bindings": [
    {
      "test_id": "systems_source_compiles_to_the_same_checked_package_as_its_document",
      "ac_id": "FR-710-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "systems_source_compiles_to_the_same_checked_package_as_its_document",
      "ac_id": "FR-897-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "systems_source_rejects_each_invalid_endpoint_before_writing_a_document",
      "ac_id": "FR-710-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "systems_source_refuses_reversed_channel_and_non_port_roles",
      "ac_id": "FR-897-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "systems_source_refuses_reversed_channel_and_non_port_roles",
      "ac_id": "FR-897-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "systems_source_preserves_independent_connection_admission_causes",
      "ac_id": "FR-086-AC-2",
      "trace": "correct"
    }
  ]
}
```

## Evidence and limits

The exact inherited runtime model ID is unavailable to both reviewer and lead; model=unavailable records this metadata limitation.

Supplied evidence: /tmp/qsl-2738-command-frozen.log has four passing command tests, with two accepted variants and 30 negative variants (22 endpoint, five role/channel, three independent admission causes), not 30 tests. /tmp/qsl-2738-source-regression.log has one passing existing systems-source regression. /tmp/qsl-2738-allocation-supported.log runs exactly systems_part_port_allocation_records with --features test-support: one pass. The earlier default-feature allocation command selected zero tests and is not proof. /tmp/qsl-2738-clippy-core.log succeeds for CST/semantics libraries. Replay-library and command-test Clippy exit 101 with four unchanged incorporated-main diagnostics (product.rs:1047,1054,1062 filter_map_bool_then; refinement_replay.rs:1665 let_and_return); /tmp/qsl-2738-unchanged-base.diff is empty for those files versus 7bb1249428. No baseline build was run, so this is unchanged-source evidence, not a passing baseline gate. Refinement let-and-return was reported fixed by later main; later-main integration was not reviewed. Existing linker and integration-test warnings remain recorded in the supplied logs. No aggregate make ci, full feature matrix or full gate was run or claimed.

Plan completion: not assessed

Scope excludes inheritance and original/effective identity migration. V1-TYPE-025 is identity; no claim is made that this ticket implements or expands identity. No other optional spec lenses were selected: dependency (no edge change), scope-boundary/domain-object (unchanged responsibility/objects), risk-complexity (no new architecture), criterion-strength (not selected; no Jev run).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The dispatching lead explicitly accepts the preexisting repository assurance backlog without change for QSL-2738 only. FR-001-AC-7 and its source file are unchanged versus incorporated main 7bb1249428; global coverage remains FAIL and no repository assurance closure is claimed. |
| FND-002 | accepted-no-change | The dispatching lead explicitly accepts preexisting assurance backlog without change for this scoped PR only. The ignored todo! test and its FR-1021-AC-3 binding are unchanged and explicitly owned by QSL-1978; no hybrid-plan implementation or qualification is claimed by QSL-2738. |

Both dispositions apply solely to this PR under the lead's explicit dispatch terms. They do not change the global FAIL verdict. No semantic fix round was required.
