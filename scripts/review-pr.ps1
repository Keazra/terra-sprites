# Terra Sprites: Gemini reviews pull requests, through the Antigravity CLI.
#
#   .\scripts\review-pr.ps1               lists the open PRs to tick, then reviews the ticked ones
#   .\scripts\review-pr.ps1 193           reviews PR #193
#   .\scripts\review-pr.ps1 193 -DryRun   shows the review without posting it
#   -Model M                              the model agy uses (agy's own default if not given)
#   -Sandbox                              hands-off, in the review sandbox (below)
#
# Everything with an effect is done here, as in scripts/baseline.ps1. The PR's
# newest commit is checked out in a worktree of its own under TEMP, and what
# the reviewer needs from GitHub is fetched into a .review folder there. agy
# runs in that worktree without --dangerously-skip-permissions, so the
# reviewer only reads; its instructions are scripts/review-pr.md. Its answer
# is structured output. A comment on a line GitHub can't take goes in the
# summary instead, and the review is shown and posted as one review, after
# asking. This folder's branch and files are left alone. Keep this file
# ASCII: Windows PowerShell 5.1 reads it as ANSI.
#
# With -Sandbox, agy runs in FTA2-Review instead: a Linux system (WSL) on the
# owner's PC that can't see the PC's drives or start Windows programs, built
# and shared with the owner's FTA2 project by "Set up the Gemini sandbox.bat"
# in its folder. There the reviewer may also run commands, without asking, in
# the sandbox's own clone of this repository at the PR's commit. .review is
# copied in, Gemini's steps show here as it works, and its answer is read
# back; GitHub is still only reached from here. The review is posted without
# asking, and the sandbox is shut down afterwards. Only the repository
# owner's own PRs are reviewed there: this repository is public, and the
# sandbox also holds the private FTA2 repository and a GitHub key for it,
# which a stranger's PR could try to talk the reviewer into sending out.
#
# With no PR number, the open PRs are listed with where each stands with
# Gemini, to tick with the arrow keys and Space. The ones never reviewed, or
# changed since their review with one of its findings unresolved, start
# ticked. Each ticked PR is then reviewed in turn by this script, run again
# with its number in a PowerShell of its own, so one PR's failure doesn't
# stop the rest.

param(
    [Parameter(Position = 0)][string]$Pr,
    [string]$Model,
    [switch]$DryRun,
    [switch]$Sandbox
)

$ErrorActionPreference = "Continue"
$Repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Instructions = Join-Path $PSScriptRoot "review-pr.md"
$GitHubRepo = "Keazra/terra-sprites"
$Tag = "[Review from Gemini]"
$Grades = @{ 1 = "Critical"; 2 = "Medium"; 3 = "Minor" }
$Confidences = "confirmed", "likely", "unconfirmed"
$MostMinor = 5
$SandboxName = "FTA2-Review"
$Owner = $GitHubRepo.Split("/")[0]
$SandboxSetup = "Run 'Set up the Gemini sandbox.bat', in the FTA2 folder, again."

function Fail([string]$Why) {
    Write-Host $Why -ForegroundColor Red
    exit 1
}

# Writes UTF-8 without a byte-order mark, which PowerShell 5.1's own cmdlets add.
function Write-Text([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding $false))
}

function Read-Text([string]$Path) {
    [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
}

# A program's output as one string, lines joined with LF.
function Join-Lines($Lines) {
    (@($Lines) -join "`n") + "`n"
}

# The diff's lines GitHub can take a comment on: per file, the ranges of the
# new file's lines that its hunks cover.
function Get-DiffRanges([string]$Patch) {
    $ranges = @{}
    $path = $null
    $header = $false
    foreach ($line in ($Patch -split "`n")) {
        if ($line.StartsWith("diff --git ")) {
            $header = $true
            $path = $null
        } elseif ($header -and $line.StartsWith("+++ ")) {
            $header = $false
            $target = $line.Substring(4).Trim().Trim('"')
            if ($target -ne "/dev/null") { $path = $target -replace '^b/', '' }
        } elseif ($path -and $line -match '^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@') {
            $header = $false
            $start = [int]$Matches[1]
            $count = 1
            if ($Matches[2]) { $count = [int]$Matches[2] }
            if ($count -gt 0) {
                if (-not $ranges.ContainsKey($path)) { $ranges[$path] = New-Object System.Collections.ArrayList }
                [void]$ranges[$path].Add(@($start, ($start + $count - 1)))
            }
        }
    }
    return $ranges
}

# Which of the file's hunks holds the line, or -1.
function Find-Hunk($Ranges, [string]$Path, $Line) {
    if ($null -eq $Line -or -not $Ranges.ContainsKey($Path)) { return -1 }
    $hunks = $Ranges[$Path]
    for ($i = 0; $i -lt $hunks.Count; $i++) {
        if ($Line -ge $hunks[$i][0] -and $Line -le $hunks[$i][1]) { return $i }
    }
    return -1
}

function Format-Finding($Finding) {
    "**Grade $($Finding.grade) ($($Grades[$Finding.grade])), $($Finding.confidence):** $($Finding.headline)"
}

function Format-Where($Finding) {
    if ($Finding.start_line) { return "``$($Finding.path):$($Finding.start_line)-$($Finding.line)``" }
    "``$($Finding.path):$($Finding.line)``"
}

# Where an open PR stands with Gemini:
#   none      no Gemini review yet
#   head      Gemini reviewed its newest commit
#   resolved  Gemini reviewed an earlier commit, and every thread it started is resolved
#   open      Gemini reviewed an earlier commit, and a thread it started is still open
function Get-ReviewState($Open) {
    $reviews = @(& gh api "repos/$GitHubRepo/pulls/$($Open.number)/reviews" --paginate --jq '.[] | {commit_id, body}')
    if ($LASTEXITCODE -ne 0) { throw "couldn't read #$($Open.number)'s reviews" }
    $commits = @(foreach ($line in $reviews) {
        $r = $line | ConvertFrom-Json
        if ("$($r.body)".StartsWith($Tag)) { $r.commit_id }
    })
    if ($commits.Count -eq 0) { return "none" }
    if ($commits -contains $Open.headRefOid) { return "head" }
    # Only GraphQL says which threads are resolved. The query has no double
    # quotes, which PowerShell 5.1 would strip from an argument.
    $query = 'query($owner: String!, $name: String!, $number: Int!) { repository(owner: $owner, name: $name) ' +
        '{ pullRequest(number: $number) { reviewThreads(first: 100) { nodes { isResolved comments(first: 1) { nodes { body } } } } } } }'
    $owner, $name = $GitHubRepo.Split("/")
    $answer = Join-Lines (& gh api graphql -f "query=$query" -f "owner=$owner" -f "name=$name" -F "number=$($Open.number)")
    if ($LASTEXITCODE -ne 0) { throw "couldn't read #$($Open.number)'s review threads" }
    foreach ($thread in @(($answer | ConvertFrom-Json).data.repository.pullRequest.reviewThreads.nodes)) {
        $first = @($thread.comments.nodes)[0]
        if (-not $thread.isResolved -and $first -and "$($first.body)".StartsWith($Tag)) { return "open" }
    }
    "resolved"
}

# Whether this window can read single key presses, for the tick list (it
# can't when its input comes from a file or another program).
function Test-KeysReadable {
    try { return (-not [Console]::IsInputRedirected) -and ([Console]::WindowWidth -gt 0) } catch { return $false }
}

# Writes one line of the tick list in place, cut to the window's width so it
# never wraps and pushes the rest of the list out of place.
function Write-ListLine([int]$Line, [string]$Text, [bool]$Current) {
    $width = [Math]::Max(20, [Math]::Min([Console]::WindowWidth, [Console]::BufferWidth) - 1)
    if ($Text.Length -gt $width) { $Text = $Text.Substring(0, $width) }
    [Console]::SetCursorPosition(0, $Line)
    if ($Current) {
        Write-Host $Text.PadRight($width) -NoNewline -ForegroundColor Black -BackgroundColor Gray
    } else {
        Write-Host $Text.PadRight($width) -NoNewline
    }
}

# The open PRs as a list to move through with the arrow keys, ticking each
# with Space. Returns the ticked numbers on Enter, or none on Q or Esc.
function Select-PullRequests($Rows) {
    $count = $Rows.Count
    $current = 0
    Write-Host "Tick the PRs to review. The ones that need a review start ticked."
    Write-Host "  Up/Down: move   Space: tick or untick   A: tick all   N: untick all"
    Write-Host "  Enter: review the ticked ones   Q: quit"
    Write-Host ""
    # Room for the list and a message line under it; the window may have scrolled.
    for ($i = 0; $i -le $count; $i++) { Write-Host "" }
    $top = [Console]::CursorTop - ($count + 1)
    $message = ""
    # The tick box and number take 12 characters; titles get what's left
    # after the longest status, so each status stays in view.
    $statusWidth = ($Rows | ForEach-Object { $_.Status.Length } | Measure-Object -Maximum).Maximum
    $titleWidth = [Math]::Max(15, [Math]::Min(60, [Console]::WindowWidth - 1 - 12 - 2 - $statusWidth))
    try { [Console]::CursorVisible = $false } catch { }
    try {
        while ($true) {
            for ($i = 0; $i -lt $count; $i++) {
                $row = $Rows[$i]
                $mark = " "
                if ($row.Ticked) { $mark = "x" }
                $pointer = " "
                if ($i -eq $current) { $pointer = ">" }
                $title = $row.Title
                if ($title.Length -gt $titleWidth) { $title = $title.Substring(0, $titleWidth - 3) + "..." }
                $text = "{0} [{1}] #{2,-4} {3}  {4}" -f $pointer, $mark, $row.Number, $title.PadRight($titleWidth), $row.Status
                Write-ListLine ($top + $i) $text ($i -eq $current)
            }
            Write-ListLine ($top + $count) $message $false
            $message = ""
            $key = [Console]::ReadKey($true)
            switch ($key.Key) {
                "UpArrow" { if ($current -gt 0) { $current-- } }
                "DownArrow" { if ($current -lt $count - 1) { $current++ } }
                "Home" { $current = 0 }
                "End" { $current = $count - 1 }
                "Spacebar" { $Rows[$current].Ticked = -not $Rows[$current].Ticked }
                "A" { foreach ($row in $Rows) { $row.Ticked = $true } }
                "N" { foreach ($row in $Rows) { $row.Ticked = $false } }
                "Enter" {
                    $ticked = @($Rows | Where-Object { $_.Ticked } | ForEach-Object { $_.Number })
                    if ($ticked.Count -gt 0) { return ,$ticked }
                    $message = "  Nothing is ticked. Tick a PR with Space, or press Q to quit."
                }
                "Q" { return ,@() }
                "Escape" { return ,@() }
            }
        }
    } finally {
        try { [Console]::SetCursorPosition(0, $top + $count + 1) } catch { }
        try { [Console]::CursorVisible = $true } catch { }
    }
}

# The typed way to pick, for a window that can't show the tick list: the
# PRs' numbers, or Enter alone for the ones that start ticked.
function Read-TypedChoice($Rows) {
    foreach ($row in $Rows) { Write-Host ("  #{0,-4} {1,-60}  {2}" -f $row.Number, $row.Title, $row.Status) }
    $numbers = @($Rows | ForEach-Object { $_.Number })
    $needed = @($Rows | Where-Object { $_.Ticked } | ForEach-Object { $_.Number })
    while ($true) {
        $typed = Read-Host "Which PRs? Their numbers (like 193 195), or Enter for the ones that need a review"
        if ("$typed".Trim() -eq "") { return ,$needed }
        $chosen = @()
        foreach ($word in ("$typed" -split '[\s,]+' | Where-Object { $_ -ne "" })) {
            $word = $word.TrimStart("#")
            if ($word -notmatch '^\d{1,9}$' -or $numbers -notcontains [int]$word) {
                Write-Host "  '$word' isn't one of the open PRs."
                $chosen = $null
                break
            }
            if ($chosen -notcontains [int]$word) { $chosen += [int]$word }
        }
        if ($chosen) { return ,$chosen }
    }
}

# Whether the review sandbox exists. Older WSL answers in UTF-16 despite
# WSL_UTF8, which arrives with a NUL between letters, so those go.
function Test-SandboxExists {
    $names = (& wsl.exe --list --quiet 2>$null) | ForEach-Object { ($_ -replace "`0", "").Trim() }
    @($names) -contains $SandboxName
}

# A bash script as one argument for wsl.exe. PowerShell 5.1 mangles quotes in
# arguments to other programs, so the script travels as base64 and becomes a
# file again inside. It runs as a login shell, which puts Rust on the PATH.
function Get-SandboxCommand([string]$Script) {
    $b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($Script -replace "`r`n", "`n")))
    "f=`$(mktemp) && echo $b64 | base64 -d > `$f && bash -l `$f; rc=`$?; rm -f `$f; exit `$rc"
}

# Stops the sandbox, and anything Gemini left running in it. When no other
# Linux system is running, WSL's engine is closed too, rather than left
# idling until Windows closes it.
function Stop-Sandbox {
    & wsl.exe --terminate $SandboxName 2>$null | Out-Null
    $running = @((& wsl.exe --list --running --quiet 2>$null) | ForEach-Object { ($_ -replace "`0", "").Trim() } | Where-Object { $_ -ne "" })
    if ($LASTEXITCODE -eq 0 -and $running.Count -eq 0) { & wsl.exe --shutdown 2>$null | Out-Null }
}

# Gemini's review from agy's answer: its structured output, or failing that
# its reply, if that is the review's JSON (perhaps in a code fence).
function Get-Review($Output) {
    if ($Output.structured_output) { return $Output.structured_output }
    $reply = "$($Output.response)".Trim() -replace '^```(?:json)?\s*', '' -replace '\s*```$', ''
    try { return ($reply | ConvertFrom-Json) } catch { return $null }
}

Set-Location $Repo
$consoleEncoding = [Console]::OutputEncoding
# gh and git write UTF-8; PowerShell 5.1 would read it in the console's code page.
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false } catch { }

if ($Sandbox) {
    # WSL's own messages in plain text rather than UTF-16.
    $env:WSL_UTF8 = "1"
    if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) { Fail "-Sandbox needs WSL. $SandboxSetup" }
    if (-not (Test-SandboxExists)) { Fail "There's no $SandboxName sandbox on this PC. $SandboxSetup" }
} else {
    $agy = (Get-Command agy.exe -ErrorAction SilentlyContinue).Source
    if (-not $agy) { $agy = Join-Path "$env:LOCALAPPDATA" "agy\bin\agy.exe" }
    if (-not (Test-Path $agy)) { Fail "Couldn't find agy.exe, the Antigravity CLI." }
}
if (-not (Test-Path $Instructions)) { Fail "Couldn't find the reviewer's instructions, $Instructions." }

# 1. Which PRs. With none given, the owner ticks them, and each is reviewed
# by this script run again with its number.
if (-not $Pr) {
    $listed = & gh pr list --repo $GitHubRepo --state open --limit 100 --json number,title,isDraft,headRefOid,author
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't list the open PRs on GitHub. Is gh signed in? (gh auth status)" }
    # The parentheses make PowerShell 5.1 hand on the array's items, not the array.
    $open = @((Join-Lines $listed | ConvertFrom-Json) | ForEach-Object { $_ } | Sort-Object { [int]$_.number })
    if ($open.Count -eq 0) { Fail "There are no open PRs to review." }
    Write-Host "Looking up where each open PR stands with Gemini..."
    $rows = @(foreach ($p in $open) {
        $status = "draft"
        $needed = $false
        if ($Sandbox -and $p.author.login -ne $Owner) {
            $status = "by $($p.author.login): not in the sandbox"
        } elseif (-not $p.isDraft) {
            # If GitHub can't say, count it as needing a review: a second
            # review costs less than a missed one.
            $state = "unknown"
            try { $state = Get-ReviewState $p } catch { }
            switch ($state) {
                "none" { $status = "needs a review"; $needed = $true }
                "head" { $status = "reviewed at its latest commit" }
                "resolved" { $status = "reviewed, findings all resolved" }
                "open" { $status = "changed since review, findings open"; $needed = $true }
                default { $status = "needs a review (couldn't check earlier reviews)"; $needed = $true }
            }
        }
        $title = "$($p.title)"
        if ($title.Length -gt 60) { $title = $title.Substring(0, 57) + "..." }
        [pscustomobject]@{ Number = [int]$p.number; Title = $title; Status = $status; Ticked = $needed }
    })
    Write-Host ""
    $chosen = $null
    if ((Test-KeysReadable) -and ($rows.Count + 8 -lt [Console]::WindowHeight)) {
        try {
            $chosen = Select-PullRequests $rows
        } catch {
            Write-Host "The tick list couldn't be shown in this window ($($_.Exception.Message)), so type the numbers instead."
        }
    }
    if ($null -eq $chosen) { $chosen = Read-TypedChoice $rows }
    if ($chosen.Count -eq 0) {
        Write-Host "No reviews started."
        exit 0
    }
    Write-Host ("Reviewing " + (($chosen | ForEach-Object { "#$_" }) -join ", ") + ".")
    $shell = (Get-Process -Id $PID).Path
    $results = @()
    try {
        foreach ($number in $chosen) {
            Write-Host ""
            $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $PSCommandPath, "$number")
            if ($Model) { $arguments += @("-Model", $Model) }
            if ($DryRun) { $arguments += "-DryRun" }
            if ($Sandbox) { $arguments += "-Sandbox" }
            & $shell @arguments
            if ($LASTEXITCODE -eq 0) { $results += "  #$number  done" } else { $results += "  #$number  stopped: see its messages above" }
        }
    } finally {
        # Each review shuts the sandbox down itself; this covers Ctrl+C.
        if ($Sandbox) { Stop-Sandbox }
    }
    Write-Host ""
    Write-Host "All done:"
    $results | ForEach-Object { Write-Host $_ }
    exit 0
}
$Pr = $Pr.Trim().TrimStart("#")
if ($Pr -notmatch '^\d+$') { Fail "'$Pr' isn't a PR number." }

$fields = "number,title,body,author,state,isDraft,headRefName,headRefOid,baseRefName,files,commits,url"
$prJson = Join-Lines (& gh pr view $Pr --repo $GitHubRepo --json $fields)
if ($LASTEXITCODE -ne 0) { Fail "Couldn't find PR #$Pr on GitHub." }
$info = $prJson | ConvertFrom-Json
if ($info.state -ne "OPEN") { Fail "PR #$Pr is $($info.state.ToLower()), so there's nothing to review." }
if ($Sandbox -and $info.author.login -ne $Owner) {
    Fail "PR #$Pr is by $($info.author.login), not $Owner, so it isn't reviewed in the sandbox (see this script's opening comment). Review it without -Sandbox."
}
$head = $info.headRefOid
$short = $head.Substring(0, 7)
Write-Host "Reviewing #${Pr}: $($info.title), at $short."

# 2. The PR's newest commit, in a worktree of its own.
$work = Join-Path ([IO.Path]::GetTempPath()) "terra-review-$Pr"
$tree = Join-Path $work "tree"
& git worktree remove --force $tree 2>$null
& git worktree prune
if (Test-Path $work) { Remove-Item -Recurse -Force $work }
New-Item -ItemType Directory -Force -Path $work | Out-Null
& git fetch --quiet origin "pull/$Pr/head"
if ($LASTEXITCODE -ne 0) { Fail "Couldn't fetch PR #$Pr from GitHub." }
& git worktree add --quiet --detach $tree $head
if ($LASTEXITCODE -ne 0) { Fail "Couldn't check PR #$Pr out in $tree." }

try {
    # 3. What the reviewer needs from GitHub, since it can't run commands.
    $inputs = Join-Path $tree ".review"
    New-Item -ItemType Directory -Force -Path $inputs | Out-Null
    Copy-Item $Instructions (Join-Path $inputs "instructions.md")
    Write-Text (Join-Path $inputs "pr.json") $prJson
    $patch = Join-Lines (& gh pr diff $Pr --repo $GitHubRepo)
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't fetch PR #$Pr's diff." }
    Write-Text (Join-Path $inputs "diff.patch") $patch
    # gh pr checks fails when a check fails or is still running: its output is the point.
    $checks = & gh pr checks $Pr --repo $GitHubRepo 2>&1
    Write-Text (Join-Path $inputs "checks.txt") (Join-Lines $checks)
    $comments = @(& gh api "repos/$GitHubRepo/pulls/$Pr/comments" --paginate `
        --jq '.[] | {id, in_reply_to_id, user: .user.login, path, line, original_line, original_commit_id, created_at, body}')
    Write-Text (Join-Path $inputs "comments.jsonl") (Join-Lines $comments)
    $reviews = @(& gh api "repos/$GitHubRepo/pulls/$Pr/reviews" --paginate `
        --jq '.[] | {user: .user.login, state, commit_id, submitted_at, body}')
    Write-Text (Join-Path $inputs "reviews.jsonl") (Join-Lines $reviews)
    Write-Text (Join-Path $inputs "conversation.txt") (Join-Lines (& gh pr view $Pr --repo $GitHubRepo --comments))
    $issues = [regex]::Matches("$($info.body)", '(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?|part of)\s+#(\d+)') |
        ForEach-Object { $_.Groups[1].Value } | Select-Object -Unique
    foreach ($issue in $issues) {
        $text = & gh issue view $issue --repo $GitHubRepo --comments 2>$null
        if ($LASTEXITCODE -eq 0) { Write-Text (Join-Path $inputs "issue-$issue.txt") (Join-Lines $text) }
    }

    # A PR Gemini has reviewed before: what changed since that review's commit.
    $last = $null
    foreach ($line in $reviews) {
        $r = $line | ConvertFrom-Json
        if ("$($r.body)".StartsWith($Tag) -and ($null -eq $last -or $r.submitted_at -gt $last.at)) {
            $last = @{ at = $r.submitted_at; commit = $r.commit_id }
        }
    }
    foreach ($line in $comments) {
        $c = $line | ConvertFrom-Json
        if ("$($c.body)".StartsWith($Tag) -and ($null -eq $last -or $c.created_at -gt $last.at)) {
            $last = @{ at = $c.created_at; commit = $c.original_commit_id }
        }
    }
    if ($last -and $last.commit -and $last.commit -ne $head) {
        & git cat-file -e "$($last.commit)^{commit}" 2>$null
        if ($LASTEXITCODE -eq 0) {
            $since = (Join-Lines (& git log --format="%h %s" "$($last.commit)..$head")) + "`n" +
                (Join-Lines (& git diff $last.commit $head))
            Write-Text (Join-Path $inputs "since-last-review.patch") $since
            Write-Host "Gemini reviewed this PR before, at $($last.commit.Substring(0, 7)): it looks at what changed since."
        }
    }

    $schema = '{"type":"object","properties":{"summary":{"type":"string"},"comments":{"type":"array","items":' +
        '{"type":"object","properties":{"path":{"type":"string"},"line":{"type":"integer"},' +
        '"start_line":{"type":["integer","null"]},"grade":{"type":"integer","enum":[1,2,3]},' +
        '"confidence":{"type":"string","enum":["confirmed","likely","unconfirmed"]},' +
        '"headline":{"type":"string"},"details":{"type":"string"}},' +
        '"required":["path","line","start_line","grade","confidence","headline","details"]}}},' +
        '"required":["summary","comments"]}'
    Write-Text (Join-Path $inputs "schema.json") $schema

    # 4. The reviewer. The prompt has no double quotes, so it can go on the
    # command line in quotes of its own. Headless, agy may read only what's in
    # its workspace, which --add-dir makes the worktree.
    $prompt = "You are reviewing pull request $Pr of $GitHubRepo, checked out here at commit $short. " +
        "Read .review/instructions.md and follow it."
    $answer = Join-Path $work "answer.json"
    $errors = Join-Path $work "errors.log"
    if ($Sandbox) {
        if ($Model -and $Model -notmatch '^[A-Za-z0-9._:/-]+$') { Fail "'$Model' isn't a model name." }
        $log = "\\wsl$\$SandboxName\home\reviewer\review-logs\terra-pr-$Pr.txt"

        # 4a. The sandbox's clone at the PR's commit, with .review copied in. The
        # files travel as one line each, name and base64, on the script's input.
        $bundle = (Get-ChildItem -LiteralPath $inputs -File | ForEach-Object {
            "$($_.Name) $([Convert]::ToBase64String([IO.File]::ReadAllBytes($_.FullName)))"
        }) -join "`n"
        $prepare = @'
set -u
bundle=$(mktemp)
cat > "$bundle"
if ! command -v review-walls-check >/dev/null || ! command -v review-show >/dev/null || [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    echo "The sandbox doesn't have what Terra Sprites reviews need yet."
    exit 5
fi
review-walls-check || exit 4
set -e
clone="$HOME/terra-sprites"
if [ ! -d "$clone/.git" ]; then
    echo "Copying the repository into the sandbox, the first time only..."
    git clone --quiet https://github.com/@REPO@.git "$clone"
fi
cd "$clone"
git fetch --quiet origin main "pull/@PR@/head"
git checkout --quiet --force --detach @HEAD@
# Last review's files go, but not target/: builds are kept between reviews,
# unless they've grown past 5 GB.
if [ -d target ] && [ "$(du -sm target | cut -f1)" -gt 5000 ]; then rm -rf target; fi
git clean --quiet -ffdx -e /target
grep -qx '/.review/' .git/info/exclude || echo '/.review/' >> .git/info/exclude
mkdir .review
python3 -c 'import base64, os, sys
for line in open(sys.argv[1]):
    name, _, data = line.strip().partition(" ")
    if name and name == os.path.basename(name) and not name.startswith("."):
        with open(os.path.join(".review", name), "wb") as f:
            f.write(base64.b64decode(data))' "$bundle"
rm -f "$bundle"
echo "Checking Rust is up to date..."
"$HOME/.cargo/bin/rustup" update stable --no-self-update >/dev/null 2>&1 || echo "(Couldn't check; the Rust there is used.)"
mkdir -p "$HOME/review-logs"
rm -f "$HOME/review-logs/terra-pr-@PR@.json"
'@
        $prepare = $prepare.Replace("@REPO@", $GitHubRepo).Replace("@PR@", $Pr).Replace("@HEAD@", $head)
        Write-Host "Getting the sandbox ready..."
        $bundle | & wsl.exe --distribution $SandboxName --user reviewer --cd "~" --exec bash -c (Get-SandboxCommand $prepare) | Out-Host
        $code = $LASTEXITCODE
        if ($code -eq 4) { Fail "The sandbox's walls aren't up, so Gemini wasn't started. $SandboxSetup" }
        if ($code -eq 5) { Fail "The sandbox isn't set up for Terra Sprites yet. $SandboxSetup" }
        if ($code -ne 0) { Fail "Couldn't get PR #$Pr ready in the sandbox (exit $code)." }

        # 4b. Gemini, hands-off. Its steps show here as it works, through the
        # sandbox's review-show, which also saves its answer. wsl.exe writes
        # straight to this window, so this line mustn't be piped or captured.
        $flags = "--effort high --add-dir . --output-format stream-json --json-schema .review/schema.json --print-timeout 45m"
        if ($Model) { $flags = "--model $Model $flags" }
        $sandboxPrompt = "$prompt You are in the sandbox, so section 4 of the instructions applies: you may run commands."
        $run = @'
set -u
review-walls-check || exit 4
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
cd "$HOME/terra-sprites" || exit 5
set -o pipefail
agy -p "@PROMPT@" --dangerously-skip-permissions @FLAGS@ \
    | python3 /usr/local/bin/review-show --log terra-pr-@PR@ --save-result "$HOME/review-logs/terra-pr-@PR@.json"
'@
        $run = $run.Replace("@PROMPT@", $sandboxPrompt).Replace("@FLAGS@", $flags).Replace("@PR@", $Pr)
        Write-Host "Gemini is reviewing the PR in the sandbox, hands-off. Its steps show below; a copy is kept in $log."
        & wsl.exe --distribution $SandboxName --user reviewer --cd "~" --exec bash -c (Get-SandboxCommand $run)
        $code = $LASTEXITCODE
        if ($code -eq 4) { Fail "The sandbox's walls aren't up, so Gemini wasn't started. $SandboxSetup" }
        $saved = & wsl.exe --distribution $SandboxName --user reviewer --cd "~" --exec cat "/home/reviewer/review-logs/terra-pr-$Pr.json" 2>$null
        Write-Text $answer (Join-Lines $saved)
        Stop-Sandbox
        $stopped = $true
        $result = $null
        try { $result = Get-Review (Read-Text $answer | ConvertFrom-Json) } catch { }
        if (-not $result.summary) { Fail "Gemini gave no review (exit $code). What it did is in $log." }
    } else {
        $arguments = "--effort high --add-dir . --output-format json --json-schema .review/schema.json --print-timeout 30m"
        if ($Model) { $arguments = "--model $Model $arguments" }
        $arguments = "$arguments --print `"$prompt`""
        Write-Host "Gemini is reading the PR. This takes a few minutes..."
        $process = Start-Process -FilePath $agy -ArgumentList $arguments -WorkingDirectory $tree `
            -RedirectStandardOutput $answer -RedirectStandardError $errors -NoNewWindow -PassThru
        $null = $process.Handle # PowerShell 5.1 loses the exit code without this
        if (-not $process.WaitForExit(40 * 60 * 1000)) {
            # Kill() would stop only agy, leaving what it started.
            & taskkill.exe /pid $process.Id /T /F | Out-Null
            Fail "Gemini didn't finish within 40 minutes, so it was stopped. Its output is in $work."
        }
        try {
            $result = Get-Review (Read-Text $answer | ConvertFrom-Json)
            if (-not $result.summary) { throw "no summary in the answer" }
        } catch {
            Fail "Gemini gave no review (exit $($process.ExitCode)). What it said is in $answer and $errors."
        }
    }

    # 5. The review: each comment on a line of the diff, or in the summary.
    $ranges = Get-DiffRanges $patch
    $inline = New-Object System.Collections.ArrayList
    $elsewhere = New-Object System.Collections.ArrayList
    $minor = 0
    foreach ($f in @($result.comments)) {
        if ($null -eq $f) { continue }
        $f.path = ("$($f.path)" -replace '\\', '/') -replace '^\./', ''
        if ($Grades.Keys -notcontains [int]$f.grade) { $f.grade = 3 }
        $f.grade = [int]$f.grade
        $confidence = "$($f.confidence)".Trim().ToLowerInvariant()
        if ($Confidences -notcontains $confidence) { $confidence = "unconfirmed" }
        $f | Add-Member -Force -NotePropertyName confidence -NotePropertyValue $confidence
        $hunk = Find-Hunk $ranges $f.path $f.line
        if ($f.start_line -and ($f.start_line -ge $f.line -or (Find-Hunk $ranges $f.path $f.start_line) -ne $hunk)) {
            $f.start_line = $null
        }
        if ($hunk -lt 0) { [void]$elsewhere.Add($f); continue }
        if ($f.grade -eq 3) {
            $minor++
            if ($minor -gt $MostMinor) { [void]$elsewhere.Add($f); continue }
        }
        [void]$inline.Add($f)
    }
    $inline = @(foreach ($g in 1, 2, 3) { $inline | Where-Object { $_.grade -eq $g } })

    $posted = @()
    foreach ($f in $inline) {
        $comment = @{ path = $f.path; line = [int]$f.line; side = "RIGHT"
            body = "$Tag`n$(Format-Finding $f)`n`n$($f.details)" }
        if ($f.start_line) {
            $comment.start_line = [int]$f.start_line
            $comment.start_side = "RIGHT"
        }
        $posted += $comment
    }
    $counts = foreach ($g in 1, 2, 3) {
        "$(@($inline | Where-Object { $_.grade -eq $g }).Count) $($Grades[$g])"
    }
    $body = "$Tag`n## Summary of #$Pr at $short`n`n$($result.summary.Trim())`n`n**Findings inline:** $($counts -join ', ')"
    $unconfirmed = @($inline | Where-Object { $_.confidence -eq "unconfirmed" }).Count
    if ($unconfirmed -gt 0) { $body += " ($unconfirmed of them unconfirmed)" }
    foreach ($f in $inline) { $body += "`n- $(Format-Finding $f) $(Format-Where $f)" }
    if ($elsewhere.Count -gt 0) {
        $body += "`n`n**Not on a line of the diff, or past the first $MostMinor Minor notes:**"
        foreach ($f in $elsewhere) {
            $details = ("$($f.details)".Trim() -split "`n") -join "`n  "
            $body += "`n- $(Format-Finding $f) $(Format-Where $f)`n  $details"
        }
    }
    $review = @{ commit_id = $head; event = "COMMENT"; body = $body; comments = @($posted) }
    $reviewFile = Join-Path $work "review.json"
    Write-Text $reviewFile ($review | ConvertTo-Json -Depth 5)

    # 6. Show it, then post it.
    Write-Host ""
    Write-Host $body
    Write-Host ""
    Write-Host "$($inline.Count) inline comments, saved with the summary in $reviewFile."
    if ($DryRun) {
        Write-Host "A dry run: nothing was posted."
        exit 0
    }
    # In the sandbox, hands-off, it's posted without asking.
    if (-not $Sandbox) {
        $go = Read-Host "Post this review to PR #$Pr on GitHub? [Y/n]"
        if ($go -and $go -notmatch '^(y|yes)$') {
            Write-Host "Not posted. The review is saved in $reviewFile."
            exit 0
        }
    }
    $now = (& gh pr view $Pr --repo $GitHubRepo --json headRefOid --jq .headRefOid)
    if ($now -and $now -ne $head) {
        Write-Host "PR #$Pr has new commits since $short. The review stays on $short, where its line numbers are right." -ForegroundColor Yellow
    }
    $url = & gh api "repos/$GitHubRepo/pulls/$Pr/reviews" --method POST --input $reviewFile --jq .html_url
    if ($LASTEXITCODE -ne 0) { Fail "GitHub refused the review. It's saved in $reviewFile." }
    Write-Host "Posted: $url" -ForegroundColor Green
} finally {
    & git worktree remove --force $tree 2>$null
    & git worktree prune
    # Also when Ctrl+C or a failure stops the review part-way.
    if ($Sandbox -and -not $stopped) { Stop-Sandbox }
    try { [Console]::OutputEncoding = $consoleEncoding } catch { }
}
exit 0
