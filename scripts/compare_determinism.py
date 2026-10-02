"""Compares the determinism check's runs across CI's runners (design §7.5).

    python3 scripts/compare_determinism.py <folder>

The folder holds one subfolder per runner, as CI downloads them: each named
`determinism-<runner>`, with the check's output in `hashes.txt` and the
target it ran on in `target.txt`. Every runner must have printed exactly the
same text. The comparison is printed as a table, and also written to the
file `GITHUB_STEP_SUMMARY` names, if set, for the run's summary page.

Exits with 1 when the runs differ, naming the first checkpoint where they
part, and with 2 when a run is unreadable, there are fewer than two, or no
run has a checkpoint to compare.
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

    def hashes(self):
        """Its state hash at each checkpoint, by tick."""
        found = (checkpoint_in(line) for line in self.lines)
        return {checkpoint.tick: checkpoint.hash for checkpoint in found if checkpoint}


class Checkpoint(NamedTuple):
    tick: int
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
    if len(words) >= 4 and words[0] == "tick" and words[1].isdigit() and words[2] == "hash":
        return Checkpoint(int(words[1]), words[3])
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

    # Rows go by tick, not by line, so a runner that printed a line more or
    # less still lines up with the rest.
    hashes = [run.hashes() for run in runs]
    out.append("| Tick | " + " | ".join(run.runner for run in runs) + " | Match |")
    out.append("|---:|" + "---|" * len(runs) + ":---:|")
    for tick in sorted(set().union(*hashes)):
        row = [by_tick.get(tick) for by_tick in hashes]
        cells = [f"`{hash}`" if hash else "(none)" for hash in row]
        mark = "✓" if len(set(row)) == 1 else "✗"
        out.append(f"| {tick} | " + " | ".join(cells) + f" | {mark} |")
    return "\n".join(out) + "\n"


def main():
    if len(sys.argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    try:
        runs = read_runs(sys.argv[1])
    except OSError as error:
        return fail(f"Can't read a run: {error}")
    if len(runs) < 2:
        finished = ", ".join(run.runner for run in runs) or "none"
        return fail(f"Too few runners finished to compare: {finished}. It needs at least two.")
    if not any(run.hashes() for run in runs):
        return fail("No runner printed a checkpoint, so there was nothing to compare.")
    differs_at = first_difference(runs)
    text = report(runs, differs_at)
    sys.stdout.buffer.write(text.encode("utf-8"))
    summarise(text)
    return 0 if differs_at is None else 1


def fail(message):
    """Says why nothing could be compared, here and in the run's summary."""
    print(message, file=sys.stderr)
    summarise(f"**Not compared.** {message}\n")
    return 2


def summarise(text):
    """Adds `text` to the run's summary page, when there is one."""
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as file:
            file.write("## Cross-platform determinism\n\n" + text)


if __name__ == "__main__":
    sys.exit(main())
