#!/usr/bin/env bash
# Terra Sprites: CI's test job (.github/workflows/ci.yml), step for step, on
# this folder. `.\scripts\check.ps1` runs this from PowerShell. AGENTS.md says
# what the checks are and why it's `fmt --check`.
#
# When they pass on a committed tree, with no untracked files cargo would also
# read, the tree is noted so the pre-push hook (.githooks/pre-push) needn't run
# them again.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
tested=$(git rev-parse 'HEAD^{tree}')

step() { printf '\n== %s\n' "$1"; }

step "Formatting"
cargo fmt --all --check
step "Lints (including the simulation's determinism rules)"
cargo clippy --workspace --all-targets -- -D warnings
step "Tests"
cargo test --workspace
step "The simulation crate has no terminal dependencies (design section 2.1)"
if cargo tree -p terra-sim -e normal --prefix none | grep -Ei '^(ratatui|crossterm)'; then
    echo "terra-sim must not depend on terminal crates"
    exit 1
fi

# Cargo reads untracked files too (a forgotten module, another agent's test),
# so a pass with any around says nothing about the commit.
untracked=$(git ls-files --others --exclude-standard -- crates data themes Cargo.toml Cargo.lock rustfmt.toml)
if [ -n "$untracked" ]; then
    printf '\nAll checks passed, with files git doesn'"'"'t track that cargo read:\n%s\n' "$untracked"
    echo "Add the ones that belong to your change; pushing then checks the commit on its own."
elif ! git diff --quiet HEAD -- || [ "$(git rev-parse 'HEAD^{tree}')" != "$tested" ]; then
    printf '\nAll checks passed, on changes not yet committed (or HEAD moved during the run): pushing checks the commit again.\n'
else
    echo "$tested" >> "$(git rev-parse --git-common-dir)/terra-checks-passed"
    printf '\nAll checks passed on %s.\n' "$(git rev-parse --short HEAD)"
fi
