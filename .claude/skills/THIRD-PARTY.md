# Skills copied from mattpocock/skills

These skills are copied from [mattpocock/skills](https://github.com/mattpocock/skills), plugin version 1.2.3, so that cloud sessions have them. Cloud sessions don't install plugins, but they load a repo's `.claude/skills/`. The Claude Code plugin is turned off for this project in `.claude/settings.json`, so local sessions use these copies too.

| Skill | From |
|---|---|
| `tdd` | `skills/engineering/tdd` |
| `codebase-design` | `skills/engineering/codebase-design` |
| `two-axis-review` | `skills/engineering/code-review`, renamed |
| `triage` | `skills/engineering/triage` |
| `diagnosing-bugs` | `skills/engineering/diagnosing-bugs` |
| `prototype` | `skills/engineering/prototype` |
| `domain-modeling` | `skills/engineering/domain-modeling` |
| `implement` | `skills/engineering/implement` |
| `grilling` | `skills/productivity/grilling` |

## Changes from the originals

- **`code-review` is `two-axis-review`.** A project skill named `code-review` would replace Claude Code's built-in `/code-review`, including `/code-review ultra`. `tdd` and `implement` name it by its new name.
- **`two-axis-review` reads this project's review checklist,** `docs/agents/review-checklist.md`, as a standards source on every review (step 3).
- **Line endings** are LF, as the rest of the repo.
- **`triage` and `implement` can be started by Claude.** The originals set `disable-model-invocation: true`, which hides a skill from Claude so that only a typed `/triage` starts it. Claude Code Projects and other cloud chats have no slash commands, so the two could never run there. Their descriptions say instead to use them only when the user names them, and `AGENTS.md` says a message starting with a skill's name asks for that skill.
- **No `/setup-matt-pocock-skills`.** `triage` and `two-axis-review` point at `docs/agents/triage-labels.md` and `docs/agents/issue-tracker.md` instead of telling the user to run a setup skill this repo doesn't have.
- **`tdd` proposes the seams and carries on,** since the owner replies only to what they disagree with, rather than writing no test until the user confirms them.
- **No `agents/openai.yaml`.** Those files configure the skills for OpenAI's tools; nothing here reads them.

## Updating

Copy a skill's folder again from a newer version of the plugin, and redo the changes above.

## License

MIT License

Copyright (c) 2026 Matt Pocock

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
