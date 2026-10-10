# Terra Sprites: CI's checks, from PowerShell: .\scripts\check.ps1
# It runs scripts/check.sh with Git for Windows' own bash, as the pre-push
# hook does, so the checks live in one place. Not a bare `bash`, which can be
# WSL's, where there's no Windows cargo. Keep this file ASCII: Windows
# PowerShell 5.1 reads it as ANSI.

$gitBash = Join-Path (git --exec-path) "..\..\..\bin\bash.exe"
if (-not (Test-Path $gitBash)) {
    Write-Host "Couldn't find Git for Windows' bash at $gitBash." -ForegroundColor Red
    exit 1
}
& $gitBash (Join-Path (git rev-parse --show-toplevel) "scripts/check.sh")
exit $LASTEXITCODE
