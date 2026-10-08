# Design changes

Each change to a design is a file here, from the PR that makes it. The designs themselves keep one name each, [`m1-a-sprite-lives.md`](../m1-a-sprite-lives.md) and [`m2-generations.md`](../m2-generations.md), and are edited in place.

Until 2026-10-08 every change was a new numbered revision, a whole copy of the design. Two slices built at once both took the next number, and the second to merge had to renumber and redo its links: it happened at least five times. The numbering stopped at M1 v40 and M2 v5.

## A change file

- **Its name** is the date the change was settled and a few words, as `2026-10-08-version-flag.md`, so the files list in order and two PRs never pick the same name.
- **It holds a table per design it changes,** as the revisions did: what changed, where the decision came from (the issue, or the owner's words and the date), and which sections it touches.
- **Until the PR merges,** it and the design may be amended freely. After that, a later change is a new file.
- **A rule it adds is marked with its issue** in the design, as "(#160)", or with the date when there's no issue, where it used to be marked with a revision, "(v40)".

Code comments cite the design by section, `design §6.7`, as before.

## Before 2026-10-08

- **M1:** the tables of v1 to v40 are in [`m1-changelog.md`](../m1-changelog.md), and the full text of each is tag `design/m1-vN`.
- **M2:** v1 to v5 are in [`archive/`](../archive/), each with its table at the top.

Git keeps every version of each design since.

## Template

```markdown
# <What changed, in a few words>

<One or two sentences: what was settled, with whom, and when. Link the issue and the PR.>

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **<The change in bold,>** then the detail. | #N | §x.y |
```

Leave out a design the change doesn't touch.
