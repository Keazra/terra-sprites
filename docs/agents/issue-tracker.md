# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations, except in cloud sessions (Claude Code on the web, Projects), where `gh` isn't signed in: there, use the GitHub MCP tools (`mcp__github__*`, such as `issue_read`, `issue_write`, `add_issue_comment` and `list_issues`) for the same operations. The commands below say what to do; the MCP tools take the body as a parameter, so the `--body-file` rule is for `gh` only.

## Conventions

Always pass issue and comment bodies with `--body-file` pointing at a temp file. Never inline `--body` or use heredocs: they break on backticks, quotes and Windows paths.

- **Create an issue**: `gh issue create --title "..." --body-file <tmp>`
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body-file <tmp>`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: comment with `--body-file` first, then `gh issue close <number>`

Infer the repo from `git remote -v`; `gh` does this automatically when run inside a clone.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh api "repos/{owner}/{repo}/pulls?state=open&per_page=100" --jq '.[] | {number, title, body, author: .user.login, author_association, labels: [.labels[].name]}'` then keep only `author_association` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, `FIRST_TIMER`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`). `gh pr list`/`gh pr view` don't expose this field. Fetch comments per PR with `gh pr view <number> --comments`.
- **Comment / label / close**: `gh pr comment <number> --body-file <tmp>`, `gh pr edit <number> --add-label`/`--remove-label`, `gh pr close <number>`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Blocking

A slice issue lists the issues it waits on in a section at the end of its body:

```markdown
## Blocked by

- #<n>
```

It's unblocked when every issue listed there is closed. `AGENTS.md` ("Starting work") picks the next slice this way.
