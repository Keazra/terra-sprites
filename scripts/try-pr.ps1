# Terra Sprites: try a pull request's build on this PC.
#
#   .\scripts\try-pr.ps1 170             PR #170, exactly as it is on GitHub, then start the game
#   .\scripts\try-pr.ps1 170 -Seed 7     the same, on world seed 7
#   .\scripts\try-pr.ps1 170 -NoRun      switch to it without starting the game
#   .\scripts\try-pr.ps1 170 --ascii     any further flags go to the game
#   .\scripts\try-pr.ps1 main            back to main, up to date with GitHub (a
#                                        git pull that also mends an old main)
#
# It always takes the newest commit from GitHub, whatever older copy this
# folder has, and says which commit that is. A PR goes on a branch of its own,
# try-pr-170. If main or try-pr-170 has commits GitHub's lacks, they're kept
# on a backup branch first. Files other tools made and never committed are
# left alone: git refuses rather than overwrite one. Keep this file ASCII:
# Windows PowerShell 5.1 reads it as ANSI.

param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Pr,
    [Nullable[UInt64]]$Seed,
    [switch]$NoRun,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$GameFlags
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

# Puts $branch at $target. Commits on $branch that $target lacks, such as a
# main from before the history rewrite of 2026-10-05, go on a backup branch.
function Switch-To($branch, $target) {
    git rev-parse -q --verify "refs/heads/$branch" > $null
    if ($LASTEXITCODE -eq 0) {
        git merge-base --is-ancestor $branch $target
        if ($LASTEXITCODE -ne 0) {
            $backup = "$branch-backup-" + (Get-Date -Format "yyyyMMdd-HHmmss")
            git branch $backup $branch
            if ($LASTEXITCODE -ne 0) { Fail "Couldn't save $branch as $backup, so it's left as it was." }
            Write-Host "$branch had commits GitHub's doesn't have. They're kept on branch $backup." -ForegroundColor Yellow
        }
    }
    git switch -q -C $branch $target
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't switch to $branch. If git named files in the way, move them aside and run this again." }
}

if ($Pr -eq "main") {
    git fetch -q origin main
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't fetch main from GitHub." }
    Switch-To "main" "origin/main"
    $label = "main"
} else {
    $Pr = $Pr.TrimStart('#')
    if ($Pr -notmatch '^\d+$') { Fail "Give a PR number, such as 170, or 'main'." }
    Write-Host "Fetching PR #$Pr from GitHub..."
    git fetch -q origin "pull/$Pr/head"
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't fetch PR #$Pr. Is the number right?" }
    Switch-To "try-pr-$Pr" (git rev-parse FETCH_HEAD)
    $label = "PR #$Pr"
}

$commit = git log -1 '--format=%h, %s (%cr)'
Write-Host "You're on $label at commit $commit." -ForegroundColor Green
Write-Host "The game shows the same commit with: cargo run --release -- --version"

if ($NoRun) { exit 0 }

$gameArgs = @()
if ($null -ne $Seed) { $gameArgs = @("--seed", "$Seed") }
if ($GameFlags) { $gameArgs += $GameFlags }
Write-Host "Building and starting the game (the first build takes a few minutes)..."
# '--' is quoted so PowerShell passes it on rather than reading it itself.
cargo run --release '--' @gameArgs
