---
id: TC-2194
title: "A gate that did not build this checkout fails the examples check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2194: A gate that did not build this checkout fails the examples check

## Description

The examples check passes only when the driver gate reports building `fsl`
from this checkout.

Scope: FR-765-AC-5.

## Test Procedure

In a temporary folder, create a stand-in driver package whose
`examples_gate` test passes and, with `FSL_LANGUAGE_DIR` set, prints: nothing;
`fsl built from language checkout /elsewhere`; or the line naming the
`FSL_LANGUAGE_DIR` value. Create a stand-in examples folder with `examples/`
and `tools/fsl-examples/Cargo.toml`. Run `make examples-check` over each with
the built xtask binary and `EXAMPLES_CHECK_TESTS=`.

Tag the tests `#[trace("FR-765-AC-5", "TC-2194")]`.

## Expected Results

- Nothing printed: a nonzero exit whose error says the gate did not report
  building `fsl` from this checkout's path.
- Another path: a nonzero exit naming `/elsewhere`.
- This checkout: exit 0 and `passed with fsl built from <this checkout>`.
