# How we work

The owner's preferences and the project's conventions, for any agent session, local or in the cloud. The session routine itself is the `handover` skill (`.claude/skills/handover/SKILL.md`); the rules for git and GitHub are in `AGENTS.md`.

## Working with the owner

- **They review by feel, after trying the build** in Windows Terminal, and change direction when something feels off. Don't build on a direction they haven't confirmed.
- **Plain words, from the start.** Write questions and options for someone who hasn't read the code: say what the player sees or what changes, not step letters, gene jargon or type names. A question once had to be asked again because it wasn't plain.
  - **Name a place by what it's for, not by its file:** "a word list of its own", not `themes/words.ron`. Slice 9b's question about where plurals should live had to be asked again for that reason.
  - **Results too.** Before any numbers, say in a sentence or two what was tested and why, then what happened, as a story; tables come after. In slice 10 the owner had to ask twice ("in plain language?", "what is it we are testing, again?") before a measurement round made sense to them.
- **They reply only to what they disagree with.** Give a recommendation with each question or list; silence accepts it. Keep going on anything they don't object to.
- **A real decision gets a real question.** Use `AskUserQuestion` with the recommended option first. For anything about layout, give each option an ASCII mockup preview: the owner chose the Chem tab's layout that way. For a colour, show swatches in the terminal's own palette, over what the thing sits on: in slice 10b the owner chose Train mode's light magenta from the Cursor drawn over grass in Windows Terminal's colours, where the placeholder light green all but vanished. They may dismiss the form to answer in their own words, as they did for the Cursor's lore in slice 10: take the words as the answer, and ask follow-ups in plain text.
- **A choice with real trade-offs comes with its pros and cons,** each option's, in plain words, before the question. In slice 8 the owner had to ask for them before choosing how to break the thorn trap.
- **When measuring shows a design won't work,** stop and bring the numbers, what was tried and the options, rather than retuning quietly. Slice 8's decisions to drop no-op Approaches and to shorten the trace were made that way (design v15, changes 9–10).
- **When something they asked for can't be done as written,** say so before building, with options, rather than quietly doing something else. If an agreed detail changes during the work, say so in the PR.
- **When they raise a broad idea, lay out what it could mean before narrowing it.** In slice 9 "the simulated fly brain" was first read as one mechanism, a concept pool, and measured. The owner had meant the brain as a whole, so the idea had to be revisited: first say which readings there are, and ask.
- **`/grill-with-docs`** is the owner's name for the `grilling` skill together with `domain-modeling`: questions in rounds, with each settled term written into `CONTEXT.md` as it's decided, and the decisions in a new design revision.
- **Other agents may work in the same folder.** The owner runs tools such as Antigravity alongside a session, so untracked files you didn't make are theirs: leave them unstaged and untouched, and pull before pushing in case they committed to the branch.
  - **Their code builds with ours.** Antigravity's untracked `crates/terra-sim/examples/soak.rs` uses the lab's public API, so changing a shape it reads breaks the local build and tests (CI never sees it). In slice 10, `LabRun.control` kept its shape and gained a separate `without` field instead. And `cargo fmt --all` would reformat their files: run `rustfmt` on your own files, and `cargo fmt --all --check` to check.

## The owner's design principles

- **The engine enforces physics only.** Behaviour and safety policies belong in each object's data rules, and the data names things. For example, the cause of death "hurt by thornbush" comes from the object type, not from a list in the code.
  - **The one exception is content safety.** A rule the project must never break, whatever a genome or data pack says, is enforced in the engine, because data can be edited and genes evolve. For now there is one: attraction and mating attempts only ever between adults ([#69](https://github.com/Keazra/terra-sprites/issues/69), for M2).
- **Solid means impassable, whatever a sprite has learned.**
- **Each thing we add is its own self-contained thing.** What it is, what it does and what it's called live in its own entry, not spread across lists in the code or other files. So its plural sits beside its name in `objects.ron` (design v17), rather than in a word list of its own.
  - **The aim is easy modding,** as in the Creatures games, whose fun included how simple it was to make new species and new items. When a design choice comes up, say whether it makes adding a species or an item easier or harder.
- **The player has to care first.** Sprites start unnamed; the player names the ones they care about.
- **Think diegetically.** When something the player uses or sees comes up, ask what it *is* inside the terrarium, and whether sprites can perceive it. In slice 10 the "hand" became **the Cursor**: the player is an advanced creature to the sprites, and the Cursor the hard-light projection they cast into the terrarium, which explains its 3×3 grid. Whether sprites can see it became a design question of its own ([#60](https://github.com/Keazra/terra-sprites/issues/60)). Its name, its look and what it does follow from that.
- **Terra Sprites is a stepping stone** to a larger game: one adventurer in a world of NPCs that run on the Sprite system, with skills a player can lock and a character that can live on as an NPC ([#47](https://github.com/Keazra/terra-sprites/issues/47)). Nothing is built for it yet, but keep `terra-sim` a general engine, and when a design choice comes up, say whether it helps or hinders reusing sprites as NPCs.
- **Drives have a pecking order.** Hunger and thirst are first-order, powerful drives; play, company and fighting are second-order. A higher tier quiets or amplifies a lower one but never blocks it, and the more pressing a drive, the more it quiets: a sprite dying of thirst would take "a mountain of poor choices" to do anything else, while a little thirst matters little. Slice 10 built the first step, hunger and thirst quieting what a sprite merely likes (design v21 change 13); the fuller hierarchy is [#50](https://github.com/Keazra/terra-sprites/issues/50).
- **Realism settles behaviour questions.** When two behaviours both work, the owner asks which is more realistic. Answer with how real animals behave, then recommend. In slice 9c, fear was kept quiet in a cornered sprite's choice because cornered animals fight.
- **Sprites are blank slates.** They should learn what's good or bad, pass on their genes, and later teach the next generation, so instincts stay general and make mistakes rather than holding knowledge a sprite should earn. "Bored → play with whatever it's looking at", thornbushes included, stays, even when it costs lives until learning arrives (design v12, change 11).
  - **Not competent from the start** (design v16): the starter genome is very basic. Needs lead to actions, and sprites know their own kind and fight or flee, but they know nothing of food, water, toys or danger. They're a species dropped into a world they don't know, meant to evolve fast; the gene types can still express smarter instincts, so evolution can grow them.
- **Real science is inspiration, not a blueprint.** The owner cares how a real system works, such as the fruit-fly brain behind design v16, abstracted for sprites, which are visual, social and tactile. A copy of its parts isn't the goal.

## Design documents

- The spec is the highest `docs/design/*-vN.md`. Each revision is a new copy with a "Changes from vN-1" table at the top, saying what changed, where the decision came from, and which sections it touches.
  - **It carries only its own table.** When copying the previous revision, replace its table with the new one; the old table stays at the top of the archived copy. So a citation such as "v16 change 16" is row 16 of the table heading `archive/…-v16.md`. The owner asked for this in slice 10b: the tables of v2–v21 had grown to a third of the spec, all ahead of it. Trimming history changes no design, so it needs no new revision.
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

**A bar that counts side effects needs a baseline that shares the conditions.** Slice 10's bar for late pets was "no odd lessons": lessons the trained sprite learns and its untrained control doesn't. But a trained sprite lives a different life, so even pets on time scored 3; the fair comparison for lateness was pets on time. **And measure the harm case, not only the working one:** late pets at several look-backs all looked alike, and only shocks at random moments showed a long look-back teaching "water is bad" and killing sprites of thirst (design v21 change 6).

**When a decision waits on a measurement, agree the bar before measuring.** Take the baseline on `main`, and settle with the owner, in numbers, what counts as passing, so the result can't choose its own bar. The bar for moving thornbushes into the bushes was set that way (#77): no worse than `main`, allowing 10% for noise. **Say how each number is counted,** too: the lab prints each verb's median, and #77's baseline of 60 thornbush touches was three of them added, where the median of each seed's total was 61. Slice 9e-b had to check its bar both ways.

**A prototype's totals can hide a particular case,** so the existing behaviour tests are the check while building on its numbers. In slice 9c the default world's numbers looked fine, but the old cornered-sprite test showed fear stopping a cornered sprite from turning on its attacker (4 times in 10, against 8 or more). When a fix comes out of such a case, measure it in the prototype again before bringing it to the owner.

**Test a timing rule with the sequence that reaches it in play.** In slice 10b, two bugs in the status marks' flashes passed their tests. The tests fed the world's report without the click that comes before it in play, or used a miss on empty ground, whose `?` starts at once, where a refusal from the world waits its turn. Gemini found both; the tests that caught them follow click, report and next click with real gaps between them.

## Outside reviews

The owner runs Gemini, and sometimes Codex, on each PR. They post Gemini's findings as inline PR threads headed "[Review from Gemini]", with a summary review.

- Check each claim against the code before acting on it, and fix what holds up, test-first.
- Reply on every thread, saying what changed (with the commit) or why not, then resolve it. With `gh`: GraphQL `addPullRequestReviewThreadReply` (pass the body with `-F body=@file`), then `resolveReviewThread`.
- Update the PR description to match.

## Cloud sessions

- **Update Rust before running the checks:** `rustup update stable`. The container's toolchain can be older than the stable CI uses, and an older clippy flags lints CI doesn't: in slice 9e-b, 1.94 flagged `nonminimal_bool` in `inspector.rs`, on `main` too.
- **The session names its own branch** (`claude/…`). The project's is `feat/slice-N-<slug>`, so ask the owner which to use before the first push.
- **There's no `gh`;** the GitHub tools do the same work. A review thread gets a reply to its comment, then is resolved by its thread ID.
- **A branch on GitHub can't be deleted from a session:** the git proxy refuses with a 403. The owner deletes a merged branch from its PR page.

## Triage

Issues are labelled `afk` (an agent can do it alone) or `hitl` (it needs the owner), plus `needs-triage` and `needs-info`; see `docs/agents/triage-labels.md`. Untriaged issues go through `/triage`. Design ideas parked outside M1 are `hitl` issues.
