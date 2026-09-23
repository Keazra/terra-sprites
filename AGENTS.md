# Agent instructions

## Project

Terra Sprites is an ASCII artificial-life game in Rust. The spec is the current design doc in `docs/design/` (the highest `-vN`); see `docs/agents/domain.md` for how to read it.

Before pushing, run the checks CI runs (`.github/workflows/ci.yml` is the source of truth):

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings` (this includes the simulation's determinism lints)
- `cargo test --workspace`

`terra-sim` must not depend on terminal crates (`ratatui`, `crossterm`); CI checks this too.

## Agent skills

### Issue tracker

Issues live in GitHub Issues for Keazra/terra-sprites and are handled with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The default triage labels, except `ready-for-agent` is `afk` and `ready-for-human` is `hitl`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
