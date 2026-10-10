# Terra Sprites: a baseline run (design section 7.6).
#
# Measures GitHub's `main` in a clean worktree of its own, if it has moved
# since the last baseline report, and has the observer write a briefing.
# Task Scheduler runs it daily (scripts/schedule-baseline.ps1); it can also be
# run by hand. Reports, briefings and the log go in docs/reports/.
#
# Everything with an effect is done here. The observer (Gemini, through the
# Antigravity CLI) runs without --dangerously-skip-permissions: it reads, and
# answers with its briefing as structured output. Only a broken run (a panic
# or a broken invariant) files an issue, or comments on the open one.
#
#   -Force     measure even if main hasn't moved
#   -Seeds N   fewer seeds, for trying it out (the report says how many)
#   -DryRun    print the issue it would file or comment, instead of posting it
#   -Ref R     measure R instead of GitHub's main, for trying out a branch
#
# A trial (-Ref, or fewer seeds than 10) is compared with main's last report
# but saved in docs/reports/trials/, never becomes main's baseline, and never
# posts: its issues are only printed.

param(
    [string]$Model = "gemini-3.8-flash-high",
    [int]$Seeds = 10,
    [switch]$Force,
    [switch]$DryRun,
    [string]$Ref = "origin/main"
)

$ErrorActionPreference = "Continue"
$Repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Reports = Join-Path $Repo "docs\reports"
$Worktree = Join-Path (Split-Path -Parent $Repo) "terra-sprites-baseline"
$ObserverDir = Join-Path $Worktree ".baseline"
$LogFile = Join-Path $Reports "baseline.log"
$LastCommit = Join-Path $Reports "last-commit.txt"
$GitHubRepo = "Keazra/terra-sprites"

function Write-Log([string]$Message) {
    $line = "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] $Message"
    Write-Host $line
    [IO.File]::AppendAllText($LogFile, "$line`n", (New-Object System.Text.UTF8Encoding $false))
}

# Writes UTF-8 without a byte-order mark, which PowerShell 5.1's own cmdlets add.
function Write-Text([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding $false))
}

function Read-Text([string]$Path) {
    [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
}

# Runs a program with its output in files, waiting at most $Minutes; returns
# its exit code, or $null if it ran out of time and was stopped. The
# arguments are one string: each one with spaces must already be in quotes.
function Invoke-Logged([string]$Program, [string]$Arguments, [string]$Directory, [string]$Out, [string]$Err, [int]$Minutes) {
    $process = Start-Process -FilePath $Program -ArgumentList $Arguments -WorkingDirectory $Directory `
        -RedirectStandardOutput $Out -RedirectStandardError $Err -NoNewWindow -PassThru
    $null = $process.Handle # PowerShell 5.1 loses the exit code without this
    if (-not $process.WaitForExit($Minutes * 60 * 1000)) {
        # Kill() would stop only cargo or agy, leaving what they started.
        & taskkill.exe /pid $process.Id /T /F | Out-Null
        return $null
    }
    return $process.ExitCode
}

# The Broken section of a report page, or $null if it has none.
function Get-BrokenSection([string]$Page) {
    $match = [regex]::Match($Page, "(?s)## Broken\r?\n\r?\n(.*?)(\r?\n## |\z)")
    if ($match.Success) { return $match.Groups[1].Value.Trim() }
    return $null
}

# Files an issue for a broken run, or comments on the open one, after
# checking for it. With -DryRun it prints what it would post instead.
function Send-Broken([string]$Commit, [string]$Measured, [string]$Broken, [string]$Suspected, [switch]$DryRun) {
    if (-not $Suspected) { $Suspected = "The observer had no guess." }
    $body = @"
## Broken

$Broken

## The observer's guess

$Suspected

Found by the baseline run on ``main`` at $Commit, $Measured (design section 7.6). The guess is the observer's: check it against the code before acting on it.
"@
    $bodyFile = Join-Path $env:TEMP "baseline-broken-$Commit.md"
    Write-Text $bodyFile $body
    # One open issue at a time: a later broken run comments on it.
    $open = & gh issue list --repo $GitHubRepo --state open --search "Baseline run broken in:title" --json number --jq ".[0].number"
    if ($LASTEXITCODE -ne 0) {
        Write-Log "ERROR: couldn't look for an open issue on GitHub, so none was filed or commented on"
        return
    }
    if ($open) {
        if ($DryRun) { Write-Host "Would comment on #${open}:`n$body"; return }
        & gh issue comment $open --repo $GitHubRepo --body-file $bodyFile | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: couldn't comment on #${open} on GitHub"
            return
        }
        Write-Log "Commented on #${open}: still broken at $Commit"
        return
    }
    $title = "Baseline run broken at $Commit"
    $plain = @"
**In plain terms:** the baseline run of ``main`` panicked, or caught the world breaking one of its own rules, on commit $Commit. The Broken section says which scenario and seed, what it said (which names the tick, for a broken rule), and gives the command that replays it exactly.

**Caveat:** the observer's guess at the cause comes from reading the code, not from running it, so treat it as a lead rather than a diagnosis.
"@
    $plainFile = Join-Path $env:TEMP "baseline-broken-$Commit-plain.md"
    Write-Text $plainFile $plain
    if ($DryRun) { Write-Host "Would file: $title (afk)`n$body`nThen comment:`n$plain"; return }
    $url = & gh issue create --repo $GitHubRepo --title $title --label afk --body-file $bodyFile
    if ($LASTEXITCODE -ne 0 -or -not $url) {
        Write-Log "ERROR: couldn't file the issue on GitHub"
        return
    }
    & gh issue comment $url --repo $GitHubRepo --body-file $plainFile | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Log "Filed $url, but ERROR: couldn't add its plain-language comment"
        return
    }
    Write-Log "Filed $url"
}

function Invoke-Baseline {
    New-Item -ItemType Directory -Force -Path $Reports | Out-Null
    $trial = ($Ref -ne "origin/main") -or ($Seeds -ne 10)
    $out = $Reports
    if ($trial) {
        $out = Join-Path $Reports "trials"
        New-Item -ItemType Directory -Force -Path $out | Out-Null
    }

    # 1. GitHub's main, and whether it has moved. A trial of a local ref
    # needs no fetch, so it runs offline too.
    if ($Ref -eq "origin/main") {
        & git -C $Repo fetch --quiet origin main
        if ($LASTEXITCODE -ne 0) { Write-Log "ERROR: couldn't fetch main from GitHub"; return 1 }
    }
    $commit = (& git -C $Repo rev-parse --short $Ref).Trim()
    $previous = $null
    if (Test-Path $LastCommit) { $previous = (Read-Text $LastCommit).Trim() }
    if ($previous -eq $commit -and -not $Force -and -not $trial) {
        Write-Log "main is still at ${commit}: nothing to measure"
        return 0
    }

    # 2. A clean worktree of it, outside the project folder. Its target/
    # folder is ignored, so builds after the first are quick.
    if (-not (Test-Path $Worktree)) {
        & git -C $Repo worktree add --quiet --detach $Worktree $Ref
        if ($LASTEXITCODE -ne 0) { Write-Log "ERROR: couldn't make the worktree at $Worktree"; return 1 }
    }
    & git -C $Worktree reset --quiet --hard $Ref
    & git -C $Worktree clean --quiet -fd
    if ($LASTEXITCODE -ne 0) { Write-Log "ERROR: couldn't clean the worktree at $Worktree"; return 1 }

    # 3. The baseline run.
    $measured = Get-Date -Format "yyyy-MM-dd HH:mm"
    $stem = "$($measured.Substring(0, 10))-$commit"
    $arguments = "run --quiet --profile baseline -p terra-sim --example baseline -- " +
        "--commit $commit --measured `"$measured`" --out `"$out`" --seeds $Seeds"
    $previousReport = $null
    if ($previous) {
        $previousReport = Get-ChildItem -Path $Reports -Filter "*-$previous-report.ron" |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
    }
    if ($previousReport) { $arguments += " --previous `"$($previousReport.FullName)`"" }
    $what = "main"
    if ($trial) { $what = "a trial of $Ref" }
    Write-Log "Measuring $what at $commit$(if ($previousReport) { ", against $previous" })..."
    $code = Invoke-Logged "cargo" $arguments $Worktree (Join-Path $out "$stem-run.log") `
        (Join-Path $out "$stem-run-errors.log") 180
    if ($code -ne 0 -and $code -ne 2) {
        Write-Log "ERROR: the baseline run failed (exit $code); see $stem-run-errors.log"
        return 1
    }
    $isBroken = $code -eq 2
    $page = Read-Text (Join-Path $out "$stem-report.md")
    if (-not $trial) { Write-Text $LastCommit $commit }

    # 4. What the observer reads, inside its workspace: the reports, and the
    # commits since the last one with the files each changed.
    New-Item -ItemType Directory -Force -Path $ObserverDir | Out-Null
    Write-Text (Join-Path $ObserverDir "report.md") $page
    $commits = "This is the first baseline report: there is no earlier commit to compare with."
    if ($previousReport) {
        $previousPage = Join-Path $Reports ($previousReport.Name -replace "\.ron$", ".md")
        if (Test-Path $previousPage) { Copy-Item $previousPage (Join-Path $ObserverDir "previous.md") }
        $commits = (& git -C $Repo log --stat --date=short "--format=%h %ad %s" "$previous..$commit") | Out-String
    }
    Write-Text (Join-Path $ObserverDir "commits.txt") $commits
    $schema = '{"type":"object","properties":{"briefing":{"type":"string"},' +
        '"suspected_cause":{"type":["string","null"]}},"required":["briefing","suspected_cause"]}'
    Write-Text (Join-Path $ObserverDir "schema.json") $schema

    # 5. The observer. The prompt has no double quotes, so it can go on the
    # command line in quotes of its own.
    $prompt = @"
You are the observer for Terra Sprites, an artificial-life simulation in Rust. You are in a clean checkout of $(if ($trial) { $Ref } else { "main" }) at commit $commit.
Everything you need is in the .baseline folder: report.md is today's baseline report, previous.md the last one (if there was one), and commits.txt the commits since then, with the files each changed.
Rules:
- You cannot run commands: read files only. You may read any file in this checkout: the code, the design (docs/design/m1-a-sprite-lives.md is the spec for what this run measures, and its section 7 has the pass marks; docs/design/m2-generations.md plans what comes next; docs/design/changes/ says what changed in them and when) and CONTEXT.md (the project's words, which you should use).
- The simulation is deterministic: the same commit and seed always give the same numbers. Any number that moved since the last report was moved by the commits in commits.txt.
- No data means no data, not a pass. Not met yet means a later slice is meant to meet it.
Write the briefing for the owner, who reads it in the morning:
1. First, two or three plain sentences: what was measured, and what changed since the last report. No tables before this, and no jargon.
2. Then each number that moved, which commit could explain it, and why, from reading the code.
3. Then anything that looks wrong, with your evidence and how sure you are.
If the report has a Broken section, put your best guess at its cause in suspected_cause, citing files and lines. Otherwise set suspected_cause to null.
"@
    $agy = (Get-Command agy.exe -ErrorAction SilentlyContinue).Source
    if (-not $agy) { $agy = Join-Path $env:LOCALAPPDATA "agy\bin\agy.exe" }
    $briefing = Join-Path $out "$stem-briefing.md"
    $suspected = $null
    if (Test-Path $agy) {
        $answer = Join-Path $out "$stem-observer.json"
        # Headless, it may read only what's in its workspace, which --add-dir
        # makes the worktree.
        $arguments = "--model $Model --effort high --add-dir . --output-format json " +
            "--json-schema .baseline/schema.json --print-timeout 30m --print `"$prompt`""
        Write-Log "The observer ($Model) is reading the report..."
        $code = Invoke-Logged $agy $arguments $Worktree $answer (Join-Path $out "$stem-observer-errors.log") 40
        try {
            $result = (Read-Text $answer | ConvertFrom-Json).structured_output
            if (-not $result.briefing) { throw "no briefing in the answer" }
            Write-Text $briefing $result.briefing
            $suspected = $result.suspected_cause
            Write-Log "Briefing saved as $stem-briefing.md"
        } catch {
            Write-Log "ERROR: the observer gave no briefing (exit $code); see $stem-observer.json"
        }
    } else {
        Write-Log "No observer: agy.exe isn't installed, so there's no briefing"
    }

    # 6. A broken run (the program says so with exit code 2) files an issue,
    # or comments on the open one. A trial only prints it.
    if ($isBroken) {
        $broken = Get-BrokenSection $page
        if (-not $broken) { $broken = "The run broke, but its page has no Broken section: see $stem-report.md." }
        Send-Broken -Commit $commit -Measured $measured -Broken $broken -Suspected $suspected -DryRun:($DryRun -or $trial)
    }
    Write-Log "Done: $stem-report.md$(if ($isBroken) { ' (BROKEN)' })"
    return 0
}

# Dot-sourcing the script loads its functions without running it.
if ($MyInvocation.InvocationName -ne ".") {
    $result = Invoke-Baseline
    exit ($result | Select-Object -Last 1)
}
