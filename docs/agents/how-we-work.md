# How we work

The owner's preferences and the project's conventions, for any agent session, local or in the cloud. Starting and finishing a session, the checks before pushing, and the rules for git and GitHub are in `AGENTS.md`.

## Working with the owner

- **They review by feel, after trying the build** in Windows Terminal, and change direction when something feels off. Don't build on a direction they haven't confirmed.
  - **Every PR says how to try it,** in PowerShell, starting with `git switch main` and `git pull`, then `.\scripts\try-pr.ps1 <number>`, which takes the PR's newest commit whatever older copy of the branch their folder has, and says which commit it is (`--version` says it too). Say what to look for, and which seed shows it.
- **Plain words, from the start.** Write questions and options for someone who hasn't read the code: say what the player sees or what changes, not step letters, gene jargon or type names. A question once had to be asked again because it wasn't plain.
  - **Name a place by what it's for, not by its file:** "a word list of its own", not `themes/words.ron`. Slice 9b's question about where plurals should live had to be asked again for that reason.
  - **Results too.** Before any numbers, say in a sentence or two what was tested and why, then what happened, as a story; tables come after. In slice 10 the owner had to ask twice ("in plain language?", "what is it we are testing, again?") before a measurement round made sense to them.
- **They reply only to what they disagree with.** Give a recommendation with each question or list; silence accepts it. Keep going on anything they don't object to.
- **A real decision gets a real question.** Use `AskUserQuestion` with the recommended option first. For anything about layout, give each option an ASCII mockup preview: the owner chose the Chem tab's layout that way. For a colour, show swatches in the terminal's own palette, over what the thing sits on: in slice 10b the owner chose Train mode's light magenta from the Cursor drawn over grass in Windows Terminal's colours, where the placeholder light green all but vanished. They may dismiss the form to answer in their own words, as they did for the Cursor's lore in slice 10: take the words as the answer, and ask follow-ups in plain text.
- **A choice with real trade-offs comes with its pros and cons,** each option's, in plain words, before the question. In slice 8 the owner had to ask for them before choosing how to break the thorn trap.
- **When measuring shows a design won't work,** stop and bring the numbers, what was tried and the options, rather than retuning quietly. Slice 8's decisions to drop no-op Approaches and to shorten the trace were made that way (design v15, changes 9–10).
- **When something they asked for can't be done as written,** say so before building, with options, rather than quietly doing something else. If an agreed detail changes during the work, say so in the PR.
- **When they raise a broad idea, lay out what it could mean before narrowing it.** In slice 9 "the simulated fly brain" was first read as one mechanism, a concept pool, and measured. The owner had meant the brain as a whole, so the idea had to be revisited: first say which readings there are, and ask.
- **`/grill-with-docs`** is the owner's name for the `grilling` skill together with `domain-modeling`: questions in rounds, with each settled term written into `CONTEXT.md` as it's decided, and the decisions in the design, with a file in `docs/design/changes/`.
- **Other agents may work in the same folder.** The owner runs tools such as Antigravity alongside a session, so untracked files you didn't make are theirs: leave them unstaged and untouched, and pull before pushing in case they committed to the branch.
  - **`cargo fmt --all` would reformat their untracked Rust files:** run `rustfmt` on your own files, and `cargo fmt --all --check` to check. `rustfmt.toml` gives `rustfmt` the 2024 edition; without it, it sorted imports the old way and CI failed (#167).

## The owner's design principles

- **The engine enforces physics only.** Behaviour and safety policies belong in each object's data rules, and the data names things. For example, the cause of death "hurt by thornbush" comes from the object type, not from a list in the code.
  - **The one exception is content safety.** A rule the project must never break, whatever a genome or data pack says, is enforced in the engine, because data can be edited and genes evolve. For now there is one: attraction and mating attempts only ever between adults ([#69](https://github.com/Keazra/terra-sprites/issues/69), for M2).
- **Solid means impassable, whatever a sprite has learned.**
- **Each thing we add is its own self-contained thing.** What it is, what it does and what it's called live in its own entry, not spread across lists in the code or other files. So its plural sits beside its name in `objects.ron` (design v17), rather than in a word list of its own.
  - **The aim is easy modding,** as in the Creatures games, whose fun included how simple it was to make new species and new items. When a design choice comes up, say whether it makes adding a species or an item easier or harder.
- **The player has to care first.** Sprites start unnamed; the player names the ones they care about.
- **Think diegetically.** When something the player uses or sees comes up, ask what it *is* inside the terrarium, and whether sprites can perceive it. In slice 10 the "hand" became **the Cursor**: the player is an advanced creature to the sprites, and the Cursor the hard-light projection they cast into the terrarium, which explains its 3×3 grid. Whether sprites can see it became a design question of its own ([#60](https://github.com/Keazra/terra-sprites/issues/60)). Its name, its look and what it does follow from that.
- **Terra Sprites is a stepping stone** to a larger game: one adventurer in a world of NPCs that run on the Sprite system, with skills a player can lock and a character that can live on as an NPC ([#47](https://github.com/Keazra/terra-sprites/issues/47)). Nothing is built for it yet, but keep `terra-sim` a general engine, and when a design choice comes up, say whether it helps or hinders reusing sprites as NPCs.
- **Grown sprites fend for themselves; the young survive by luck or guidance.** A grown sprite should find its own food and water. An unattended newborn should die before growing up unless it stumbles across them, and survive only about 20% of the time: it should be tough to survive without guidance from the player or other sprites. M1 can't meet that yet, since scarce water kills grown sprites too while nothing lets them remember places ([#40](https://github.com/Keazra/terra-sprites/issues/40)), so A4 is a safety floor and the 20% goal waits ([#122](https://github.com/Keazra/terra-sprites/issues/122), design v37 changes 7–8).
- **Drives have a pecking order.** Hunger and thirst are first-order, powerful drives; play, company and fighting are second-order. A higher tier quiets or amplifies a lower one but never blocks it, and the more pressing a drive, the more it quiets: a sprite dying of thirst would take "a mountain of poor choices" to do anything else, while a little thirst matters little. Slice 10 built the first step, hunger and thirst quieting what a sprite merely likes (design v21 change 13); the fuller hierarchy is [#50](https://github.com/Keazra/terra-sprites/issues/50).
- **A mode never seems to change another mode's state.** Each cursor mode owns its controls, and Follow, which was called the lock until [#92](https://github.com/Keazra/terra-sprites/issues/92), has a key of its own, `F`, outside the modes. In slice 11a the lock first waited whenever the Cursor held anything in Grab mode, and in the build that looked as if Grab mode locked and unlocked. The owner's rule replaced it: Follow steps aside only while the Cursor leads the followed sprite, and otherwise means the same thing everywhere (design v23 change 6).
- **Realism settles behaviour questions.** When two behaviours both work, the owner asks which is more realistic. Answer with how real animals behave, then recommend. In slice 9c, fear was kept quiet in a cornered sprite's choice because cornered animals fight.
- **Sprites are blank slates.** They should learn what's good or bad, pass on their genes, and later teach the next generation, so instincts stay general and make mistakes rather than holding knowledge a sprite should earn. "Bored → play with whatever it's looking at", thornbushes included, stays, even when it costs lives until learning arrives (design v12, change 11).
  - **Not competent from the start** (design v16): the starter genome is very basic. Needs lead to actions, and sprites know their own kind and fight or flee, but they know nothing of food, water, toys or danger. They're a species dropped into a world they don't know, meant to evolve fast; the gene types can still express smarter instincts, so evolution can grow them.
- **Real science is inspiration, not a blueprint.** The owner cares how a real system works, such as the fruit-fly brain behind design v16, abstracted for sprites, which are visual, social and tactile. A copy of its parts isn't the goal.

## Design documents

- The spec is the two designs in `docs/design/`: M1's (`m1-a-sprite-lives.md`) is the rulebook for the game as built, and M2's (`m2-generations.md`) plans milestone 2 and holds what its slices decide.
- **A change edits the design in place and adds a file to `docs/design/changes/`,** named by the date it was settled (`2026-10-08-version-flag.md`), with a table per design it changes: what changed, where the decision came from, and which sections it touches. That folder's README has the template. Until the PR merges, both may be amended freely; after that, a later change is a new file. A rule it adds is marked with its issue, "(#160)".
  - **Why not numbered revisions:** until 2026-10-08 each change was a new copy, `-vN`, with its table at the top. Two slices at once both took the next number, and the second to merge had to renumber and redo its links, at least five times (#101, #108, #117, #120, #152). Numbering stopped at M1 v40 and M2 v5.
  - **The numbered history stays:** M1's tables in `docs/design/m1-changelog.md`, row numbers unchanged ("v16 change 16" is row 16 of the v16 section), with the full text of each as tag `design/m1-vN`; M2's v1 to v5 in `docs/design/archive/`, each with its table at the top. M1's v1 reviews are in `docs/design/reviews/`. Old citations such as "design v37 change 7" still point there.
- The designs keep their names, so the README's links and issues' links to them don't go stale.
- New or changed terms go in `CONTEXT.md` (the `domain-modeling` skill); code and comments use its words, not the ones on its _Avoid_ lists.

## The slice loop

Each slice is a GitHub issue ("Slice N: …") in its milestone; M2's are Slices 20–30. Starting work and finishing it are in `AGENTS.md`; the slice itself goes:

1. Grill the slice's open design questions (`grilling`), in plain words.
2. Record the decisions in the design, a file in `docs/design/changes/`, and `CONTEXT.md`.
3. Agree the seams (the public interfaces tests go through) and the list of behaviours.
4. Build red-green with `tdd`, one behaviour at a time, committing as it goes.
5. Run `two-axis-review` against `main` (Standards and Spec, in parallel sub-agents), with `docs/agents/review-checklist.md`: the slips outside reviews kept finding. Fix every finding that holds up, and give reasons for the ones declined.
6. Open the PR from the template ("Closes #N"), with `scripts/check` passing and its "Try it on your PC" steps filled in.
7. Answer outside reviews (below), then merge when the owner says so: a merge commit, not a squash, then delete the branch and pull `main`. PR descriptions and review replies cite the slice's commits by hash, and a squash would drop them from `main`'s history.

A slice too big for one PR ships as two ("4a", "4b"); only the last PR's description says "Closes #N".

**A slice that promises no visible change proves it.** Run the lab scenarios with the same seeds on `main` and on the branch, and compare the reports byte for byte. Slice 9d, which moved the categories into data, was checked that way: A1 and the thorn trap came out identical.
  - **Start `main`'s reports early.** The thorn trap alone takes about ten minutes in release. In slice 11a they ran in the background from a separate checkout of `main` (`git worktree add --detach <scratch dir> main`), with a `CARGO_TARGET_DIR` of their own so builds didn't wait on each other. The baseline was ready long before the review, and the branch's reports ran the same way, twice: once when the world's work was done, and again after the review's fixes.

**A slice that reworks the code and changes behaviour ships the rework first,** proved identical that way, and the behaviour change after it, measured on its own. Then a number that moves can only be the change's doing. Slice 9e shipped as 9e-a (learning per object type, identical reports) and 9e-b (thornbushes join the bushes).

**A bar that counts side effects needs a baseline that shares the conditions.** Slice 10's bar for late pets was "no odd lessons": lessons the trained sprite learns and its untrained control doesn't. But a trained sprite lives a different life, so even pets on time scored 3; the fair comparison for lateness was pets on time. **And measure the harm case, not only the working one:** late pets at several look-backs all looked alike, and only shocks at random moments showed a long look-back teaching "water is bad" and killing sprites of thirst (design v21 change 6).

**When a decision waits on a measurement, agree the bar before measuring.** Take the baseline on `main`, and settle with the owner, in numbers, what counts as passing, so the result can't choose its own bar. The bar for moving thornbushes into the bushes was set that way (#77): no worse than `main`, allowing 10% for noise. **Say how each number is counted,** too: the lab prints each verb's median, and #77's baseline of 60 thornbush touches was three of them added, where the median of each seed's total was 61. Slice 9e-b had to check its bar both ways.

**A prototype's totals can hide a particular case,** so the existing behaviour tests are the check while building on its numbers. In slice 9c the default world's numbers looked fine, but the old cornered-sprite test showed fear stopping a cornered sprite from turning on its attacker (4 times in 10, against 8 or more). When a fix comes out of such a case, measure it in the prototype again before bringing it to the owner.

**Test a timing rule with the sequence that reaches it in play.** In slice 10b, two bugs in the status marks' flashes passed their tests. The tests fed the world's report without the click that comes before it in play, or used a miss on empty ground, whose `?` starts at once, where a refusal from the world waits its turn. Gemini found both; the tests that caught them follow click, report and next click with real gaps between them.

**Test every input that reaches it: the keyboard's `Q` and `E` as well as clicks.** In slice 11a every Grab test moved the mouse before clicking, so the tests never saw a bug where `Q` acted on the Cursor's old tile: taking hold of the locked-on sprite left the Cursor on it until the mouse moved. The Spec review found it; the tests that catch it press `Q` with the pointer left where it was.

**A test that passes the first time it runs hasn't shown it can fail.** When a test is written for behaviour already built, break the code it guards for a moment and watch the test go red, then put the code back. In slice 11b, the test that a crash's pain teaches no habit along the trace passed with the rule switched off: its sprite was scripted, and **a scripted sprite decides nothing itself, so it commits no trace entries.** A test of learning along the trace needs a sprite that chooses, such as one with an instinct to approach things, which tries nothing on them. The rewritten test went red without the rule, as did the test of slides moving after rolling items with the two swapped.

## Baseline runs

Each time `main` moves, a baseline run (design v24 §7.6) measures what's too slow for CI, and the observer, Gemini, writes a briefing on it. Both are in `docs/reports/`, which git ignores, on the owner's machine: `<date>-<commit>-report.md` and `<date>-<commit>-briefing.md`, with `baseline.log`. A local session reads the newest briefing when it starts (`AGENTS.md`). `scripts/baseline.ps1` runs one, by hand or from the task that `scripts/schedule-baseline.ps1` sets up.

- **The numbers are `main`'s baseline.** When the newest report's commit is `main`'s, its viability run, thorn trap and A1–A3 numbers (seeds 1–10) are what a slice compares itself with, without measuring `main` again.
- **The observer's guesses are leads.** It reads the code but can't run it, so check a suggested cause before acting on it.

## Outside reviews

The owner runs Gemini, and sometimes Codex, on each PR. They post Gemini's findings as inline PR threads headed "[Review from Gemini]", with a summary review.

- Check each claim against the code before acting on it, and fix what holds up, test-first.
- Reply on every thread, saying what changed (with the commit) or why not, then resolve it. With `gh`: GraphQL `addPullRequestReviewThreadReply` (pass the body with `-F body=@file`), then `resolveReviewThread`.
- Update the PR description to match.

## Cloud sessions

- **Update Rust before running the checks:** `rustup update stable`. The container's toolchain can be older than the stable CI uses, and an older clippy flags lints CI doesn't: in slice 9e-b, 1.94 flagged `nonminimal_bool` in `inspector.rs`, on `main` too.
- **The session names its own branch** (`claude/…`). The project's is `feat/slice-N-<slug>`, so ask the owner which to use before the first push.
- **Try `gh` before saying something can't be done.** The GitHub tools cover most work: a review thread gets a reply to its comment, then is resolved by its thread ID. What they can't do, such as making a milestone, `gh api` may: in the M2 planning session it worked through the proxy, where the session had first told the owner it couldn't be done.
- **A branch on GitHub can't be deleted from a session:** the git proxy refuses with a 403. The owner deletes a merged branch from its PR page.

## Triage

Issues are labelled `afk` (an agent can do it alone) or `hitl` (it needs the owner), plus `needs-triage` and `needs-info`; see `docs/agents/triage-labels.md`. Untriaged issues go through `/triage`. Design ideas parked outside M1 are `hitl` issues.
