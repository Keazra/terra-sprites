# Review checklist

What the outside reviews of #97 to #168 kept finding that our own review had missed. `two-axis-review` gives this list to both its sub-agents; check each point against the diff before opening a PR.

| Check | What went wrong without it |
|---|---|
| **The checks pass on the last commit:** `scripts/check.sh` or `.\scripts\check.ps1`, every test, the unit tests in `src/` too | #163 (an unused helper failed clippy), #167 (imports in the old order failed fmt), #168 (a unit test in `brain.rs` still expected the old rule) |
| **The glossary and the design say what the code does today,** no more. A term for something not built yet is marked as planned, and a new term doesn't contradict another entry's _Avoid_ list | #152 ("sees empty" where the code forgets what's gone; `strength` where the glossary says recall), #165 (four findings: verbs and sizes that don't exist yet, and Touch both defined and avoided) |
| **Saves and replays from before still load,** or the change says why they needn't. New saved fields have defaults that agree with `data/` | #152 (a world remembering 4 places of a kind was refused on load) |
| **Every new rule has a test, seen red** with the rule broken for a moment, through the input that reaches it in play | #152 (trips held up by a sprite had none) |
| **One source of truth.** A second copy of a rule (rows rebuilt for clicks, a range written in code and in data) goes through the first instead | #168 (the Brain tab's clicks skipped the tab-visibility check that drawing applied) |
| **Numbers in the design can be run again** from what's committed: the command, the seeds and the commit | #152 (the scarce-water table came from an uncommitted build) |
| **The design's change file and "Closes #N"** match what the PR does: "Part of #N" when it only works towards an issue | #8 closed early when another PR said "Only 7c's PR closes #8" |
