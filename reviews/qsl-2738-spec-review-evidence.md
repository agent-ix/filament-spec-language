---
id: "SR-17952730000007"
title: "QSL-2738 spec-review/evidence review"
type: "SpecReview"
analysis: "evidence"
scope: "agent-ix/filament-spec-language; QSL-2738; spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md; spec/test-cases/TC-3013-systems-part-port-and-allocation-records-are-admitted.md; source systems compiler and protocol binding"
review_set: "subset"
---

## Summary

Ticket: QSL-2738. Frozen head 4e05bb700c82895b0791ca819f7c283f82cfe6cf; reviewed against incorporated main 7bb1249428 (merge b0143e7912). Exactly the supplied worktree was inspected; no source edits, builds, Cargo commands, checkouts, pushes or extra worktrees. Quoin 0.36.7, Quire 0.52.0 (engine 0.62.2), spec-artifacts-process@v0.29.2. No applicable AssuranceProfile exists (no type: AssuranceProfile document in spec/).

## Verdict

PASS. No new scoped merge-blocking defect found. Aggregate gate completion remains the lead's responsibility.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The mandatory deterministic advisor completed successfully. Both FR-710-AC-11 and FR-710-AC-12 have authored Test and recommendations bdd-spec-by-example (E2E) and unit-testing (Unit), from property_shapes=example; mismatch=false and inconclusive=false for both. Independent review agrees with Test: the command fixture drives source compilation, model admission and package admission, while standalone source failures test exact codes, causes and spans. This is reviewer judgment about adequacy in addition to catalog advice, not an advisor-issued verdict. The existing feature-supported allocation check remains required and has supplied execution evidence. The single new FR-086-AC-2 test exercises each cause independently; the existing tc_234 test supplies the criterion's simultaneous-failure and no-spurious-interface-failure oracle, inspected but not rerun in this review. No mutation-testing, formal qualification or complete runtime behavior is claimed.

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
      "role": "context_only",
      "excerpt": "When compiling a `connection C p.outlet -> q.inlet;` or its `<->` form,\nthe compiler SHALL resolve `p` and `q` as Parts and `outlet` and `inlet` as\nPorts of this model unit, checking that each Port's declared owning Part is\nthe Part written before its dot. The qualifier is an ownership assertion;\nit does not create a nested declaration identity. The compiled connection's\nends name the Ports' own `ix://<domain>/<port name>` identities. `->` writes\n`source-to-target`, and `<->` writes `bidirectional`; each end has\nmultiplicity `[1, 1]`, unordered and unique."
    },
    {
      "id": "FR-710",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "context_only",
      "excerpt": "If a qualifier or Port name is absent, then the compiler SHALL refuse\n`missing_declaration`/`missing-name` at that name. If a qualifier names a\ndeclaration other than a Part, or an end names a declaration other than a\nPort, then the compiler SHALL refuse `invalid_model_binding`/`wrong-export`\nat that name, retaining required and actual kinds. If a Port belongs to a\ndifferent Part than the written qualifier, then the compiler SHALL refuse\n`invalid_model_binding`/`malformed-declaration` at the qualifier, retaining\nthe Port and the written and declared owning Part keys, with no document."
    },
    {
      "id": "FR-710",
      "path": "spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md",
      "role": "context_only",
      "excerpt": "The model-unit compiler SHALL preserve the Part's owner, declared type and\nmultiplicity, and the Port's owner, direction, interface type and\nmultiplicity, for the unchanged classification and connection admission\nrules of [FR-086](FR-086-bind-systems-model-references.md). A consuming unit\nselects the compiled model with `model M = \"<domain>\";` and names its Ports\nand Connections as `M::outlet` and `M::C`, under\n[FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md) and\n[FR-897](FR-897-carry-a-protocol-channel-over-a-systems-connection.md).\nThis adds no systems declarations to a con"
    },
    {
      "id": "TC-3013",
      "path": "spec/test-cases/TC-3013-systems-part-port-and-allocation-records-are-admitted.md",
      "role": "context_only",
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
      "role": "context_only",
      "excerpt": "Over FR-383-AC-62's model (`Sys` owning `pump_out` (out) and `tank_in` (in), connection `link` from `pump_out` to `tank_in`, source-to-target), `relationship Link = M::link;` with `channel Feed from Outlet to Inlet via Link carries Flow ...` checks and compiles. The channel node's `connection` argument references `link`'s relationship node, which its dependencies list, and the package is admitted."
    },
    {
      "id": "FR-897-AC-3",
      "path": "spec/functional/FR-897-carry-a-protocol-channel-over-a-systems-connection.md",
      "role": "context_only",
      "excerpt": "`channel Back from Inlet to Outlet via Link` (the reverse of a source-to-target connection) refuses `invalid_model_binding`/`malformed-declaration` at `Link`, retaining the expected and actual ports. A `from` role on the component `Sys` refuses at that role's name."
    },
    {
      "id": "FR-897-AC-5",
      "path": "spec/functional/FR-897-carry-a-protocol-channel-over-a-systems-connection.md",
      "role": "context_only",
      "excerpt": "`related by Link (a, b)` on a `send` refuses `ill_typed`/`operator-ineligible` at `Link` with the message naming `via`. `role R on M::link` still refuses as FSpec FR-465-AC-12 states."
    }
  ],
  "bindings": []
}
```

## Evidence and limits

The exact inherited runtime model ID is unavailable to both reviewer and lead; model=unavailable records this metadata limitation.

Supplied evidence: /tmp/qsl-2738-command-frozen.log has four passing command tests, with two accepted variants and 30 negative variants (22 endpoint, five role/channel, three independent admission causes), not 30 tests. /tmp/qsl-2738-source-regression.log has one passing existing systems-source regression. /tmp/qsl-2738-allocation-supported.log runs exactly systems_part_port_allocation_records with --features test-support: one pass. The earlier default-feature allocation command selected zero tests and is not proof. /tmp/qsl-2738-clippy-core.log succeeds for CST/semantics libraries. Replay-library and command-test Clippy exit 101 with four unchanged incorporated-main diagnostics (product.rs:1047,1054,1062 filter_map_bool_then; refinement_replay.rs:1665 let_and_return); /tmp/qsl-2738-unchanged-base.diff is empty for those files versus 7bb1249428. No baseline build was run, so this is unchanged-source evidence, not a passing baseline gate. Refinement let-and-return was reported fixed by later main; later-main integration was not reviewed. Existing linker and integration-test warnings remain recorded in the supplied logs. No aggregate make ci, full feature matrix or full gate was run or claimed.

Scope excludes inheritance and original/effective identity migration. V1-TYPE-025 is identity; no claim is made that this ticket implements or expands identity. No other optional spec lenses were selected: dependency (no edge change), scope-boundary/domain-object (unchanged responsibility/objects), risk-complexity (no new architecture), criterion-strength (not selected; no Jev run).
