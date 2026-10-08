# The game says which build it is

Fauxpaw asked on 2026-10-08 for ways to raise the quality of PRs. One was knowing, when trying a PR, that the game running is that PR's build: `--version` names the commit it was built from, and `scripts/try-pr.ps1` names it too when it switches.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`--version`** prints the game's version and the commit it was built from, as `terra-sprites 0.1.0 (cc96347)`, and stops. The commit is "unknown" for a build from outside git. | Owner, 2026-10-08 | §6.7 |
