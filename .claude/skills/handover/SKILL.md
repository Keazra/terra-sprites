---
name: handover
description: Session handover for Terra Sprites. Use when the user starts a session ("start the session", `/handover start`) or ends one ("end the session", "wrap up", `/handover end`).
argument-hint: "start | end"
---

# Handover

A session opens by taking the handover and closes by giving one. Run the branch the argument names, `start` or `end`; with no argument, infer it from the request and ask if it's unclear.

Check each step against live git and GitHub state, since memory records what was true when it was written. Skip a step quietly when there's nothing to do.

How the owner likes to work, and the project's conventions, are in `docs/agents/how-we-work.md`. Read it at the start of every session: a cloud session has no local memory, so that file is what carries them over.

## Start: take the handover

1. Sync `main` (`git pull --ff-only`, clean tree), read `docs/agents/how-we-work.md`, and list open PRs.
2. Find the next slice: the earliest open "Slice N" issue in the current milestone. Confirm its `## Blocked by` issues are closed, and read its comments for items deferred from earlier slices.
3. List open issues with no triage label (roles in `docs/agents/triage-labels.md`). If there are any, suggest the user types `/triage`.
4. Brief the user in a few lines: what merged since last time, what's open, the proposed slice and anything notable in its issue. Wait for their go.
5. On their go, branch `feat/slice-N-<slug>`, read the sections of the current design doc the slice touches, and work the slice loop.

Done when the user has the brief and has chosen what to work on.

## The slice loop

1. Settle the slice's open design questions with `grilling`, in plain words, and record the decisions in a new `-vN` design revision and `CONTEXT.md` (`domain-modeling`). Then agree the interface and the behaviour list with the user, and build with `tdd`. If the user types `/implement #N` instead, the agreement still comes first.
2. Reach for side skills as the work demands: `diagnosing-bugs` for a bug or slowdown; `prototype` for a feel or design question, then a new `-vN` design revision if the design changes; `domain-modeling` when terms drift.
3. Before opening the PR: the CI checks in `AGENTS.md` pass locally, and `two-axis-review` against `main` has run with every finding that holds up fixed.
4. Open the PR with "Closes #N" in the body.
5. Weigh outside reviews point by point: verify each claim, reply on every thread with what changed or why not, then resolve it.
6. Merge when the user says so; they check the build in Windows Terminal first. Merge commit, delete the branch, pull `main`.

## End: give the handover

1. Every change is committed and pushed to its feature branch; `main` moves only through merged PRs.
2. Follow-ups are on GitHub: an item deferred to a later slice as a comment on that slice's issue, anything else as a new issue labelled `afk` or `hitl`.
3. A new convention or owner preference is in `docs/agents/how-we-work.md`, in the feature branch's PR or a small PR of its own. Progress needs no record: GitHub has it. A local session also updates its project memory.
4. Hand off to the user in a few lines: what merged, what's open, the next step.
