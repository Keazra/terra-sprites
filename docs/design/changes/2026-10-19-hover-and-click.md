# What the pointer is over, and a click on a log line

Whatever the pointer is over says what it is on the status line, and a click on a line of the event log selects its sprite. Fauxpaw settled it in the UI lessons round of 2026-10-19 (question 4), as Look 3 ([#304](https://github.com/Keazra/terra-sprites/issues/304)) of the Look slices ([#301](https://github.com/Keazra/terra-sprites/issues/301)). The tabs' clicks came in Look 1 ([#302](https://github.com/Keazra/terra-sprites/issues/302)).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The inspector, the event log and each pop-up note their spots as they're drawn:** the tabs, the drives' bars, the chemicals, the genes and the memory headings, the log's filters and its lines, and each pop-up's box. The list is made afresh each frame, so what the pointer is over and what a click lands on can't drift from the layout, and a pop-up takes the pointer before what's under it: a click or a turn of the wheel on the debug overlay's box no longer lands on the map beneath. The tabs', the memory headings' and the filters' clicks go through it, where each worked out its own place before. | #304 | §6.1, §6.8 |
| 2 | **The status line says what the pointer is over** in place of the tile, the mode and what follows them: a tab's name, what it shows and its keys; what the log's filters do; a drive's or a chemical's name and what it is; what a gene does; and which sprite a log line is about. Too long for the line, it ends `...`. | #304 | §6.1 |
| 3 | **What each chemical is goes in `chemicals.ron`,** beside its name, as `about`: for a drive, what raises it and what lowers it in the starter genes. A chemical whose pack says nothing says what sort of chemical it is. | #304 | §6.1 |
| 4 | **A gene says what it does in a sentence of its kind,** worked out from the gene, as `An emitter: hunger rises while energy is below .72`, and a trait's part of the line the traits share says what that trait is. These are the engine's, not the data's: genes change as they're handed down, and their kinds and the traits are the engine's physics, with no data file of their own. A trait's words say only what the engine makes it mean, since what it costs is the data's. | #304 | §6.1 |
| 5 | **A click on a line of the event log selects the sprite it's about,** the first it names, while it's in the world, as `Tab` does, and leaves Follow as it is. A line about a sprite no longer living takes no click. While the pointer is on the line, the map lights the sprite up in reverse video, as it does a selected one, unless a pop-up is over it. | #304 | §6.1, §6.5 |
| 6 | **The help lists both:** pointing, under the pointer's controls, and a click on the log, under sprites. | #304 | §6.1 |
