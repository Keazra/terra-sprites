# Agent instructions

## Project

Terra Sprites is an ASCII artificial-life game in Rust. The spec is the current design doc in `docs/design/` (the highest `-vN`); see `docs/agents/domain.md` for how to read it.

Before pushing, run the checks CI runs (`.github/workflows/ci.yml` is the source of truth):

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings` (this includes the simulation's determinism lints)
- `cargo test --workspace`

`terra-sim` must not depend on terminal crates (`ratatui`, `crossterm`); CI checks this too.

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
