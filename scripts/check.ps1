# Terra Sprites: the checks CI's test job runs (.github/workflows/ci.yml), in
# the same order. Run it before pushing: .\scripts\check.ps1
# scripts/check.sh is the same for bash. When everything passes on a clean
# working tree, it notes the commit, so the pre-push hook (.githooks/pre-push)
# doesn't run them again.
#
# It runs `cargo fmt --all --check`, not `cargo fmt --all`: the latter would
# reformat other agents' untracked files (docs/agents/how-we-work.md).

$ErrorActionPreference = "Stop"
Set-Location (git rev-parse --show-toplevel)

function Step($name, [scriptblock]$run) {
    Write-Host "`n== $name"
    & $run
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`nFailed: $name" -ForegroundColor Red
        exit 1
    }
}

Step "Formatting" { cargo fmt --all --check }
Step "Lints (including the simulation's determinism rules)" { cargo clippy --workspace --all-targets -- -D warnings }
Step "Tests" { cargo test --workspace }
Write-Host "`n== The simulation crate has no terminal dependencies (design §2.1)"
$terminal = cargo tree -p terra-sim -e normal --prefix none | Select-String -Pattern '^(ratatui|crossterm)'
if ($terminal) {
    Write-Host "terra-sim must not depend on terminal crates" -ForegroundColor Red
    exit 1
}

git diff --quiet HEAD --
if ($LASTEXITCODE -eq 0) {
    git rev-parse 'HEAD^{tree}' | Set-Content -NoNewline -Encoding ascii (git rev-parse --git-path terra-checks-passed)
    Write-Host "`nAll checks passed on $(git rev-parse --short HEAD)." -ForegroundColor Green
} else {
    Write-Host "`nAll checks passed, on changes not yet committed: commit them and pushing runs the checks once more." -ForegroundColor Green
}
