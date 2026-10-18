# Agent instructions

## Project

Terra Sprites is an ASCII artificial-life game in Rust. The spec is the design docs in `docs/design/`: M1's (`m1-a-sprite-lives.md`) for the game as built, and M2's (`m2-generations.md`) for milestone 2, which builds on it; see `docs/agents/domain.md` for how to read them.

Before pushing, run the checks CI runs: `scripts/check.sh`, or `.\scripts\check.ps1` in PowerShell. In a cloud session, run `rustup update stable` first, since the container's Rust can be older than CI's (`docs/agents/how-we-work.md`, "Cloud sessions"). The checks run, as CI's test job does (`.github/workflows/ci.yml` is the source of truth):

- `cargo fmt --all --check` (`listed_enum!` bodies aren't formatted by rustfmt; keep them tidy by hand)
- `cargo clippy --workspace --all-targets -- -D warnings` (this includes the simulation's determinism lints)
- `cargo test --workspace`: every test, the unit tests inside `src/` too, not only the files you changed

A pre-push hook (`.githooks/pre-push`) runs them again on any commit they haven't passed, which makes the push take minutes. Commit first and then run them, and the push is instant. It's on once a clone has `git config core.hooksPath .githooks`; Claude Code sessions switch it on themselves. Don't skip it with `--no-verify`: three of six PRs in a row (#163, #167, #168) were opened with CI red for a check that wasn't run.

To format a file of your own, `rustfmt <file>` (it reads `rustfmt.toml` for the 2024 edition) or `cargo fmt --all` when no other agent has untracked Rust files in the folder.

`terra-sim` must not depend on terminal crates (`ratatui`, `crossterm`); CI checks this too.

## Commands

- Run the game: `cargo run --release -- --seed 7` (flags: `--seed <n>`, `--preset <file.ron>`, `--ascii`). The binary is `terra-sprites`, in the `terra-tui` crate.
- One test file: `cargo test -p terra-sim --test movement`
- One test by name (substring): `cargo test -p terra-sim --test movement -- head_on`
- Unit tests inside a module: `cargo test -p terra-sim --lib biochem::`
- The soak (A7), one seed again: `cargo run --profile baseline -p terra-sim --example soak -- --seed 1717` (a million ticks; `--ticks N` for fewer). The `baseline` profile keeps the self-check on.
- Speed check (A5, 100 sprites, ticks a second): `cargo run --release -p terra-sim --example speed`. Benchmarks of a tick, a flood and a brain step: `cargo bench -p terra-sim`.
- Resting by age (#130): `cargo run --release -p terra-sim --example rest` (`--seeds A-B`, `--ticks N`, `--sprites`).

The dev profile is `opt-level = 1` because the self-check after every tick makes unoptimised long tests take minutes. Debug assertions stay on, so tests still run the self-check.

## Architecture

Two crates in `crates/`. The layout and which crate may depend on what are in the design doc, §2.1.

**`terra-sim`: the simulation.** No terminal crates and no filesystem access. CI checks the first; `crates/terra-sim/clippy.toml` enforces the second, along with determinism:

- `HashMap`/`HashSet` are banned (iteration order). Use `BTreeMap`, `BTreeSet` or `Vec`.
- std's transcendental float functions and `mul_add` are banned. Use `libm` (design §2.6).
- `std::fs` is banned. The data pack arrives as already-read text.

The game's content is data in `data/*.ron` (terrain, chemicals, loci, brain inputs, objects, physiology, the starter genome, the default preset). `DataPack::builtin()` and `WorldConfig::builtin()` embed these files with `include_str!`. Editing `data/` changes the compiled game, and a malformed file fails at load with a `DataError`. The owner's principle is that the engine enforces physics only. Behaviour, safety policies and names, such as a cause of death, belong in each object type's data rules, not in lists in the code. The one exception is content safety, which the engine enforces so no data pack or evolved genome can undo it (see `docs/agents/how-we-work.md`).

`World` (`world.rs`) is the entry point. `World::new(config, data, seed)` generates a world; `World::from_scenario` builds a hand-drawn one for tests. `World::step()` runs one tick in the canonical order of design §2.4 (commands, environment, biochemistry, learning, sense and decide, resolve actions, finish) and returns `Event`s. In debug builds it then runs `check_invariants` (design §7.1) and panics naming the tick. Everything that determines how the world evolves lives in `WorldState`, which is serialised for `state_hash`. That includes its single `ChaCha8Rng`, the world's only source of randomness (design §2.3). New simulation state belongs in `WorldState`, or determinism and hashing break silently. The front end reads the world through views (`SpriteView`, `ObjectView`, `ActionView`) rather than internal types. `lib.rs` is the whole public API.

**`terra-tui`: the terminal front end** (ratatui). `main.rs` owns the terminal and the frame loop. All testable logic is in the library: `App` (UI state and actions), `clock` (speed and tick pacing within a per-frame simulation budget), `input` (key and mouse to action), `ui` (layout and rendering) and `inspector` (the sprite inspector's tabs). Themes are `themes/*.ron`, also embedded. UI tests render with ratatui's `TestBackend`.

## Tests

- Integration tests in `crates/*/tests/` go through the public API. Build test worlds with `Scenario` rather than reaching into internals. Use each crate's `tests/common` (and terra-sim's `src/test_util` in unit tests) instead of copying helpers.
- Golden files (`crates/terra-sim/tests/golden/`) must round-trip byte for byte. `.gitattributes` forces LF everywhere, which they rely on.
- `proptest-regressions/` files are committed; keep them.

## Public and private documents

This repo is public. The owner's working documents are in a private one, [Keazra/terra-sprites-docs](https://github.com/Keazra/terra-sprites-docs), which their PC has at `docs/terra-sprites-docs/`. Before writing a document, decide which it belongs in:

- **Public, here:** what anyone with a copy of the game needs to play, change or build on it. That's the designs with their change files and outside reviews (`docs/design/`), the data-format reference (`docs/reference/`, which the tests read), the acceptance records, the release notes (`docs/releases/`), ADRs (`docs/adr/`), how agents work here (`docs/agents/`), and `AGENTS.md`, `CONTEXT.md` and the README.
- **Private, in `docs/terra-sprites-docs/`:** the work behind those. That's each topic's question rounds, research, mockups, run logs and patches, the baseline reports and briefings (design §7.6), play-test notes and the advertising material. The private repo's own `AGENTS.md` says where in it things go.
- **When unsure, it's private.** A private document can be published later, but one pushed here stays public in git's history. The owner's real name and their PC's paths never go here, nor in this repo's PRs, issues and comments, whose old wording GitHub keeps in each edit's history. A "Try it on your PC" section runs from "the terra-sprites folder", with no `cd` to the PC's own path.

How it fits together:

- **Where it is.** On the owner's PC, `docs/terra-sprites-docs/` is a submodule. A cloud session given the private repo has it cloned beside this one, as `../terra-sprites-docs`. Work in that clone, and leave this repo's `docs/terra-sprites-docs/` empty, as it is in every public copy.
- **Which sessions need it.** Work on code alone (the game, its tests, CI) needs only this repo. Work that reads or writes a private document needs both, so the owner gives that session both repos; in a Claude Code project, they're in the project's settings and every thread gets them. A session that needs the private repo and wasn't given it says so to the owner, rather than writing the document here.
- **Each repo is its own source of truth:** its `main` on GitHub. The owner's PC and each session hold copies, which pull before they write and push after, and GitHub turns away a push from a copy that's behind. This repo holds none of the private repo's content, only the pointer below, which isn't a version of it.
- **Nothing public may need it,** since nobody else can fetch it. The build, the tests, CI and the README's links work without it, and a public document doesn't link into it. Only the owner's own scripts use it: `scripts/baseline.ps1` keeps its reports there. Its entry in `.gitmodules` makes clones and the README's `cargo install --git` skip it rather than fail, and keeps `git status` quiet as the private repo moves on; the comment there says how.
- **Leave its pointer alone.** This repo pins it at one commit only to say where it goes, so never commit a change to `docs/terra-sprites-docs`. `git commit -a` does that silently, since `ignore = all` hides it from the diff too.
- **Never initialise it** (`git submodule init`, or `update --init`). Left uninitialised, git's recursive commands pass it by. Once it's initialised, a recursive switch to a branch from before the submodule removes the folder, and switching back leaves it on the pinned commit instead of its `main`.

## Workflow

How the owner likes to work, the project's conventions and the slice loop are in `docs/agents/how-we-work.md`. Read it before starting.

**Starting work:**

1. Start from an up-to-date `main` (`git pull --ff-only`) and a clean tree, and check the open PRs: one may already touch what you're about to change, and then the work may belong on top of it (`docs/agents/stacked-prs.md`, "When to stack").
2. For the next slice, take the earliest open "Slice N" issue in the current milestone whose `## Blocked by` issues are closed, and read its comments for items deferred from earlier slices.
3. Read the newest briefing in the private repo's `reports/` (design §7.6), if the session has the private repo and there is one.
4. Open issues with no triage label (`docs/agents/triage-labels.md`) go through `/triage`: mention them to the owner.

**Before you finish:**

1. Everything is committed and pushed to the feature branch, with `scripts/check` passing, and the PR is open ("Pull requests", below).
2. Follow-ups are on GitHub (below), not only in the conversation.
3. A new convention or owner preference is in `docs/agents/how-we-work.md`, in the PR or a small one of its own.

**Skills** live in `.claude/skills/`. When one the workflow names isn't loaded (cloud sessions don't always load the repo's skills), read its `SKILL.md` there and follow it as written.

**Pick the repo's copy of a Matt Pocock skill** (`tdd`) over the plugin's (`mattpocock-skills:tdd`). The owner has the whole set as the "Skills For Real Engineers" plugin, and the repo's copies are adjusted for this project (`.claude/skills/THIRD-PARTY.md`). For a review, that's `two-axis-review`. Plugin skills with no copy here, such as `to-spec`, are fine, except `pr` and `handoff`: PR bodies follow the template, and starting and finishing follow the steps above. This repo needs no `/setup-matt-pocock-skills`, since `docs/agents/` already holds what it would write.

**A message that starts with a skill's name** (`/triage`, `/tdd`) asks you to run that skill. Claude Code Projects and other cloud chats have no slash-command menu, so the name arrives as plain text: load the skill with the Skill tool. A plugin skill marked for the user to start (`to-spec`, `to-tickets`, `grill-me`) is hidden from the Skill tool: find it with `find ~/.claude -path '*/skills/<name>/SKILL.md'` (the newest, if several match) and follow it as written. `/grill-with-docs` means this repo's `grilling` with `domain-modeling` (`docs/agents/how-we-work.md`), not the plugin's skill of that name.

## Git and GitHub

- Work on a feature branch and open a PR. `main` moves only through merged PRs, and the owner merges each one themselves.
- Stage files by name, never with `git add -A` or `git add .`. The owner's review tooling writes files into the repo folder (`.agents/`, `ORIGINAL_REQUEST.md`).
- With `gh`, post issue and PR bodies and comments with `--body-file` and a temp file: inline `--body` text breaks on backticks, quotes and paths.
- After opening an issue, post a plain-language comment on it for a human reader: what it proposes, why it matters, and any caveats.
- Follow-up work goes on GitHub, not just in the conversation. An item for a later slice is a comment on that slice's issue; anything else is a new issue, triaged `afk` or `hitl`.
- "Closes #N" anywhere in a PR description closes that issue on merge, so write it only in the PR meant to close it. GitHub matches the keyword mid-sentence and in any form ("close", "closes", "fix", "fixes", "resolve", "resolves"), so a PR that only works towards an issue says "Part of #N" and never puts one of those words in front of the number. "Only 7c's PR closes #8" closed #8 when 7a merged.

### Pull requests

1. **Pick its base, every time:** `main`, or the open PR it needs. List the open PRs first, and stack on one only when this PR needs it (`docs/agents/stacked-prs.md`, "When to stack"). The description's first line, above "What changes", says which: "Based on `main`: needs nothing open." or "Stacked on #245: merge that first."
2. **Open it as a draft, from the template** (`.github/pull_request_template.md`), its "Try it on your PC" section included.
3. **Comment on it in plain language** for a human reader: what it changes, why it matters, and any caveats for the reviewer.
4. **Mark it ready yourself once CI is green,** so ready means nothing is left for the session. First bring in `main` if anything merged since the branch's base: merge `origin/main` into it and run `scripts/check` again. `main` doesn't require a branch to be up to date, so two green PRs can break it together: #192 merged after #191 had changed a test helper both used, and `main`'s terra-tui tests stopped compiling until #197 reverted it.
5. **Give the merge order** whenever PRs must merge in one: a stack, a pair with a private-docs PR, or two PRs that change the same lines. The message handing them to the owner lists the merges as numbered steps, first to last, with what to check between them, and each PR's plain-language comment repeats them.

**Stacking a PR** on another, or working on one in a stack: `docs/agents/stacked-prs.md`.

### Releases

Each finished milestone is a numbered release (M1 is v0.1.0, M2 v0.2.0), and from v0.2.0 on the README installs the newest one. `main` carries the work in progress, and PRs keep going to it one at a time: a milestone is never held back on a branch or a stack of its own. The milestone's acceptance slice prepares its release, and the owner makes it with one `gh` command, since GitHub refuses a session's tags and releases: `docs/agents/releases.md`.

## Agent skills

### Issue tracker

Issues live in GitHub Issues for Keazra/terra-sprites and are handled with the `gh` CLI, or in cloud sessions, where `gh`'s own commands fail, with the GitHub MCP tools and `gh api`. See `docs/agents/issue-tracker.md`.

### Triage labels

The default triage labels, except `ready-for-agent` is `afk` and `ready-for-human` is `hitl`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

`CONTEXT.md` is the glossary. Names in code, tests and comments use its terms, not the ones on its _Avoid_ lists (tile, not cell; region, not island). Code comments cite the design doc by section (`design §3.4`), and so should new code.
