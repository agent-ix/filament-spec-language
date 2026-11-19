---
id: TC-2191
title: "No workflow runs on a push or a pull request"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2191: No workflow runs on a push or a pull request

## Description

Hosted workflows stay manual.

Scope: FR-765-AC-2.

## Test Procedure

Read the top-level `on:` key of every `.github/workflows/*.yml` file and
collect its trigger names. Check the reader on a scalar, a flow list, a
quoted key with a block mapping, a block list, and a block with a comment.

Tag the tests `#[trace("FR-765-AC-2", "TC-2191")]`.

## Expected Results

- Every workflow has at least one trigger, and none is `push` or
  `pull_request` (`pull_request_target` is a different trigger).
- The reader returns `push` or `pull_request` in every spelling checked.
