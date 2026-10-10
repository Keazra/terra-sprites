## What changes

<!-- In plain words, for someone who hasn't read the code: what the player sees or what's different, before and after. -->

## Try it on your PC

<!-- PowerShell, from the terra-sprites folder. Pick a seed that shows the change, and say what to look for. The script pulls the PR's newest commit, whatever branch the folder is on. -->

```powershell
.\scripts\try-pr.ps1 NUMBER -Seed 7
```

What to look for:

## Checks

- [ ] `scripts/check.ps1` or `scripts/check.sh` passed on the last commit (CI's test job: fmt, clippy, every test)
- [ ] `two-axis-review` against `main`, with `docs/agents/review-checklist.md`; what held up is fixed
- [ ] Design and glossary changes, if any, are in the design, in `docs/design/changes/` and in `CONTEXT.md`

## Issues

<!-- "Closes #N" only in the PR that finishes the issue. A PR that only works towards one says "Part of #N", with no "close", "fix" or "resolve" in front of the number. -->
