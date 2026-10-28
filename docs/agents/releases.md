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

Later milestones carry on the same way. When the game becomes 1.0 is the owner's call. A patch number is a free update between milestones, such as v0.2.1, which shipped the new look, or a fix to a release that's already out, such as v0.2.2 would be (both below).

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
   - Say in the same places that players without Rust can download the game ready-made from the release's page instead, with a link to it (`https://github.com/Keazra/terra-sprites/releases/tag/v0.2.0`), which works once step 3 has published it.
   - The title screen shows the version in its corner, so the opening scene's clip changes with it. Render the README's clip again with the manual's command (`cargo run --release -p terra-tui --example render -- title -o docs/clips/title.gif`), and give the renderer's test `the_same_clip_comes_out_the_same_on_every_computer` (`crates/terra-tui/examples/render/main.rs`) the new file's two hashes, which its failure prints.
2. **The owner merges it, then runs the downloads workflow for the release at once,** since the README now names it. Sessions can't make tags or releases: the git proxy refuses their tags, and GitHub refuses their releases ("not permitted for this session type", 2026-10-18). A workflow can, so the downloads workflow (`.github/workflows/downloads.yml`) makes the release, as a draft. With `gh`, from the terra-sprites folder in PowerShell:

   ```powershell
   gh workflow run downloads.yml -f tag=v0.2.0
   ```

   Or on GitHub: Actions, "Downloads", "Run workflow", with the tag. It builds the newest commit on `main`, which is the version PR's merge as long as nothing else has merged since. It builds only the commit it runs on, so code from someone else's copy of the repo never runs with the release's permissions. To build an older commit, push its tag first and run the workflow from the tag (on GitHub, the tag under "Use workflow from"):

   ```powershell
   git tag v0.2.0 <the version PR's merge commit>; git push origin v0.2.0
   gh workflow run downloads.yml --ref v0.2.0 -f tag=v0.2.0
   ```

   It builds the game for Windows, Mac (Apple silicon) and Linux, in about 3 minutes, and makes a draft release for the tag holding one zip for each: the program, both licences, and a `README.txt` on running it (`.github/downloads/`), with a Mac's first-run steps for a program from an unknown developer. The draft takes its title from the notes file's first line and its description from the rest. The run's summary links to it.

   A draft release made by hand on GitHub beforehand works too, when it names the commit being built or its branch: the workflow adds the zips to it. Running it again replaces the zips. It refuses a release that's already published, a tag already on GitHub on another commit, and a tag that doesn't match the version in `Cargo.toml`.

   To try the workflow without a release, give it a test tag: the game's version with a label after a hyphen, such as `v0.1.0-test.1` while `Cargo.toml` says 0.1.0. A tag with a hyphen may differ from the version after the hyphen only, and then the run just warns. Delete the draft it made afterwards. A draft never made a tag, so nothing else is left behind.
3. **Check the draft, then publish it.** On GitHub, under Releases, the draft should hold the three zips, `terra-sprites-v0.2.0-windows-x86_64.zip`, `…-mac-apple-silicon.zip` and `…-linux-x86_64.zip`. Open the one for your computer, and check its `terra-sprites --version` names the release. Then "Edit" and "Publish release", or:

   ```powershell
   gh release edit v0.2.0 --draft=false
   ```

   Publishing makes the tag, on the commit the zips were built from. The workflow never publishes: until the owner does, only those who can push to the repo can see the draft.
4. **Check it installs, and uninstalls cleanly:** the `cargo install` line above, on the owner's PC or in a cloud session, then `terra-sprites --version`; then the notes' uninstall steps leave neither the `terra-sprites` command nor the game's folder behind. On the owner's PC, the "Removing it" steps in the `README.txt` of their system's zip should leave nothing behind too.

### The notes

```markdown
# v0.2.0: Generations

One or two sentences: what this milestone is about, for someone who has never played.

## What's new

- A bullet per thing a player would notice, newest milestone first. No gene names or type names.

## Known problems

- Anything a player will hit that's left for later, with its issue.

## Install

Download the zip for your computer from this page's Assets, below: Windows, Mac (Apple silicon) or Linux. Each has a `README.txt` that says how to run it, and on a Mac how to open a program from a developer Apple doesn't know, the first time. Or, with Rust 1.97.1 or newer:

    cargo install --git https://github.com/Keazra/terra-sprites --tag v0.2.0 --locked terra-tui

Either way, it's best in a terminal of 100×30 or more (on Windows, Windows Terminal).

## Uninstall

Delete the folder you unzipped, the zip if you kept it, and on Linux `terra-sprites` from the folder on your PATH if you moved it there. Or, if you installed it with Rust, run `cargo uninstall terra-tui`. Your worlds and settings stay in the game's own folder, so a newer version finds them, until you delete that too:

- Windows (PowerShell): `Remove-Item -Recurse "$env:APPDATA\terra-sprites"`
- Mac: `rm -rf ~/Library/Application\ Support/terra-sprites`
- Linux: `rm -rf ~/.local/share/terra-sprites`, or `$XDG_DATA_HOME/terra-sprites` if you've set that

The game writes nothing anywhere else. A Rust install also leaves the code it downloaded in Cargo's folder: the manual's [Uninstalling](https://github.com/Keazra/terra-sprites/blob/v0.2.0/docs/manual.md#uninstalling) says how to clear it.
```

Every release's notes end with these steps for a full, clean uninstall (the owner, 2026-10-26), and so does each download's `README.txt` (`.github/downloads/`). If a release makes the game write anywhere new, or install something else, add it to both, and to the manual's "Uninstalling".

## A free update between milestones

Finished work that isn't a milestone can reach players before the next milestone does, as a patch release made from `main`: v0.2.1 shipped the new look, Looks 1 to 6 ([#301](https://github.com/Keazra/terra-sprites/issues/301)), between M2 and M3 (the owner, 2026-10-19, in the Looks plan). It's made just as in "Making one", from the version PR on, with its own notes, and the README's clips drawn again when the update changes how the game looks.

It works only while `main` holds nothing of the next milestone's code, so it's made after the update's last PR merges and before the next milestone's first slice does. Once that slice is in, a release from `main` would carry its half-built pieces, so later work of the same kind waits for the milestone's release: Looks 7 to 9 ship with v0.3.0.

## A fix to a release

Only for a serious problem players hit in a release (a crash, a lost save, a world that won't load), and only once `main` has moved on to the next milestone; before that, the fix merges to `main` like any other and the release waits for it.

1. The fix merges to `main` first, as usual.
2. A branch `release/0.2`, made from the newest v0.2 tag, takes the same change in a PR of its own, with the version raised a patch number, to 0.2.2 after v0.2.1 (and the title clip with it, as in "Making one"), and a short `docs/releases/v0.2.2.md`.
3. The owner merges it and makes the release `v0.2.2` on that branch: the downloads workflow, as in "Making one", with the tag `v0.2.2`, run from `release/0.2` (`--ref release/0.2`, or that branch under "Use workflow from"). A PR to `main` then points the README's and the manual's install commands and release-page links at `v0.2.2`, and carries the notes file across.

## The first release

M1 shipped before there were releases. Its release, v0.1.0, went on M1's acceptance merge, `fc2ce3f` (PR #127, as it is after the history rewrite of 2026-10-05), whose `Cargo.toml` already said 0.1.0. Its notes are [`docs/releases/v0.1.0.md`](../releases/v0.1.0.md). The README installed `main` until v0.2.0, since M2 was most of what the README and the clips showed.
