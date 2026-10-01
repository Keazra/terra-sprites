# Terra Sprites: schedules the daily baseline run (design section 7.6).
#
# Run it once, in PowerShell, as yourself. It registers TerraSprites-Baseline
# to run scripts/baseline.ps1 at 9:00 every day: as soon as possible after a
# missed start if the PC was off, never waking it. It removes the old
# TerraSprites-DailySoak task, which this replaces.

$script = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "baseline.ps1"
$action = New-ScheduledTaskAction -Execute "powershell.exe" `
    -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$script`""
$trigger = New-ScheduledTaskTrigger -Daily -At 9:00
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Hours 4)
Register-ScheduledTask -TaskName "TerraSprites-Baseline" -Action $action -Trigger $trigger -Settings $settings `
    -Description "Terra Sprites: measures main when it has moved, and has the observer brief on it" -Force | Out-Null
if (Get-ScheduledTask -TaskName "TerraSprites-DailySoak" -ErrorAction SilentlyContinue) {
    Unregister-ScheduledTask -TaskName "TerraSprites-DailySoak" -Confirm:$false
}
Get-ScheduledTask -TaskName "TerraSprites-Baseline" | Select-Object TaskName, State
