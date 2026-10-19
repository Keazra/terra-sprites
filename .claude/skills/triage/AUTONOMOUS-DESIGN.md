# Autonomous design

This project's addition to triage (the owner, 2026-10-19). Most design issues need the owner: their rounds of questions are where the game's direction is set. A few are small enough for an agent to settle alone, from the owner's principles and the decisions already made, with the owner checking the result. Those are **design candidates**, and the `ready-for-agent-design` role marks them.

## The tests

A candidate passes all four. For an issue that fails, note the first test it fails, in a few words.

1. **Answered.** Every open question in it already has an answer: in the owner's design principles (`docs/agents/how-we-work.md`), the designs, `CONTEXT.md`, or a decision recorded on an issue. A question of the owner's taste fails it: lore and names, how something feels to play, a layout or colour they'd pick from mockups, a number only a long run can settle.
2. **Free.** No milestone's rounds, no slice and no running thread owns it, and nothing it waits on is still open. Check the newest issue sweep if there is one, the open PRs, and the project's running threads.
3. **Contained.** It changes no save's shape, and moves nothing that a slice or an acceptance check is measuring now. The screen, tooling and docs pass easily; a change to how sprites live passes only while nothing it touches is being measured.
4. **Cloud-sized.** Checking it needs no long run on the owner's PC (`docs/agents/how-we-work.md`, "Cloud sessions").

## Ranking

Order the candidates by how much each helps the work coming next (the current milestone, its playtests, the next milestone's design), then by how little it touches. Present them as a table: rank, issue, what it is in plain words, its depth (below), and why now. Under it, count the issues each test ruled out, and keep the full list, with each issue's failed test, in a file the owner can open.

## How far the designer goes

The depth follows the playtest rank the change would get (`docs/agents/how-we-work.md`):

- **Minor (no playtest): design and build.** One draft PR designs and builds it, lists every design call at the top of its description, and closes the issue.
- **Medium or critical: design, then the owner's OK.** A draft PR changes the design where the area's rules live, adds its file in `docs/design/changes/` and any new terms to `CONTEXT.md`, and builds nothing. Once the owner accepts it, the issue moves to `ready-for-agent` for the build.

The designing thread is titled "AUTONOMOUS: <name>". It runs the slice loop's grilling step on itself: each question, its answer, and the principle or decision the answer rests on go into the design change's file, so the owner reads the reasoning with the result. When a question turns out to need the owner after all, the thread stops, asks it on the issue in plain words, and moves the issue back to `ready-for-human`.

## Design brief

Posted when an issue moves to `ready-for-agent-design`, in place of an agent brief. It follows the principles in [AGENT-BRIEF.md](AGENT-BRIEF.md) (durable, behavioural, testable, scoped) and carries each open question with triage's answer, so the designing thread starts from the owner's rules rather than a guess.

```markdown
## Design Brief

**Category:** enhancement
**Summary:** one-line description of what needs designing
**Depth:** design and build (minor) / design, then the owner's OK (medium or critical)

**Current behavior:**
What the game does now, in plain words.

**The questions, with triage's answers:**
1. A question, in plain words.
   **Answer:** what triage recommends.
   **Rests on:** the principle, design section or issue decision that settles it.

**Acceptance criteria:**
- [ ] Specific, testable criterion

**Out of scope:**
- What the design leaves alone

**Back to the owner if:**
- What would turn a question into one of taste after all
```
