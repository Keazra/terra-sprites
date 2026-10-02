"""Compares the determinism check's runs across CI's runners (design §7.5).

    python3 scripts/compare_determinism.py <folder>

The folder holds one subfolder per runner, as CI downloads them: each named
`determinism-<runner>`, with the check's output in `hashes.txt` and the
target it ran on in `target.txt`. Every runner must have printed exactly the
same text. The comparison is printed as a table, and also written to the
file `GITHUB_STEP_SUMMARY` names, if set, for the run's summary page.

Exits with 1 when the runs differ, naming the first checkpoint where they
part, and with 2 when a run is unreadable or there are fewer than two.
"""

import os
import sys
from pathlib import Path
from typing import NamedTuple

PREFIX = "determinism-"


class Run(NamedTuple):
    """What one runner printed."""

    runner: str
    target: str
    lines: list

    def line(self, i):
        """Its output's line `i`, or None past its end."""
        return self.lines[i] if i < len(self.lines) else None


class Checkpoint(NamedTuple):
    tick: str
    hash: str


def read_runs(folder):
    """Each runner's run, in runner name order."""
    runs = []
    for sub in sorted(Path(folder).glob(PREFIX + "*")):
        if not sub.is_dir():
            continue
        lines = (sub / "hashes.txt").read_text(encoding="utf-8").splitlines()
        target = (sub / "target.txt").read_text(encoding="utf-8").strip()
        runs.append(Run(sub.name[len(PREFIX):], target, lines))
    return runs


def checkpoint_in(line):
    """The checkpoint a line of output gives, or None for any other line."""
    words = (line or "").split()
    if len(words) >= 4 and words[0] == "tick" and words[2] == "hash":
        return Checkpoint(words[1], words[3])
    return None


def first_difference(runs):
    """The first line where the runs' outputs differ, as an index into
    them, or None when they all match."""
    longest = max(len(run.lines) for run in runs)
    for i in range(longest):
        if len({run.line(i) for run in runs}) > 1:
            return i
    return None


def report(runs, differs_at):
    """The comparison as Markdown: the verdict, each runner's target, and
    every checkpoint's hash on each runner."""
    out = []
    if differs_at is None:
        out.append(f"**The simulation matched on all {len(runs)} runners.**")
    else:
        out.append("**The simulation differs between runners.** First difference:")
        out.append("")
        for run in runs:
            out.append(f"- {run.runner}: `{run.line(differs_at) or '(nothing)'}`")
    out.append("")
    out.append("| Runner | Target |")
    out.append("|---|---|")
    for run in runs:
        out.append(f"| {run.runner} | `{run.target}` |")
    out.append("")

    out.append("| Tick | " + " | ".join(run.runner for run in runs) + " | |")
    out.append("|---:|" + "---|" * len(runs) + ":---:|")
    for i in range(max(len(run.lines) for run in runs)):
        row = [run.line(i) for run in runs]
        checkpoints = [checkpoint_in(line) for line in row]
        ticks = [checkpoint.tick for checkpoint in checkpoints if checkpoint]
        if not ticks:
            continue
        cells = [f"`{checkpoint.hash}`" if checkpoint else "" for checkpoint in checkpoints]
        mark = "✓" if len(set(row)) == 1 else "✗"
        out.append(f"| {ticks[0]} | " + " | ".join(cells) + f" | {mark} |")
    return "\n".join(out) + "\n"


def main():
    if len(sys.argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    try:
        runs = read_runs(sys.argv[1])
    except OSError as error:
        print(f"can't read a run: {error}", file=sys.stderr)
        return 2
    if len(runs) < 2:
        print(f"found {len(runs)} runs in {sys.argv[1]}; need at least two", file=sys.stderr)
        return 2
    differs_at = first_difference(runs)
    text = report(runs, differs_at)
    sys.stdout.buffer.write(text.encode("utf-8"))
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as file:
            file.write("## Cross-platform determinism\n\n" + text)
    return 0 if differs_at is None else 1


if __name__ == "__main__":
    sys.exit(main())
