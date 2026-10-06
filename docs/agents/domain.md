# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **The current design doc** in `docs/design/`: it holds only the current revision of each design, the highest-numbered `-vN`. Read the sections that touch the area you're about to work in. M1's earlier revisions are tags `design/m1-vN`, and their change tables are in `docs/design/m1-changelog.md`. The v1 reviews are in `docs/design/reviews/`. M2's earlier revisions are in `docs/design/archive/`. All of that is history, not spec.
- **`CONTEXT.md`** at the repo root: the glossary.
- **`docs/adr/`**: read ADRs that touch the area you're about to work in.

If `CONTEXT.md` or `docs/adr/` don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `domain-modeling` skill creates them lazily when terms or decisions actually get resolved. Until then, the design doc's terms are the vocabulary.

## File structure

This is a single-context repo:

```
/
├── CONTEXT.md          ← glossary (created lazily)
├── docs/
│   ├── adr/            ← decision records, 0001-<slug>.md (created lazily)
│   └── design/         ← the current design doc of each design (the highest -vN)
│       └── archive/    ← earlier revisions and their reviews: history, not spec
├── crates/
└── data/
```

The full code layout, including which crate may depend on what, is in the design doc (§2.1 in M1).

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `CONTEXT.md`, or in the design doc where the glossary doesn't cover it yet. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in either, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR and design conflicts

If your output contradicts an existing ADR or the current design doc, surface it explicitly rather than silently overriding:

> _Contradicts ADR-000N (<title>) / design §N.N, but worth reopening because…_

Design changes go in a new `-vN` revision with a "Changes from vN-1" table, not an edit to the current one. The exception is a revision that hasn't reached `main` yet, which may be amended within its PR.
