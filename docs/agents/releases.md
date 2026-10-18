# Releases

Each finished milestone is a numbered release: a fixed build that players install, while `main` carries on with the next milestone. PRs still go to `main` one at a time, as `AGENTS.md`'s "Pull requests" says. A milestone is never held back from `main` on a branch or a stack of its own; the release is what gives players a finished one (the owner, 2026-10-18).

## The numbers

One minor number per milestone:

| Release | Milestone |
|---|---|
| v0.1.0 | M1, *A Sprite Lives* |
| v0.2.0 | M2, *Generations* |
| v0.3.0 | M3, *Body & verbs* |
| v0.4.0 | M4, *Terra* |

Later milestones carry on the same way. When the game becomes 1.0 is the owner's call. A fix to a release that's already out is a patch release, such as v0.2.1 (below).

## When

At the end of the milestone's acceptance slice (M2's is Slice 30b, [#148](https://github.com/Keazra/terra-sprites/issues/148)), once the owner's play-through has passed. The release is the slice's last step.

## Making one

1. **The version PR.** The acceptance slice's last PR, or a small one of its own titled "Version 0.2.0":
   - Raise `version` in the root `Cargo.toml`'s `[workspace.package]` to the release's number, so `terra-sprites --version` says it, and let `Cargo.lock` follow.
   - Write the release notes, `docs/releases/v0.2.0.md`, from the template below: what's new since the last release, in plain words for players and in the README's casual voice, from the milestone's design changes and PRs.
   - Point the install commands in the README ("Try it") and the manual ("Getting started") at the release, and say in one line that leaving out `--tag` installs the newest work in progress instead:

     ```bash
     cargo install --git https://github.com/Keazra/terra-sprites --tag v0.2.0 --locked terra-tui
     ```

     `--locked` builds with the release's own `Cargo.lock`. Without it, `cargo install` picks the newest versions of every library, so an old release can stop building, or build differently, as libraries move on.
2. **The owner merges it, then makes the release at once,** since the README now names it. Sessions can't: the git proxy refuses their tags, and GitHub refuses their releases ("not permitted for this session type", 2026-10-18). With `gh`, from the terra-sprites folder in PowerShell, on an up-to-date `main`, one command makes the tag and the release together:

   ```powershell
   gh release create v0.2.0 --target <the version PR's merge commit> --title "v0.2.0: Generations" --notes-file docs/releases/v0.2.0.md
   ```

   Or on GitHub: Releases, "Draft a new release", a new tag `v0.2.0` on that commit, the same title, and the notes file's text as the description. When the tag is already pushed, as v0.1.0's was, `--verify-tag` in place of `--target` makes `gh` use it.
3. **Check it installs:** the `cargo install` line above, on the owner's PC or in a cloud session, then `terra-sprites --version`.

### The notes

```markdown
# v0.2.0: Generations

One or two sentences: what this milestone is about, for someone who has never played.

## What's new

- A bullet per thing a player would notice, newest milestone first. No gene names or type names.

## Known problems

- Anything a player will hit that's left for later, with its issue.

## Install

You need Rust 1.97.1 or newer, and a terminal of at least 100×30 (on Windows, Windows Terminal).

    cargo install --git https://github.com/Keazra/terra-sprites --tag v0.2.0 --locked terra-tui
```

## A fix to a release

Only for a serious problem players hit in a release (a crash, a lost save, a world that won't load), and only once `main` has moved on to the next milestone; before that, the fix merges to `main` like any other and the release waits for it.

1. The fix merges to `main` first, as usual.
2. A branch `release/0.2`, made from the tag, takes the same change in a PR of its own, with the version raised to 0.2.1 and a short `docs/releases/v0.2.1.md`.
3. The owner merges it and makes the release `v0.2.1` on that branch. A PR to `main` then points the README's and the manual's install commands at `v0.2.1`, and carries the notes file across.

## The first release

M1 shipped before there were releases. Its release, v0.1.0, goes on M1's acceptance merge, `fc2ce3f` (PR #127, as it is after the history rewrite of 2026-10-05), whose `Cargo.toml` already says 0.1.0. Its notes are [`docs/releases/v0.1.0.md`](../releases/v0.1.0.md). The README keeps installing `main` until v0.2.0 is out, since M2 is most of what the README and the clips show.
