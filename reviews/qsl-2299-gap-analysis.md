---
id: SR-2026111612172400000002
title: "gap analysis of PR 2751 compile output file against FR-810"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 2751 head f9612564d3fc94f7c088c8aadd66cf70d62ab152; FR-810 Behavior and FR-810-AC-1..5; tests/it/compile_package_output.rs, fsl-command/tests/it/request_overrides.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: references
---

## Summary

Ticket: QSL-2299. This is a planless audit using `quire matrix --scope .`. FR-810-AC-1 to AC-3 are tagged by non-ignored tests. AC-4 has no tag, as expected: the driver test belongs to a later PR in the driver repository. AC-5 is tagged only by an ignored test, also as expected: it is blocked on QSL-2193's `handoff` member and split into a follow-up ticket. The AC-2 test misses one stated sub-case and part of another.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-810-AC-2 states "a request whose program does not check carries `output` and writes no file", and no test runs that case. Its missing-directory sub-case also asserts only exit 20. It does not check the I/O code (`io-error`) or that the message names the path, both of which the AC and FR-810 "Behavior" require. Fix: in `a_compile_output_member_refuses_and_writes_nothing`, add a request over a fixture that fails checking, carrying `output`, and assert the refusal and that the file is absent. In the missing-directory case, assert `value["code"] == "io-error"` and that the message contains the path. | tests/it/compile_package_output.rs:54-97 |

## Verdict

PASS WITH FINDINGS. Every new production item traces to FR-810: the `output` member, `OutputMember`, `RequestOverrides::output`, `OutputGivenTwice`, `OutputExists` and `write_new_file`. The tests assert real values: byte equality of the file with `package_bytes()`, `artifacts: []`, preserved bytes of an existing file, code, stage and exit for the refusals, and absence of both files for the doubly supplied case. The AC-1 test runs a single `compile_outcome` in place of the two calls the stub made. Under never-overwrite, a second run refuses, and `compile` is a projection of `compile_outcome`, so the test still checks what AC-1 states.

## Coverage

FR-810-AC-1, AC-2 and AC-3, and the FR-810 Behavior bullets, were examined. AC-4 (driver) and AC-5 (handoff, blocked on QSL-2193) were read for context only. Reverse gap: none.

Plan completion: not assessed

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The missing-directory case now asserts `io-error` and the path. A new case replaces the program with `not a program`, keeps `output`, and asserts that no file is written. It also asserts that the refusal (exit, code, stage, message) matches the same request without `output`. This program fails at the parse stage, not the check stage. That is enough for AC-2, because the write runs only after `complete` returns. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      assert_eq!(value["code"], "io-error", "{value}");
      assert!(
          value.to_string().contains("missing/spine.package.json"),
          "{value}"
      );
      ...
      std::fs::write(directory.path().join("program.native"), b"not a program").unwrap();
      let (exit, value) = refusal(&path);
      ...
      assert!(!directory.path().join("spine.package.json").exists());
```
