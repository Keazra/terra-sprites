# How we work

The owner's preferences and the project's conventions, for any agent session, local or in the cloud. The session routine itself is the `handover` skill (`.claude/skills/handover/SKILL.md`); the rules for git and GitHub are in `AGENTS.md`.

## Working with the owner

- **They review by feel, after trying the build** in Windows Terminal, and change direction when something feels off. Don't build on a direction they haven't confirmed.
- **Plain words, from the start.** Write questions and options for someone who hasn't read the code: say what the player sees or what changes, not step letters, gene jargon or type names. A question once had to be asked again because it wasn't plain.
  - **Name a place by what it's for, not by its file:** "a word list of its own", not `themes/words.ron`. Slice 9b's question about where plurals should live had to be asked again for that reason.
- **They reply only to what they disagree with.** Give a recommendation with each question or list; silence accepts it. Keep going on anything they don't object to.
- **A real decision gets a real question.** Use `AskUserQuestion` with the recommended option first. For anything about layout, give each option an ASCII mockup preview: the owner chose the Chem tab's layout that way.
- **A choice with real trade-offs comes with its pros and cons,** each option's, in plain words, before the question. In slice 8 the owner had to ask for them before choosing how to break the thorn trap.
- **When measuring shows a design won't work,** stop and bring the numbers, what was tried and the options, rather than retuning quietly. Slice 8's decisions to drop no-op Approaches and to shorten the trace were made that way (design v15, changes 9–10).
- **When something they asked for can't be done as written,** say so before building, with options, rather than quietly doing something else. If an agreed detail changes during the work, say so in the PR.
- **When they raise a broad idea, lay out what it could mean before narrowing it.** In slice 9 "the simulated fly brain" was first read as one mechanism, a concept pool, and measured. The owner had meant the brain as a whole, so the idea had to be revisited: first say which readings there are, and ask.
- **`/grill-with-docs`** is the owner's name for the `grilling` skill together with `domain-modeling`: questions in rounds, with each settled term written into `CONTEXT.md` as it's decided, and the decisions in a new design revision.
- **Other agents may work in the same folder.** The owner runs tools such as Antigravity alongside a session, so untracked files you didn't make are theirs: leave them unstaged and untouched, and pull before pushing in case they committed to the branch.

## The owner's design principles

- **The engine enforces physics only.** Behaviour and safety policies belong in each object's data rules, and the data names things. For example, the cause of death "hurt by thornbush" comes from the object type, not from a list in the code.
  - **The one exception is content safety.** A rule the project must never break, whatever a genome or data pack says, is enforced in the engine, because data can be edited and genes evolve. For now there is one: attraction and mating attempts only ever between adults ([#69](https://github.com/Keazra/terra-sprites/issues/69), for M2).
- **Solid means impassable, whatever a sprite has learned.**
- **Each thing we add is its own self-contained thing.** What it is, what it does and what it's called live in its own entry, not spread across lists in the code or other files. So its plural sits beside its name in `objects.ron` (design v17), rather than in a word list of its own.
  - **The aim is easy modding,** as in the Creatures games, whose fun included how simple it was to make new species and new items. When a design choice comes up, say whether it makes adding a species or an item easier or harder.
- **The player has to care first.** Sprites start unnamed; the player names the ones they care about.
- **Terra Sprites is a stepping stone** to a larger game: one adventurer in a world of NPCs that run on the Sprite system, with skills a player can lock and a character that can live on as an NPC ([#47](https://github.com/Keazra/terra-sprites/issues/47)). Nothing is built for it yet, but keep `terra-sim` a general engine, and when a design choice comes up, say whether it helps or hinders reusing sprites as NPCs.
- **Realism settles behaviour questions.** When two behaviours both work, the owner asks which is more realistic. Answer with how real animals behave, then recommend. In slice 9c, fear was kept quiet in a cornered sprite's choice because cornered animals fight.
- **Sprites are blank slates.** They should learn what's good or bad, pass on their genes, and later teach the next generation, so instincts stay general and make mistakes rather than holding knowledge a sprite should earn. "Bored → play with whatever it's looking at", thornbushes included, stays, even when it costs lives until learning arrives (design v12, change 11).
  - **Not competent from the start** (design v16): the starter genome is very basic. Needs lead to actions, and sprites know their own kind and fight or flee, but they know nothing of food, water, toys or danger. They're a species dropped into a world they don't know, meant to evolve fast; the gene types can still express smarter instincts, so evolution can grow them.
- **Real science is inspiration, not a blueprint.** The owner cares how a real system works, such as the fruit-fly brain behind design v16, abstracted for sprites, which are visual, social and tactile. A copy of its parts isn't the goal.

## Design documents

- The spec is the highest `docs/design/*-vN.md`. Each revision is a new copy with a "Changes from vN-1" table at the top, saying what changed, where the decision came from, and which sections it touches.
- A revision not yet on `main` is amended in place within its PR. Once it's on `main`, the next change is a new version.
- **`docs/design/` holds only the current revision of each design.** The PR that adds a revision moves the one it supersedes into `docs/design/archive/` with `git mv`, the new revision's "Supersedes" links point into `archive/`, and the README's design link moves to the new revision. Reviews of a design, such as the `*-eval.md` files, are archived with the revision they reviewed. Issues link to the current revision, since an archived one is history.
- New or changed terms go in `CONTEXT.md` (the `domain-modeling` skill); code and comments use its words, not the ones on its _Avoid_ lists.

## The slice loop

Each slice is a GitHub issue ("Slice N: …") in the M1 milestone, worked as in the `handover` skill:

1. Grill the slice's open design questions (`grilling`), in plain words.
2. Record the decisions in a new design revision and `CONTEXT.md`.
3. Agree the seams (the public interfaces tests go through) and the list of behaviours.
4. Build red-green with `tdd`, one behaviour at a time, committing as it goes.
5. Run `two-axis-review` against `main` (Standards and Spec, in parallel sub-agents). Fix every finding that holds up, and give reasons for the ones declined.
6. Open the PR ("Closes #N"), with the checks in `AGENTS.md` passing.
7. Answer outside reviews (below), then merge when the owner says so: a merge commit, not a squash, then delete the branch and pull `main`. PR descriptions and review replies cite the slice's commits by hash, and a squash would drop them from `main`'s history.

A slice too big for one PR ships as two ("4a", "4b"); only the last PR's description says "Closes #N".

**A slice that promises no visible change proves it.** Run the lab scenarios with the same seeds on `main` and on the branch, and compare the reports byte for byte. Slice 9d, which moved the categories into data, was checked that way: A1 and the thorn trap came out identical.

**A slice that reworks the code and changes behaviour ships the rework first,** proved identical that way, and the behaviour change after it, measured on its own. Then a number that moves can only be the change's doing. Slice 9e shipped as 9e-a (learning per object type, identical reports) and 9e-b (thornbushes join the bushes).

**When a decision waits on a measurement, agree the bar before measuring.** Take the baseline on `main`, and settle with the owner, in numbers, what counts as passing, so the result can't choose its own bar. The bar for moving thornbushes into the bushes was set that way (#77): no worse than `main`, allowing 10% for noise.

**A prototype's totals can hide a particular case,** so the existing behaviour tests are the check while building on its numbers. In slice 9c the default world's numbers looked fine, but the old cornered-sprite test showed fear stopping a cornered sprite from turning on its attacker (4 times in 10, against 8 or more). When a fix comes out of such a case, measure it in the prototype again before bringing it to the owner.

## Outside reviews

The owner runs Gemini, and sometimes Codex, on each PR. They post Gemini's findings as inline PR threads headed "[Review from Gemini]", with a summary review.

- Check each claim against the code before acting on it, and fix what holds up, test-first.
- Reply on every thread, saying what changed (with the commit) or why not, then resolve it. With `gh`: GraphQL `addPullRequestReviewThreadReply` (pass the body with `-F body=@file`), then `resolveReviewThread`.
- Update the PR description to match.

## Triage

Issues are labelled `afk` (an agent can do it alone) or `hitl` (it needs the owner), plus `needs-triage` and `needs-info`; see `docs/agents/triage-labels.md`. Untriaged issues go through `/triage`. Design ideas parked outside M1 are `hitl` issues.
