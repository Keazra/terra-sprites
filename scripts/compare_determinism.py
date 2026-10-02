"""Compares the determinism check's runs across CI's runners (design §7.5).

    python3 scripts/compare_determinism.py <folder>

The folder holds one subfolder per runner, as CI downloads them: each named
`determinism-<runner>`, with the check's output in `hashes.txt` and the
target it ran on in `target.txt`. Every runner must have printed exactly the
same text. The comparison is printed as a table, and also written to the
file `GITHUB_STEP_SUMMARY` names, if set, for the run's summary page.

Exits with 1 when the runs differ, naming the first checkpoint where they
part, and with 2 when there are fewer than two runs to compare.
"""

import os
import sys
from pathlib import Path

PREFIX = "determinism-"


def read_runs(folder):
    """Each runner's name, target and output lines, in name order."""
    runs = []
    for sub in sorted(Path(folder).glob(PREFIX + "*")):
        if not sub.is_dir():
            continue
        lines = (sub / "hashes.txt").read_text(encoding="utf-8").splitlines()
        target = (sub / "target.txt").read_text(encoding="utf-8").strip()
        runs.append((sub.name[len(PREFIX):], target, lines))
    return runs


def hash_of(line):
    """A checkpoint line's tick and hash, or None for any other line."""
    words = line.split()
    if len(words) >= 4 and words[0] == "tick" and words[2] == "hash":
        return words[1], words[3]
    return None


def compare(runs):
    """The report as Markdown, and the first line where the runs differ
    (as an index into their output), or None when they all match."""
    first_difference = None
    longest = max(len(lines) for _, _, lines in runs)
    for i in range(longest):
        texts = {lines[i] if i < len(lines) else None for _, _, lines in runs}
        if len(texts) > 1:
            first_difference = i
            break

    out = []
    if first_difference is None:
        out.append(f"**The simulation matched on all {len(runs)} runners.**")
    else:
        out.append("**The simulation differs between runners.** First difference:")
        out.append("")
        for name, _, lines in runs:
            line = lines[first_difference] if first_difference < len(lines) else "(nothing)"
            out.append(f"- {name}: `{line}`")
    out.append("")
    out.append("| Runner | Target |")
    out.append("|---|---|")
    for name, target, _ in runs:
        out.append(f"| {name} | `{target}` |")
    out.append("")

    out.append("| Tick | " + " | ".join(name for name, _, _ in runs) + " | |")
    out.append("|---:|" + "---|" * len(runs) + ":---:|")
    for i in range(longest):
        row = [lines[i] if i < len(lines) else "" for _, _, lines in runs]
        checkpoints = [hash_of(line) for line in row]
        ticks = [checkpoint[0] for checkpoint in checkpoints if checkpoint]
        if not ticks:
            continue
        cells = [f"`{checkpoint[1]}`" if checkpoint else "" for checkpoint in checkpoints]
        same = len(set(row)) == 1
        out.append(f"| {ticks[0]} | " + " | ".join(cells) + f" | {'✓' if same else '✗'} |")
    return "\n".join(out) + "\n", first_difference


def main():
    if len(sys.argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    runs = read_runs(sys.argv[1])
    if len(runs) < 2:
        print(f"found {len(runs)} runs in {sys.argv[1]}; need at least two", file=sys.stderr)
        return 2
    report, first_difference = compare(runs)
    sys.stdout.buffer.write(report.encode("utf-8"))
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as file:
            file.write("## Cross-platform determinism\n\n" + report)
    return 0 if first_difference is None else 1


if __name__ == "__main__":
    sys.exit(main())
