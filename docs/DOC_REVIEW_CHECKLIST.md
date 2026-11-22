# Documentation review checklist

Use this checklist for a documentation-only PR or the documentation portion of
a gameplay milestone. It supplements [CONTRIBUTING.md](../CONTRIBUTING.md) and
[AGENTS.md](../AGENTS.md); it does not authorize a merge or change engineering
acceptance criteria.

Return to the [documentation index](README.md) for the file map.

## 1. Scope and concurrency

- [ ] Check current default-branch HEAD and open PRs before editing.
- [ ] Work on a separate branch; do not make housekeeping edits directly on
      master or push into another agent's active milestone branch.
- [ ] Identify documents likely to be edited by a concurrent gameplay PR.
      Prefer new, stable navigation or review material over overlapping edits.
- [ ] Confirm the change is documentation-only. If runtime, tests, generators,
      golden fixtures, CI or the canonical schema must change, use a separately
      reviewed implementation scope instead.
- [ ] Do not rewrite historical evidence as though previous observations were
      rerun on the current commit.

## 2. Find the right source of truth

- [ ] Identify the authoritative home in the [documentation authority map](README.md#documentation-authority).
- [ ] Update that home and link from other pages; do not create duplicate status
      tables, test-count lists, schema definitions or implementation logs.
- [ ] Keep current milestone summaries in ROADMAP, detailed test observations in
      milestone evidence records, and exact review commits/CI/history in GitHub.
- [ ] Treat frozen archives and dated source observations as historical evidence,
      not current status. Do not rewrite them to match a later milestone.
- [ ] If the change finds or reproduces a quirk of the original, update (or add) its
      row in the [quirk register](QUIRKS.md) (the index and the pages under `docs/quirks/`) in
      the same PR. The register points to COMPATIBILITY for support status; it does not replace it.

## 3. Link and text quality

- [ ] Verify every newly added relative path against the repository tree.
- [ ] Verify heading anchors when changing a target heading. GitHub Markdown
      heading slugs are sensitive to punctuation and duplicate headings.
- [ ] Keep milestone names, file names, branches and PR links consistent.
- [ ] Avoid ambiguous references such as "latest" or "current" without an
      identified branch or observation date.
- [ ] Make limitations visible near capability claims; distinguish metadata
      recognition, executable admission and compatibility evidence.
- [ ] Prefer durable navigation and short summaries over repeating a long
      engineering report in multiple places.
- [ ] Check Markdown tables and fenced commands for readable formatting.

## 4. Evidence integrity

- [ ] **Original C:** identify the original Linux DOOM version, source bodies,
      adapter assumptions and tested input slice. Do not conflate a synthetic
      wrapper with original in-game execution.
- [ ] **Canonical DSIM:** explain whether the expected bytes come from an
      independent encoder. State that DSIM is not a vanilla demo or savegame.
- [ ] **Rust-only policy:** label safe departures, resource bounds, and
      atomicity/determinism tests separately from source equivalence.
- [ ] Record only checks whose results were observed. Give counts and sanitizer
      settings accurately; disabled leak detection is not LeakSanitizer coverage.
- [ ] Verify exact final-head and post-merge CI **when claiming** they passed.
      Linux reference generation and other OS CI steps may differ.
- [ ] Link exact reviewed commits and completed CI in the PR/Actions evidence.
      Keep methodology in VALIDATION and measured test results in the milestone
      record. Do not create another correction commit solely to copy live CI data.
- [ ] For a stacked PR, verify the base/head and changed-file scope. If existing
      workflow filters exclude that base, report CI as unavailable for the stack;
      retarget and run fresh exact-head validation after the prerequisite merges.
- [ ] Preserve historical DSIM versions, original-C fixtures and their
      provenance. Do not silently regenerate expected data from Rust output.

## 5. Repository safeguards

- [ ] Leave the original upstream C source, release notes, attribution, license
      and Git history untouched.
- [ ] Keep the repo public and GPL-2.0-only; do not import GPLv3-only
      implementation code from another source port.
- [ ] Use only synthetic or redistributable test data. Never access or publish
      private IWADs, private repositories, credentials or private test logs.
- [ ] Keep rendering and audio presentation separate from authoritative
      deterministic gameplay; avoid new unsupported feature promises.
- [ ] Confirm no source code, binary fixture, generated artifact or private
      content slipped into a docs-only diff.

## 6. Pull request handoff

- [ ] Summarize the exact documentation changes and why they help.
- [ ] List paths added/modified and state explicitly whether runtime/CI/fixtures
      changed.
- [ ] Report link/format checks actually performed, plus any checks not run.
- [ ] Note concurrent milestone PRs and potential conflicts.
- [ ] Leave the PR open and unmerged, with auto-merge disabled, for owner review.
- [ ] Do not delete branches, squash, force-push or merge without authorization.
