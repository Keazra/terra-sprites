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

`handover` is this project's own skill, not a copy.

## Changes from the originals

- **`code-review` is `two-axis-review`.** A project skill named `code-review` would replace Claude Code's built-in `/code-review`, including `/code-review ultra`. `tdd` and `implement` name it by its new name.
- **Line endings** are LF, as the rest of the repo.

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
