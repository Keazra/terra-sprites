# Terra Sprites: try a pull request's build on this PC.
#
#   .\scripts\try-pr.ps1 170             PR #170, exactly as it is on GitHub, then start the game
#   .\scripts\try-pr.ps1 170 -Seed 7     the same, on world seed 7
#   .\scripts\try-pr.ps1 170 -NoRun      switch to it without starting the game
#   .\scripts\try-pr.ps1 main            back to main, up to date with GitHub (a
#                                        git pull that also mends an old main)
#
# It always takes the PR's newest commit from GitHub, whatever older copy of
# its branch this folder has, and says which commit that is. It puts the PR on
# a branch of its own, try-pr-170, so nothing else changes. Files other tools
# made and never committed are left alone. Keep this file ASCII: Windows
# PowerShell 5.1 reads it as ANSI.

param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Pr,
    [Nullable[UInt64]]$Seed,
    [switch]$NoRun
)

$ErrorActionPreference = "Continue"
Set-Location (git rev-parse --show-toplevel)

function Fail($why) {
    Write-Host $why -ForegroundColor Red
    exit 1
}

$changed = git status --porcelain --untracked-files=no
if ($changed) {
    Write-Host "These files have changes that aren't committed:"
    $changed | ForEach-Object { Write-Host "  $_" }
    Fail "Put them aside with 'git stash' first ('git stash pop' brings them back), then run this again."
}

if ($Pr -eq "main") {
    git fetch -q origin main
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't fetch main from GitHub." }
    git switch -q main
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't switch to main." }
    # main only moves through merged PRs, so this folder's main should only
    # ever be behind GitHub's. A copy from before the history rewrite of
    # 2026-10-05 can't be pulled at all; keep it on a branch and start over.
    git merge-base --is-ancestor HEAD origin/main
    if ($LASTEXITCODE -ne 0) {
        $backup = "main-backup-" + (Get-Date -Format "yyyyMMdd-HHmmss")
        git branch $backup
        if ($LASTEXITCODE -ne 0) { Fail "Couldn't save this folder's main as $backup, so it's left as it was." }
        Write-Host "This folder's main had commits GitHub's main doesn't have, such as an old copy from before the history was rewritten. They're kept on branch $backup, and main now matches GitHub's." -ForegroundColor Yellow
    }
    git reset -q --hard origin/main
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't bring main up to date with GitHub." }
    $label = "main"
} else {
    if ($Pr -notmatch '^\d+$') { Fail "Give a PR number, such as 170, or 'main'." }
    Write-Host "Fetching PR #$Pr from GitHub..."
    git fetch -q origin "pull/$Pr/head"
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't fetch PR #$Pr. Is the number right?" }
    git switch -q -C "try-pr-$Pr" FETCH_HEAD
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't switch to PR #$Pr." }
    $label = "PR #$Pr"
}

$commit = git log -1 '--format=%h, %s (%cr)'
Write-Host "You're on $label at commit $commit." -ForegroundColor Green
Write-Host "The game shows the same commit with: cargo run --release -- --version"

if ($NoRun) { exit 0 }

$gameArgs = @()
if ($null -ne $Seed) { $gameArgs = @("--seed", "$Seed") }
Write-Host "Building and starting the game (the first build takes a few minutes)..."
# '--' is quoted so PowerShell passes it on rather than reading it itself.
cargo run --release '--' @gameArgs
