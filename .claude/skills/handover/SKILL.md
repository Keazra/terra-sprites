---
name: handover
description: Session handover for Terra Sprites. Use when the user starts a session ("start the session", `/handover start`) or ends one ("end the session", "wrap up", `/handover end`).
argument-hint: "start | end"
---

# Handover

A session opens by taking the handover and closes by giving one. Run the branch the argument names, `start` or `end`; with no argument, infer it from the request and ask if it's unclear.

Check each step against live git and GitHub state, since memory records what was true when it was written. Skip a step quietly when there's nothing to do.

## Start: take the handover

1. Sync `main` (`git pull --ff-only`, clean tree) and list open PRs.
2. Find the next slice: the earliest open "Slice N" issue in the current milestone. Confirm its `## Blocked by` issues are closed, and read its comments for items deferred from earlier slices.
3. List open issues with no triage label (roles in `docs/agents/triage-labels.md`). If there are any, suggest the user types `/mattpocock-skills:triage`.
4. Brief the user in a few lines: what merged since last time, what's open, the proposed slice and anything notable in its issue. Wait for their go.
5. On their go, branch `feat/slice-N-<slug>`, read the sections of the current design doc the slice touches, and work the slice loop.

Done when the user has the brief and has chosen what to work on.

## The slice loop

1. Agree the interface and the behaviour list with the user, then build with `mattpocock-skills:tdd`. If the user types `/mattpocock-skills:implement #N` instead, the agreement still comes first.
2. Reach for side skills as the work demands: `mattpocock-skills:diagnosing-bugs` for a bug or slowdown; `mattpocock-skills:prototype` for a feel or design question, then a new `-vN` design revision if the design changes; `mattpocock-skills:domain-modeling` when terms drift.
3. Before opening the PR: the CI checks in `AGENTS.md` pass locally, and `mattpocock-skills:code-review` against `main` has run with every finding that holds up fixed.
4. Open the PR with "Closes #N" in the body.
5. Weigh outside reviews point by point: verify each claim, reply on every thread with what changed or why not, then resolve it.
6. Merge when the user says so; they check the build in Windows Terminal first. Merge commit, delete the branch, pull `main`.

## End: give the handover

1. Every change is committed and pushed to its feature branch; `main` moves only through merged PRs.
2. Follow-ups are on GitHub: an item deferred to a later slice as a comment on that slice's issue, anything else as a new issue labelled `afk` or `hitl`.
3. Project memory reflects the session: progress, the next slice, any new conventions.
4. Hand off to the user in a few lines: what merged, what's open, the next step.
