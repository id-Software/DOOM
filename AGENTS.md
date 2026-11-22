# Standing engineering instructions

## Scope and source authority

- Work only within this repository and explicitly authorized resources. Do not
  access unrelated private repositories, proprietary assets, private cloud
  storage, credentials or confidential files. Public tests use synthetic or
  legally redistributable fixtures; commercial IWADs never enter Git, CI or logs.
- IWADs are read-only at `/mnt/d/DoomIWADs`, used locally only, with copies only in
  scratch. Committed results are counts, pass/fail facts and state digests only: never
  lump contents, names beyond the counts pages, coordinates or geometry.
- Preserve all 165 original upstream files, original history, attribution,
  `README.TXT` and `LICENSE.TXT`. Keep the repository public and GPL-2.0-only.
  Never import GPLv3-only implementation code.
- Original id Software Linux DOOM 1.10 at
  `a77dfb96cb91780ca334d0d4cfd86957558007e0` is behavioral authority. Read the
  relevant source investigation before changing an admitted behavior. Metadata
  recognition, executable admission and full compatibility are distinct claims.

## Architecture and determinism

- Prefer idiomatic safe Rust and explicit ownership. Workspace lints forbid
  unsafe code; exceptions require a separately reviewed policy and safety proof.
- Preserve 35 logical tics/second, fixed-point/integer semantics, original RNG
  ordering and deterministic actor/thinker/spatial order. No gameplay-critical
  unordered iteration, floating-point substitution or wall-clock dependency.
- Keep simulation separate from parsing, I/O, audio and future rendering.
  Presentation reads must not consume gameplay RNG or change outcomes.
- Validate offsets, sizes, counts, arithmetic, actor roles and generational
  references. Never sort handles to select blockers or rebuild source BLOCKMAP.
- Recoverable simulation errors preserve complete canonical state. Preflight
  bounded work before mutation; no full-simulation clone per tic. General user
  thinker budgets retain their separately documented nontransactional contract.
- Reuse actor storage, geometry, state entry, shared damage and lifecycle services.
  Unsupported actions must reject explicitly; callbacks must never silently no-op.
- Keep every historical profile, canonical schema and fixture expectation unchanged.
  New behavior requires an explicit configuration/schema choice. Preserve actor roles and
  existing admission/resource bounds; see [compatibility](docs/COMPATIBILITY.md).
  Source-compatible original state entry is immediate; raw spawn has no action.

## Validation

- Run build, format (including fuzz workspace), Clippy with warnings denied,
  workspace tests, release simulation/type tests and warnings-denied rustdoc.
  Verify Rust 1.85.0 and exact-head Linux, Windows, native macOS and MSRV CI.
  Preserve `macos-26-intel`, required check names and existing validation steps.
- Run every original-C generator and independent DSIM encoder with `--check`,
  all historical/new asserted examples in debug/release, all fuzz builds and a
  bounded relevant sanitizer campaign. Commands are in [testing](docs/TESTING.md).
- Evidence hierarchy (from M4a): primary evidence is the whole-engine harness (genuine
  engine against Rust on real maps, tic for tic, local-only) plus synthetic WADs through
  the same harness in CI; bespoke C scenarios cover quirks the maps do not reach; self-tests
  must prove the harness can fail.
- Expected C traces come from genuine source bodies and declared adapters;
  canonical expectations come from independent encoders, never Rust output.
  Preserve historical fixtures, helper bytes, provenance and qualified evidence.
- Distinguish source equivalence, explicit fixture projection, independent DSIM
  consistency and Rust-only safety tests. Report actual counts, sanitizer options,
  findings and limitations. Disabled leak detection is not LeakSanitizer coverage.

## Branches, review and documentation

- Inspect status, master and open PRs first. Implement on focused feature branches,
  never directly on master; preserve unrelated changes and shared history.
- Merges require every applicable fresh local/exact-head gate; after the owner merges,
  verify post-merge CI when required. Exception: documentation-only commits after a
  green exact head are covered by the CI fast path; batch docs fixes into one push.
- Record dates in local time (America/Los_Angeles).
- Open/update a focused PR with behavior, evidence, restrictions and unresolved
  issues. Do not claim unobserved passes.
- Maintain human-readable README, architecture, compatibility, roadmap, testing,
  source investigation and schema documentation. Use the
  [documentation review checklist](docs/DOC_REVIEW_CHECKLIST.md). Archive historical
  evidence with digests and working links instead of deleting or rewriting it.
- Documentation is an equal engineering gate alongside source and tests. A milestone
  is not ready when its guides are stale, contradictory, incomplete, misleading
  or difficult for humans to understand; resolving links alone is insufficient.
- **Quirk register:** [docs/QUIRKS.md](docs/QUIRKS.md) (the index) and the per-C-file pages
  under `docs/quirks/` are the home of the list of linuxdoom-1.10 quirks. A PR that finds or
  reproduces a quirk of the original updates its row there in the same PR, and adds the row
  if it is new.
- **Documentation authority:** Each information category has one authoritative
  home, mapped in the [documentation index](docs/README.md#documentation-authority).
  Milestone status belongs in ROADMAP, support/limits in COMPATIBILITY, schemas in
  STATE_FORMAT, commands in TESTING, observations in milestone evidence records,
  and exact review commits/CI/history in GitHub. Other pages summarize briefly and
  link; never maintain duplicate status tables, hashes, test totals or logs.
  Before editing, identify the authoritative home and check existing information
  there rather than creating another copy because a milestone changed.
- README describes current state only, in plain language for newcomers, in under 150
  lines. A milestone edits the "Where it stands" list and adds or edits one row of the
  [policy table](docs/COMPATIBILITY.md#policies) (it lives in COMPATIBILITY; the README
  links to it once), and never appends prose history (history lives in
  [COMPATIBILITY.md](docs/COMPATIBILITY.md) and [ROADMAP.md](docs/ROADMAP.md)). Caveat
  counts are links to the evidence, never repeated figures, and no claim may get
  stronger by being shortened. "Where it's (maybe) going" changes only with the owner's
  approval.
- Keep the prominent machine-maintained/human-directed disclosure accurate. Do not
  imply universal manual review, absent human oversight or guaranteed correctness.
- Visuals. When a milestone adds anything that can be shown visually (rendering, the
  status bar, automap, menus, a playable loop, a new effect or monster on screen), its
  docs step adds a short animation to the README or the milestone's evidence record:
  under about 15 seconds and 2 MB, in a format GitHub renders, generated by a committed
  script so anyone can reproduce it. Use the synthetic test maps and our own generated
  graphics; never commit IWAD-derived images unless the owner approves that specific
  image. Existing visuals stay; new ones are added alongside, with a one-line plain caption.
- Stop at the assigned milestone's owner-review gate; no unrelated systems.

## Session conventions

- Commit identity comes from the repo-local git config set by the owner (a GitHub
  noreply address, no personal name or email). Never add `Co-Authored-By` trailers,
  "Generated with" lines or any AI attribution to commits, PR titles or PR bodies.
- Commit subject lines are at most 72 characters; the details go in the body after a
  blank line. This applies from 2026-11-18 on: existing history is never rewritten.
- Never commit to or push `master`. Never force-push, rebase or squash shared history,
  delete branches or enable auto-merge. Never merge a PR; merging is the owner's call.
- Open one focused PR per milestone, draft until every gate passes; mark it ready only
  after the gates, then leave it unmerged.
- Run every performance measurement alone on an idle machine (no concurrent builds,
  tests or fuzzing) and say so in the record.
- Run every timing, benchmark and Callgrind measurement from binaries built on and run from
  the Linux filesystem (ext4, for example `~/scratch`), never from `/mnt/*`; record the path
  in the evidence.
- Keep scratch builds, baseline checkouts and fuzz corpora outside the working tree;
  delete them at the end of a milestone after their results are recorded.
- Subagent models follow [model routing](#model-routing-for-subagents).
- Report only observed results; distinguish target compiled, seed replay, coverage-guided
  campaign and sanitizer campaign, and disclose disabled leak detection.
- Push the branch to origin after every completed plan step (no PR until gates pass).
- Check free disk space before large builds, baseline worktrees or fuzzing; keep at least
  20 GB free; clean scratch first or stop and report. Under WSL, `df ~` reports the
  virtual disk's ceiling, not real space: the real limit is the Windows drive that
  hosts it. Check `df -h /mnt/c` and keep at least 20 GB free there at all times.
- Long sessions: use fresh subagents per step, never two writers on one tree, and never
  leave background jobs running across the end of a reply. Run at most 2 parallel C
  reference jobs while the owner is using the machine; no limit when the owner says it is
  idle (halve and re-run on an OOM kill).
- Waiting on a process: wait on its PID (`wait PID`, or
  `while kill -0 PID 2>/dev/null; do sleep N; done`), never `pgrep -f PATTERN`, which
  matches its own command line and never exits. Every wait loop must have a timeout. An
  agent that finishes must leave no shell running.
- Deleting files: never `cd DIR; rm ...` or `rm -rf *`. Delete by absolute path under
  ~/scratch only (`rm -rf -- "$HOME/scratch/<milestone>/<dir>"`), with `set -u` so an
  empty variable fails, and never delete inside the repo, /mnt/*, or a path built from
  an unchecked variable. Download into a fresh empty directory instead of clearing one.
- Unattended rule: continue past an investigation phase only if every new decision has a
  clear recommendation consistent with prior owner decisions and admits no silent
  divergence; otherwise stop, commit, push and report.

## Model routing for subagents

Agent definitions live in `.claude/agents/` (Claude Code) and `.codex/agents/` (Codex).

| Role | Agent | Claude Code (model, effort) | Codex (model, effort) |
| --- | --- | --- | --- |
| Orchestrator (main session) | main session | whichever model the owner launched | whichever model the owner launched |
| Independent review | reviewer | opus, xhigh | gpt-6.1-sol, xhigh |
| C evidence, simulation code, DSIM encoder; tests needing new C citations | c-evidence | opus, high | gpt-6.1-sol, high |
| Tests from facts already cited; mutation-survivor tests | tester | sonnet, medium | gpt-6.1-sol, medium |
| Docs, status lines, census/count tools, scripts, CI plumbing | docs-tools | sonnet, medium | gpt-6.1-sol, medium |
| Watching long runs, log tailing, file inventory | watcher | haiku, low | gpt-6-luna, high |

- Effort names do not mean the same across vendors, so the two columns are set per vendor
  guidance, not copied across.
- Model names are as of 2026-11-18 (gpt-6.2-sol and gpt-6.2-astra launched in Codex on
  2026-11-17) and are checked with `/model` before a Codex handoff. The owner confirmed on
  2026-11-18 that Sol offers Extra High.

Rules:

- Name the agent on every spawn; no unnamed subagents for heavy work.
- A subagent never upgrades its own model.
- After two failed reviews or gates a piece may move up one tier, with the reason recorded.
  The one-tier escalation is fable on Claude Code and gpt-6-astra on Codex.
- A reviewer is always a fresh agent that did not write the code under review.
- The instruction bodies in `.claude/agents` and `.codex/agents` stay word-for-word
  identical, and a change to one is made to both in the same commit.
- Watchers poll at most every 30 minutes and do not re-read docs or code.
- When stopping an agent, keep long-running jobs its shell owns alive.
