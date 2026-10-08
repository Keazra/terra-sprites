#!/usr/bin/env bash
# Terra Sprites: the checks CI's test job runs (.github/workflows/ci.yml), in
# the same order. Run it before pushing; scripts/check.ps1 is the same for
# PowerShell. When everything passes on a clean working tree, it notes the
# commit, so the pre-push hook (.githooks/pre-push) doesn't run them again.
#
# It runs `cargo fmt --all --check`, not `cargo fmt --all`: the latter would
# reformat other agents' untracked files (docs/agents/how-we-work.md).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

step() { printf '\n== %s\n' "$1"; }

step "Formatting"
cargo fmt --all --check
step "Lints (including the simulation's determinism rules)"
cargo clippy --workspace --all-targets -- -D warnings
step "Tests"
cargo test --workspace
step "The simulation crate has no terminal dependencies (design §2.1)"
if cargo tree -p terra-sim -e normal --prefix none | grep -Ei '^(ratatui|crossterm)'; then
    echo "terra-sim must not depend on terminal crates"
    exit 1
fi

if git diff --quiet HEAD --; then
    git rev-parse 'HEAD^{tree}' > "$(git rev-parse --git-path terra-checks-passed)"
    printf '\nAll checks passed on %s.\n' "$(git rev-parse --short HEAD)"
else
    printf '\nAll checks passed, on changes not yet committed: commit them and pushing runs the checks once more.\n'
fi
