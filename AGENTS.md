# Agent instructions

## Project

Terra Sprites is an ASCII artificial-life game in Rust. The spec is the current design doc in `docs/design/` (the highest `-vN`); see `docs/agents/domain.md` for how to read it.

Before pushing, run the checks CI runs (`.github/workflows/ci.yml` is the source of truth):

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings` (this includes the simulation's determinism lints)
- `cargo test --workspace`

`terra-sim` must not depend on terminal crates (`ratatui`, `crossterm`); CI checks this too.

## Commands

- Run the game: `cargo run --release -- --seed 7` (flags: `--seed <n>`, `--preset <file.ron>`, `--ascii`). The binary is `terra-sprites`, in the `terra-tui` crate.
- One test file: `cargo test -p terra-sim --test movement`
- One test by name (substring): `cargo test -p terra-sim --test movement -- head_on`
- Unit tests inside a module: `cargo test -p terra-sim --lib biochem::`

The dev profile is `opt-level = 1` because the self-check after every tick makes unoptimised long tests take minutes. Debug assertions stay on, so tests still run the self-check.

## Architecture

Two crates in `crates/`. The layout and which crate may depend on what are in the design doc, §2.1.

**`terra-sim`: the simulation.** No terminal crates and no filesystem access. CI checks the first; `crates/terra-sim/clippy.toml` enforces the second, along with determinism:

- `HashMap`/`HashSet` are banned (iteration order). Use `BTreeMap`, `BTreeSet` or `Vec`.
- std's transcendental float functions and `mul_add` are banned. Use `libm` (design §2.6).
- `std::fs` is banned. The data pack arrives as already-read text.

The game's content is data in `data/*.ron` (terrain, chemicals, loci, objects, physiology, the starter genome, the default preset). `DataPack::builtin()` and `WorldConfig::builtin()` embed these files with `include_str!`. Editing `data/` changes the compiled game, and a malformed file fails at load with a `DataError`. The owner's principle is that the engine enforces physics only. Behaviour, safety policies and names, such as a cause of death, belong in each object type's data rules, not in lists in the code.

`World` (`world.rs`) is the entry point. `World::new(config, data, seed)` generates a world; `World::from_scenario` builds a hand-drawn one for tests. `World::step()` runs one tick in the canonical order of design §2.4 (commands, environment, biochemistry, learning, sense and decide, resolve actions, finish) and returns `Event`s. In debug builds it then runs `check_invariants` (design §7.1) and panics naming the tick. Everything that determines how the world evolves lives in `WorldState`, which is serialised for `state_hash`. That includes its single `ChaCha8Rng`, the world's only source of randomness (design §2.3). New simulation state belongs in `WorldState`, or determinism and hashing break silently. The front end reads the world through views (`SpriteView`, `ObjectView`, `ActionView`) rather than internal types. `lib.rs` is the whole public API.

**`terra-tui`: the terminal front end** (ratatui). `main.rs` owns the terminal and the frame loop. All testable logic is in the library: `App` (UI state and actions), `clock` (speed and tick pacing within a per-frame simulation budget), `input` (key and mouse to action), `ui` (layout and rendering) and `inspector` (the sprite inspector's tabs). Themes are `themes/*.ron`, also embedded. UI tests render with ratatui's `TestBackend`.

## Tests

- Integration tests in `crates/*/tests/` go through the public API. Build test worlds with `Scenario` rather than reaching into internals.
- Golden files (`crates/terra-sim/tests/golden/`) must round-trip byte for byte. `.gitattributes` forces LF everywhere, which they rely on.
- `proptest-regressions/` files are committed; keep them.

## Workflow

Sessions start and end with the `handover` skill (`/handover start`, `/handover end`), which runs each slice. How the owner likes to work, and the project's conventions, are in `docs/agents/how-we-work.md`. Read it before starting.

## Git and GitHub

- Work on a feature branch and open a PR. Never commit to `main` directly: it moves only through merged PRs.
- Stage files by name, never with `git add -A` or `git add .`. The owner's review tooling writes files into the repo folder (`.agents/`, `ORIGINAL_REQUEST.md`).
- Post issue and PR bodies and comments with `--body-file` and a temp file, never an inline `--body`: inline text breaks on backticks, quotes and paths.
- After opening an issue or a PR, post a plain-language comment on it for a human reader: what it changes or proposes, why it matters, and any caveats for the reviewer.
- Follow-up work goes on GitHub, not just in the conversation. An item for a later slice is a comment on that slice's issue; anything else is a new issue, triaged `afk` or `hitl`.
- "Closes #N" anywhere in a PR description closes that issue on merge, so write it only in the PR meant to close it.

## Agent skills

### Issue tracker

Issues live in GitHub Issues for Keazra/terra-sprites and are handled with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The default triage labels, except `ready-for-agent` is `afk` and `ready-for-human` is `hitl`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

`CONTEXT.md` is the glossary. Names in code, tests and comments use its terms, not the ones on its _Avoid_ lists (tile, not cell; region, not island). Code comments cite the design doc by section (`design §3.4`), and so should new code.
