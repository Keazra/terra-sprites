# Stacked pull requests

A **stack** is a chain of PRs where each builds on the one below. The bottom PR's base is `main`, and each PR above takes the branch of the PR below as its base, so its diff shows only its own commits. Both repos use GitHub's own stacks, which the merge box understands: it merges a stack from the bottom up, and moves the next PR onto `main` after each merge. Stack 214 (Slice 32's #211 and #213) and stack 249 (#245 and #247) are two.

The rules for every PR, stacked or not, are in `AGENTS.md`, "Pull requests".

## When to stack

Stack a PR when it needs the one below: it builds on that PR's code, or changes the same lines. The owner may ask for a stack too. Otherwise branch from `main`: a PR in a stack waits for every PR below it, so an independent change on top of one is held up for nothing.

A PR from `main` that touches files another open PR changes: fetch their branch and check the two merge cleanly with `git merge-tree --write-tree HEAD origin/<their branch>`, which prints any conflicts. If they conflict, stack it, or say in both PRs which merges first.

## Making one

1. **One branch per PR.** A cloud session is given one branch. Name each further branch after it, with a suffix for what it holds (`claude/project-thread-b4kcvw-renderer`), and branch it from the PR below.
2. **Open the PR with the branch below as its base.** Its description starts with a line naming the PR it waits on: "Stacked on #245: merge that first."
3. **Join the PRs in a GitHub stack,** bottom first, so the merge box knows the order. Stacks are made through GitHub's REST API, which `gh api` reaches from a cloud session too:

   ```sh
   echo '{"pull_requests": [245, 247]}' > stack.json
   gh api -X POST repos/Keazra/terra-sprites/stacks --input stack.json
   ```

   Each PR's base must be the branch of the one before it. To put a PR on top of a stack that already exists, or if the call is refused, ask the owner to do it on GitHub.

Put every PR of a chain in its GitHub stack. A PR stacked by its base alone on a stack's branch, as #251 was on #247, has that branch rewritten under it whenever GitHub rebases the stack, and its diff then shows the commits below it too.

## Keeping it current

- **Fetch before you push.** When GitHub rebases a stack (the merge box's "Rebase stack" button), it rewrites the stack's branches with new hashes: #247's was rewritten forty seconds after its stack was made. When `git fetch` reports a forced update to your branch (`+ 1a2b3c4...5d6e7f8 (forced update)`), move your unpushed commits onto GitHub's with `git rebase --onto origin/<branch> 1a2b3c4`, and update any hashes the PR description cites.
- **New commits on a lower branch** go up by merging: merge each branch into the one above it, from the bottom up, and push them all.
- **When `main` moves,** merge it into the bottom branch and then up the stack the same way, and run `scripts/check` on each branch. Merges keep a GitHub stack in order without rewriting anything: Slice 32's stack merged after ec3906c and b3d486b had brought `main` in that way.

## Handing it over

The owner merges. When the stack is ready, the message handing it over lists the merges as numbered steps, bottom first, with what to check between them, and each PR's plain-language comment repeats them:

1. Merge #245.
2. Check that #247 now says it merges into `main`, then merge it.

A GitHub stack can also merge all at once, as one merge commit, as Slice 32's did. Recommend that when every PR in it is ready, unless the owner wants the lower PR on `main` sooner.

**After a lower PR merges on its own,** check the next PR's base reads `main` before it's merged. GitHub deletes a merged PR's branch in both repos, and then moves any PR based on that branch onto the merged PR's base; a GitHub stack moves the next PR onto `main` itself. A PR still based on a merged branch would merge into that branch instead of `main`, so change its base (`update_pull_request` with `base: main`, or `gh api -X PATCH repos/Keazra/terra-sprites/pulls/<n> -f base=main`), merge `main` in, and run the checks.

## Across the two repos

A game PR and a private-docs PR that belong together (#251 and Keazra/terra-sprites-docs#2) can't share a stack: each repo stacks its own. Name the other PR in the opening line of each one's description, and give one merge order across both repos.

## Trying a stack

`scripts/try-pr.ps1` on the top PR builds the whole stack, since its branch holds every PR below it. A lower PR's "Try it on your PC" tries that PR alone.
