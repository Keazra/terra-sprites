# Terra Sprites: Gemini reviews a pull request, through the Antigravity CLI.
#
#   .\scripts\review-pr.ps1               lists the open PRs and asks which to review
#   .\scripts\review-pr.ps1 193           reviews PR #193
#   .\scripts\review-pr.ps1 193 -DryRun   shows the review without posting it
#   -Model M                              the model agy uses (agy's own default if not given)
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

param(
    [Parameter(Position = 0)][string]$Pr,
    [string]$Model,
    [switch]$DryRun
)

$ErrorActionPreference = "Continue"
$Repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Instructions = Join-Path $PSScriptRoot "review-pr.md"
$GitHubRepo = "Keazra/terra-sprites"
$Tag = "[Review from Gemini]"
$Grades = @{ 1 = "Critical"; 2 = "Medium"; 3 = "Minor" }
$Confidences = "confirmed", "likely", "unconfirmed"
$MostMinor = 5

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

Set-Location $Repo
$consoleEncoding = [Console]::OutputEncoding
# gh and git write UTF-8; PowerShell 5.1 would read it in the console's code page.
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false } catch { }

$agy = (Get-Command agy.exe -ErrorAction SilentlyContinue).Source
if (-not $agy) { $agy = Join-Path "$env:LOCALAPPDATA" "agy\bin\agy.exe" }
if (-not (Test-Path $agy)) { Fail "Couldn't find agy.exe, the Antigravity CLI." }
if (-not (Test-Path $Instructions)) { Fail "Couldn't find the reviewer's instructions, $Instructions." }

# 1. Which PR.
if (-not $Pr) {
    $listed = & gh pr list --repo $GitHubRepo --state open --json number,title,isDraft
    if ($LASTEXITCODE -ne 0) { Fail "Couldn't list the open PRs on GitHub. Is gh signed in? (gh auth status)" }
    # The parentheses make PowerShell 5.1 hand on the array's items, not the array.
    $open = @((Join-Lines $listed | ConvertFrom-Json) | ForEach-Object { $_ })
    if ($open.Count -eq 0) { Fail "There are no open PRs to review." }
    Write-Host "Open pull requests:"
    foreach ($p in $open) {
        $draft = ""
        if ($p.isDraft) { $draft = " (draft)" }
        Write-Host ("  #{0}  {1}{2}" -f $p.number, $p.title, $draft)
    }
    $Pr = Read-Host "Which PR should Gemini review? (Enter for #$($open[0].number))"
    if (-not $Pr) { $Pr = "$($open[0].number)" }
}
$Pr = $Pr.Trim().TrimStart("#")
if ($Pr -notmatch '^\d+$') { Fail "'$Pr' isn't a PR number." }

$fields = "number,title,body,author,state,isDraft,headRefName,headRefOid,baseRefName,files,commits,url"
$prJson = Join-Lines (& gh pr view $Pr --repo $GitHubRepo --json $fields)
if ($LASTEXITCODE -ne 0) { Fail "Couldn't find PR #$Pr on GitHub." }
$info = $prJson | ConvertFrom-Json
if ($info.state -ne "OPEN") { Fail "PR #$Pr is $($info.state.ToLower()), so there's nothing to review." }
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
    $arguments = "--effort high --add-dir . --output-format json --json-schema .review/schema.json --print-timeout 30m"
    if ($Model) { $arguments = "--model $Model $arguments" }
    $arguments = "$arguments --print `"$prompt`""
    $answer = Join-Path $work "answer.json"
    $errors = Join-Path $work "errors.log"
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
        $result = (Read-Text $answer | ConvertFrom-Json).structured_output
        if (-not $result.summary) { throw "no summary in the answer" }
    } catch {
        Fail "Gemini gave no review (exit $($process.ExitCode)). What it said is in $answer and $errors."
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
        return
    }
    $go = Read-Host "Post this review to PR #$Pr on GitHub? [Y/n]"
    if ($go -and $go -notmatch '^(y|yes)$') {
        Write-Host "Not posted. The review is saved in $reviewFile."
        return
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
    try { [Console]::OutputEncoding = $consoleEncoding } catch { }
}
