# How we work

The owner's preferences and the project's conventions, for any agent session, local or in the cloud. The session routine itself is the `handover` skill (`.claude/skills/handover/SKILL.md`); the rules for git and GitHub are in `AGENTS.md`.

## Working with the owner

- **They review by feel, after trying the build** in Windows Terminal, and change direction when something feels off. Don't build on a direction they haven't confirmed.
- **Plain words, from the start.** Write questions and options for someone who hasn't read the code: say what the player sees or what changes, not step letters, gene jargon or type names. A question once had to be asked again because it wasn't plain.
- **They reply only to what they disagree with.** Give a recommendation with each question or list; silence accepts it. Keep going on anything they don't object to.
- **A real decision gets a real question.** Use `AskUserQuestion` with the recommended option first. For anything about layout, give each option an ASCII mockup preview: the owner chose the Chem tab's layout that way.
- **When something they asked for can't be done as written,** say so before building, with options, rather than quietly doing something else. If an agreed detail changes during the work, say so in the PR.

## The owner's design principles

- **The engine enforces physics only.** Behaviour and safety policies belong in each object's data rules, and the data names things. For example, the cause of death "hurt by thornbush" comes from the object type, not from a list in the code.
- **Solid means impassable, whatever a sprite has learned.**
- **The player has to care first.** Sprites start unnamed; the player names the ones they care about.

## Design documents

- The spec is the highest `docs/design/*-vN.md`. Each revision is a new copy with a "Changes from vN-1" table at the top, saying what changed, where the decision came from, and which sections it touches.
- A revision not yet on `main` is amended in place within its PR. Once it's on `main`, the next change is a new version.
- New or changed terms go in `CONTEXT.md` (the `domain-modeling` skill); code and comments use its words, not the ones on its _Avoid_ lists.

## The slice loop

Each slice is a GitHub issue ("Slice N: …") in the M1 milestone, worked as in the `handover` skill:

1. Grill the slice's open design questions (`grilling`), in plain words.
2. Record the decisions in a new design revision and `CONTEXT.md`.
3. Agree the seams (the public interfaces tests go through) and the list of behaviours.
4. Build red-green with `tdd`, one behaviour at a time, committing as it goes.
5. Run `two-axis-review` against `main` (Standards and Spec, in parallel sub-agents). Fix every finding that holds up, and give reasons for the ones declined.
6. Open the PR ("Closes #N"), with the checks in `AGENTS.md` passing.
7. Answer outside reviews (below), then merge when the owner says so: a merge commit, then delete the branch and pull `main`.

A slice too big for one PR ships as two ("4a", "4b"); only the last PR's description says "Closes #N".

## Outside reviews

The owner runs Gemini, and sometimes Codex, on each PR. They post Gemini's findings as inline PR threads headed "[Review from Gemini]", with a summary review.

- Check each claim against the code before acting on it, and fix what holds up, test-first.
- Reply on every thread, saying what changed (with the commit) or why not, then resolve it. With `gh`: GraphQL `addPullRequestReviewThreadReply` (pass the body with `-F body=@file`), then `resolveReviewThread`.
- Update the PR description to match.

## Triage

Issues are labelled `afk` (an agent can do it alone) or `hitl` (it needs the owner), plus `needs-triage` and `needs-info`; see `docs/agents/triage-labels.md`. Untriaged issues go through `/triage`. Design ideas parked outside M1 are `hitl` issues.
