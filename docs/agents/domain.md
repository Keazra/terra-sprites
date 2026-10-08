# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **The design docs** in `docs/design/`: `m1-a-sprite-lives.md` and `m2-generations.md`. Read the sections that touch the area you're about to work in. What changed when is in `docs/design/changes/`, whose README says where the earlier numbered revisions are: history, not spec. The v1 reviews are in `docs/design/reviews/`.
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
│   └── design/         ← one design doc per milestone, edited in place
│       ├── changes/    ← one file per change, by date
│       ├── archive/    ← M2's numbered revisions v1, v3 and v4
│       └── reviews/    ← the v1 evaluations
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

A design change edits the design in place and adds a file to `docs/design/changes/` with its change table (see that folder's README).
