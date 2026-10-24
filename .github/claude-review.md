# For the Claude reviewer

What `.github/workflows/claude-review.yml` should know about Terra Sprites, an ASCII artificial-life game in Rust, on top of `AGENTS.md`.

- **The review list is in `scripts/review-pr.md`, section 3.** It's this project's, written for Gemini's reviews: go through it. Skip what it says about `.review/`, the sandbox and Gemini's answer format; your inputs are in `.claude-review/`, and your answer is the structured output.
- **`docs/agents/review-checklist.md` is what outside reviews kept finding that the authors' own review missed.** Check every row against the diff.
- **CI already runs** `cargo fmt`, clippy (with the simulation's determinism lints), every test, the no-terminal-crates check and the cross-platform determinism check. Don't redo them.
- **The spec** is the issue, the design the PR touches, by section (`docs/design/m1-a-sprite-lives.md`, `docs/design/m2-generations.md`), and the PR's file in `docs/design/changes/`, if it has one. Code, tests and docs use the terms in `CONTEXT.md`, not the ones on its _Avoid_ lists.
