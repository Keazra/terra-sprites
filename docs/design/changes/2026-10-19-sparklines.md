# Sparklines beside drives and chemicals

The inspector's Body and Chem tabs gain a sparkline at the end of each drive's and chemical's line, eight shades showing how the level moved over about the last 50 ticks: the owner settled the shades, where they go and that the screen keeps them on 2026-10-17, when they filed [#272](https://github.com/Keazra/terra-sprites/issues/272), and an autonomous design thread ([AUTONOMOUS-DESIGN.md](../../../.claude/skills/triage/AUTONOMOUS-DESIGN.md)) settled the rest on 2026-10-19 for their OK, from their principles, triage's design brief on the issue and a measurement. [#328](https://github.com/Keazra/terra-sprites/issues/328) builds it, after Looks 1 and 6 ([#302](https://github.com/Keazra/terra-sprites/issues/302), [#307](https://github.com/Keazra/terra-sprites/issues/307)) and the inspector's split into files ([#286](https://github.com/Keazra/terra-sprites/issues/286)).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A sparkline at the end of each drive's line on the Body tab and each chemical's line on the Chem tab, from #328:** eight shades, `░▒▓█`, over about the last 50 ticks, oldest on the left. | The owner, 2026-10-17 (#272) | §6.1 |
| 2 | **Each shade is the average of a six-tick stretch,** the stretches going by the tick count the status line shows (counts 6 to 11 make one), so the eight cover the last 43 to 48 ticks; the rightmost is the stretch still running. | Q2, Q3 | §6.1 |
| 3 | **A row shows its own ups and downs, but is never stretched past a tenth,** one cell of the bar: a row that hardly moves keeps its level's shade, and the bar and the number say how high it is. | Q1, measured | §6.1 |
| 4 | **A stretch not yet seen is blank,** so a row fills from the right. | Q6 | §6.1 |
| 5 | **Every chemical on a line of its own has one,** the named hormones too, but reward and punishment don't, nor the hormone grid or the Body tab's physical levels. | Q5 | §6.1 |
| 6 | **It starts in the 33rd column on both tabs,** so the rows line up and nothing on a line moves: 40 of the 44 columns. | Q4 | §6.1 |
| 7 | **`░▒▓█` in every theme,** as the bar is; a drive's is drawn in its bar's colour on both tabs. | Q7 | §6.1 |
| 8 | **The screen keeps it, for every living sprite, after every tick it runs,** a replay's included and fast-forward's not; never saved, and not in `state_hash`. Past 4×, in Lab mode, it turns over faster than it can be read. | The owner, 2026-10-17 (#272), Q8 | §6.1, §6.8 |

## The questions, and what settled each

Triage's design brief on #272 asked five questions and gave an answer to each. The thread asked itself three more. Each answer says what it rests on.

1. **What scale does a row use: 0 to 1, as the bar does, or its own range?**
   **Answer:** its own range, never narrower than a tenth. Its lowest average is `░` and its highest `█`, but averages within a tenth of each other are drawn as if they spanned a tenth, placed as the level sits between 0 and 1, so a row that hardly moves keeps the shade of its level and a small drift stays a step or two.
   **Rests on:** the purpose in the owner's note, to see "whether hunger is spiking or settling after a meal", and the row they settled, `░░▒▒▓▓██`, a climb filling it. Triage's brief had answered 0 to 1, so that a quiet drive wouldn't look stormy. Measured, 0 to 1 draws a still row nearly all the time, and a third of meals don't show at all (below); the tenth keeps a quiet drive quiet, which was the brief's reason. This is the main departure from the brief; questions 4 and 5 make two small ones.
2. **What does a shade show for its stretch: the average, or the peak?**
   **Answer:** the average.
   **Rests on:** the same purpose: an average shows a level climbing or easing without flicker, and a meal's sudden drop as a step down, where a peak would hide the drop for a whole stretch.
3. **How do the stretches line up with the ticks, and is it 50 ticks?**
   **Answer:** six ticks each, going by the tick count the status line shows once each tick has run (the levels at counts 6 to 11 make one stretch, 12 to 17 the next, and so on), so eight cover the last 43 to 48. The newest is the stretch still running, so a meal shows in it at once, and the row steps one place left as each stretch begins: every 4.8 seconds at 1×, 1.2 at 4×.
   **Rests on:** the owner's "8 cells covering the last 50 ticks", as near as whole stretches of the same length come. Stretches fixed to the tick count keep a finished stretch's average from changing, and show the same row for the same moment, in a replay too.
4. **Does it fit?**
   **Answer:** yes. A drive's line on the Body tab runs to the 30th column inside the inspector at most (`crowdedness` and a level of `1.00`), so the sparkline starts in the 33rd, two after it, and runs to the 40th of 44 (45 once Look 1 takes the frames away). The Chem tab's lines end sooner, so it starts in the same column there and the rows line up across both tabs. Nothing already on a line moves; a name too long for its column, from a data pack, pushes it right.
   **Rests on:** the screen as it's drawn at 100×30 (`crates/terra-tui/tests/snapshots/`) and the owner's round. Triage's brief asked for a space; two keep the shades apart from an arrow.
5. **Which lines on the Chem tab get one?**
   **Answer:** every chemical on a line of its own: the physical chemicals, the drives and the named hormones, such as the feminiser. Not reward and punishment, and not the hormone grid.
   **Rests on:** the owner's round ("each named chemical", and the grid has no room), and the Chem tab's own rule that reward and punishment are used up every tick, so their levels always read 0 between ticks (§6.1): their rows would be flat for good. What a sprite felt, the reward line's `felt`, is signed, which a row of four shades from low to high can't show. Reward and punishment are named chemicals, so this narrows the owner's "each named chemical" by two.
6. **What does a sprite show before 48 ticks have been seen, such as just after a load?**
   **Answer:** a blank for each stretch not yet seen, so a row fills from the right: a hatchling's, and every sprite's once a world is made or loaded.
   **Rests on:** the owner's round: the screen keeps it, never a save.
7. **What do other looks draw?**
   **Answer:** `░▒▓█` in every theme, as the bar beside it is, and whatever Look 6's glyph sets give the bars. A drive's sparkline is drawn in its bar's colour on both tabs, once Look 1 gives bars their drive's colour, and any other in its line's. The shades carry the meaning, so it reads the same without colour and to a colour-blind player.
   **Rests on:** §6.2, which keeps text in CP437 in every theme; Look 1's rule that colour never carries meaning alone (#302); Look 6 (#307).
8. **Which ticks does the screen take in?**
   **Answer:** every tick the game runs with a screen, for every living sprite, so selecting a sprite shows its past at once, playing a replay included. Fast-forward runs with no screen, so a world it saved opens with blank rows. A sprite's rows go when it dies. The open tab's policy shows or hides them with the rest of the tab (§6.4).
   **Rests on:** the owner's round ("kept for every sprite by the screen, not the world"), and design §6.8, where the screen's own state lives.

**Back to the owner if:** they'd rather a row read as a level than as a shape (question 1): then it's the 0-to-1 rows of the comparison below, and nothing else here changes. Or they'd want reward and punishment to have one after all (question 5), or another colour than the bar's (question 7).

## What was measured

**What was tested, and why.** Whether a row on the scale of 0 to 1 would show anything, since drives change slowly: hunger climbs by about a tenth in 48 ticks at the most, in a sprite out of energy. The default world ran for seeds 1, 2 and 3 from tick 5,000 to 30,000, about 4.2 million sprite-ticks, and every ten ticks each sprite's row for each chemical was drawn on three scales: 0 to 1, its own range with no least, and its own range with a tenth as the least. The game was `main` at ce9cfc7, for this and the screens below; the small tool that drew the rows is kept with the owner's working documents.

**What happened.** On 0 to 1, a drive's row showed any change at all, two shades or more, in at most one look in ten, anxiety's apart (28%), since most drives move by less than a fortieth in 48 ticks. And about a third of the meals and drinks that dropped hunger or thirst by .2 or more never showed as a fall, because they began below a quarter. With each row's own range and no least, rows that had moved by less than .02 looked stormy, with three or more shades, in up to 97% of looks (boredom). With a tenth as the least, none did, and every meal and drink showed as a fall twelve ticks later. A row's shades change between steps in up to 4% of ticks, when the running stretch sets a new high or low; on 0 to 1, never.

| Drive | Shows a change: 0 to 1 | Shows a change: own range, a tenth at least | Moved under .02 but stormy: own range, no least | The same, a tenth at least |
|---|---|---|---|---|
| hunger | 2% | 16% | 64% | 0% |
| thirst | 4% | 36% | 80% | 0% |
| pain | 1% | 3% | 8% | 0% |
| tiredness | 7% | 43% | 44% | 0% |
| boredom | 1% | 15% | 97% | 0% |
| loneliness | 0% | 7% | 70% | 0% |
| crowdedness | 0% | 3% | 7% | 0% |
| arousal | 10% | 19% | 2% | 0% |
| anxiety | 28% | 46% | 44% | 0% |

| A fall of .2 or more in a tick | How many | Shown falling 12 ticks later: 0 to 1 | Own range, a tenth at least |
|---|---|---|---|
| hunger (a meal) | 1,143 | 68% | 100% |
| thirst (a drink) | 2,979 | 69% | 100% |

The physical chemicals drift more steadily: energy's row shows a change in 6% of looks on 0 to 1 and 54% with a tenth at least, stamina's 11% and 80%.

## The tabs at 100×30

Drawn by the game itself from the default world, seed 7, at tick 6,048, with the sparklines put in as this design draws them. Sprite #536 is old, ate 14 ticks ago, filling its gut with food, and was hurt a little before that; the water in its gut is being taken in, so hydration climbs, and its anxiety is rising.

The Body tab, in the default theme:

```
 Terra Sprites │ tick 6,048 │ ► 1x │ seed 7 │ sprites 41 │ not saved                         ? help
┌─ Map ──────────────────────────────────────────────┐┌─ #536 ── [Body] Brain Chem Genome World ───┐
│ *  *      ∙║≈≈≈≈≈≈≈≈≈≈~~::::::....................•││ Wandering off · 5 tiles to go              │
│         ∙  ║≈≈≈≈≈≈≈≈≈≈~~::::.......................││ age 13,070 · old                           │
│            ║≈≈≈≈≈≈≈≈≈≈~~:::........................││ speed 6.44 · sense 9.61                    │
│       ·    ║≈≈≈≈≈≈≈≈≈≈~~~:..•......................││ male                                       │
│   *       ·║≈≈≈≈≈≈≈≈≈~~~~:.♣'.....................•││ one of the first sprites · generation 1    │
│    *     · ║≈≈≈≈≈≈≈≈~~~~~:♪.•..........#####...?...││ partner Sprite #551                        │
│       ∙    ║~~~~~~~~~~~~~:,......############......││                                            │
│            ║~~~~~~~~~~~~:,,....###############.....││ hunger      █░░░░░░░░░ .09 ▲   ████▓░░░    │
│           ·║~~~~~~~~~::::,♪....################....││ thirst      █░░░░░░░░░ .07 ▲   ░░░░▒▒▒▒    │
│            ║:::::::::::,,♪.....################....││ pain        █░░░░░░░░░ .12 ▼   █████▓░░    │
│           ·║::........♥,,,.....################....││ tiredness   ░░░░░░░░░░ .05 ▲   ░░░░░░░░    │
│            ║..........,•♣.......###############....││ boredom     ░░░░░░░░░░ .01 ▲   ░░░░░░░░    │
│            ║......X♣♠•.•........###############....││ loneliness  ░░░░░░░░░░ .00     ░░░░░░░░    │
│     ·      ║.......•♣...........###############....││ crowdedness ░░░░░░░░░░ .00     ░░░░░░░░    │
│   ∙        ║•.•••..••..........###############.....││ arousal     ░░░░░░░░░░ .00     ░░░░░░░░    │
│ ·    ·     ║•♣:♣•::::::........##############......││ anxiety     ███░░░░░░░ .30 ▲   ░░░░░▒██    │
│   ·      · ║•••••~~~~~::.......##############....♠.││                                            │
│         ∙ ∙║::~~~~~~~~~~:.....♠..........##........││ energy .01 · hydration .32 · stamina .37   │
│  ∙·      · ║:~~~~≈≈≈≈~~~~:....................♥....││ food .58 · water .26 · injury .07          │
│        *∙  ║~~~~≈≈≈≈≈≈~~~~......................•••││ wear .30 · genes .95                       │
│·           ║~~~≈≈≈≈≈≈≈≈≈~~:......................♣'││                                            │
└────────────────────────────────────────────────────┘└────────────────────────────────────────────┘
┌─ Events ────────────────────────────────────────────────────────────────── [all] selected major ─┐
│   6,047  Sprite #533 played with Sprite #550                                                     │
│   6,047  Sprite #537 played with Sprite #544 x2                                                  │
│   6,046  Sprite #533 played with Sprite #550                                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 (128,80) dirt · berry (fresh) │ SELECT                                        Tracking Sprite #536
```

The Chem tab, in the default theme. The hormone grid and the named hormones below it are further down, as the tab scrolls at 100×30 today:

```
 Terra Sprites │ tick 6,048 │ ► 1x │ seed 7 │ sprites 41 │ not saved                         ? help
┌─ Map ──────────────────────────────────────────────┐┌─ #536 ── Body Brain [Chem] Genome World ───┐
│ *  *      ∙║≈≈≈≈≈≈≈≈≈≈~~::::::....................•││ energy        .01  +.0007      ░░░░░░░░    │
│         ∙  ║≈≈≈≈≈≈≈≈≈≈~~::::.......................││ hydration     .32  +.0017      ░░▒▒▒▓▓█    │
│            ║≈≈≈≈≈≈≈≈≈≈~~:::........................││ stamina       .37  -.0020      ▒▒▒▒▒▒▒▒    │
│       ·    ║≈≈≈≈≈≈≈≈≈≈~~~:..•......................││ food          .58  -.0010      ░░░░▓███    │
│   *       ·║≈≈≈≈≈≈≈≈≈~~~~:.♣'.....................•││ water         .26  -.0020      █▓▓▒▒▒░░    │
│    *     · ║≈≈≈≈≈≈≈≈~~~~~:♪.•..........#####...?...││ injury        .07  -.0001      ░░░░▒▒▒▒    │
│       ∙    ║~~~~~~~~~~~~~:,......############......││                                            │
│            ║~~~~~~~~~~~~:,,....###############.....││ hunger        .09  +.0015      ████▓░░░    │
│           ·║~~~~~~~~~::::,♪....################....││ thirst        .07  +.0008      ░░░░▒▒▒▒    │
│            ║:::::::::::,,♪.....################....││ pain          .12  -.0046      █████▓░░    │
│           ·║::........♥,,,.....################....││ tiredness     .05  +.0004      ░░░░░░░░    │
│            ║..........,•♣.......###############....││ boredom       .01  +.0002      ░░░░░░░░    │
│            ║......X♣♠•.•........###############....││ loneliness    .00              ░░░░░░░░    │
│     ·      ║.......•♣...........###############....││ crowdedness   .00              ░░░░░░░░    │
│   ∙        ║•.•••..••..........###############.....││ arousal       .00              ░░░░░░░░    │
│ ·    ·     ║•♣:♣•::::::........##############......││ anxiety       .30  +.0188      ░░░░░▒██    │
│   ·      · ║•••••~~~~~::.......##############....♠.││ reward        .00  felt +.00               │
│         ∙ ∙║::~~~~~~~~~~:.....♠..........##........││ punishment    .00                          │
│  ∙·      · ║:~~~~≈≈≈≈~~~~:....................♥....││ HORMONES                                   │
│        *∙  ║~~~~≈≈≈≈≈≈~~~~......................•••││ h0   .26   h1   .00   h2   .00   h3   .00  │
│·           ║~~~≈≈≈≈≈≈≈≈≈~~:......................♣'││ h4   .00   h5   .00   h6   .00   h7   .00  │
└────────────────────────────────────────────────────┘└────────────────────────────────────────────┘
┌─ Events ────────────────────────────────────────────────────────────────── [all] selected major ─┐
│   6,047  Sprite #533 played with Sprite #550                                                     │
│   6,047  Sprite #537 played with Sprite #544 x2                                                  │
│   6,046  Sprite #533 played with Sprite #550                                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 (128,80) dirt · berry (fresh) │ SELECT                                        Tracking Sprite #536
```

The same two in the ASCII theme, which changes only the map, as for the bars:

```
 Terra Sprites │ tick 6,048 │ ► 1x │ seed 7 │ sprites 41 │ not saved                         ? help
┌─ Map ──────────────────────────────────────────────┐┌─ #536 ── [Body] Brain Chem Genome World ───┐
│ *  *      +|==========~~::::::....................%││ Wandering off · 5 tiles to go              │
│         +  |==========~~::::.......................││ age 13,070 · old                           │
│            |==========~~:::........................││ speed 6.44 · sense 9.61                    │
│       .    |==========~~~:..%......................││ male                                       │
│   *       .|=========~~~~:.&'.....................%││ one of the first sprites · generation 1    │
│    *     . |========~~~~~:".%..........#####...?...││ partner Sprite #551                        │
│       +    |~~~~~~~~~~~~~:,......############......││                                            │
│            |~~~~~~~~~~~~:,,....###############.....││ hunger      █░░░░░░░░░ .09 ▲   ████▓░░░    │
│           .|~~~~~~~~~::::,"....################....││ thirst      █░░░░░░░░░ .07 ▲   ░░░░▒▒▒▒    │
│            |:::::::::::,,".....################....││ pain        █░░░░░░░░░ .12 ▼   █████▓░░    │
│           .|::........+,,,.....################....││ tiredness   ░░░░░░░░░░ .05 ▲   ░░░░░░░░    │
│            |..........,%&.......###############....││ boredom     ░░░░░░░░░░ .01 ▲   ░░░░░░░░    │
│            |......X&*%.%........###############....││ loneliness  ░░░░░░░░░░ .00     ░░░░░░░░    │
│     .      |.......%&...........###############....││ crowdedness ░░░░░░░░░░ .00     ░░░░░░░░    │
│   +        |%.%%%..%%..........###############.....││ arousal     ░░░░░░░░░░ .00     ░░░░░░░░    │
│ .    .     |%&:&%::::::........##############......││ anxiety     ███░░░░░░░ .30 ▲   ░░░░░▒██    │
│   .      . |%%%%%~~~~~::.......##############....*.││                                            │
│         + +|::~~~~~~~~~~:.....*..........##........││ energy .01 · hydration .32 · stamina .37   │
│  +.      . |:~~~~====~~~~:....................+....││ food .58 · water .26 · injury .07          │
│        *+  |~~~~======~~~~......................%%%││ wear .30 · genes .95                       │
│.           |~~~=========~~:......................&'││                                            │
└────────────────────────────────────────────────────┘└────────────────────────────────────────────┘
┌─ Events ────────────────────────────────────────────────────────────────── [all] selected major ─┐
│   6,047  Sprite #533 played with Sprite #550                                                     │
│   6,047  Sprite #537 played with Sprite #544 x2                                                  │
│   6,046  Sprite #533 played with Sprite #550                                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 (128,80) dirt · berry (fresh) │ SELECT                                        Tracking Sprite #536
```

```
 Terra Sprites │ tick 6,048 │ ► 1x │ seed 7 │ sprites 41 │ not saved                         ? help
┌─ Map ──────────────────────────────────────────────┐┌─ #536 ── Body Brain [Chem] Genome World ───┐
│ *  *      +|==========~~::::::....................%││ energy        .01  +.0007      ░░░░░░░░    │
│         +  |==========~~::::.......................││ hydration     .32  +.0017      ░░▒▒▒▓▓█    │
│            |==========~~:::........................││ stamina       .37  -.0020      ▒▒▒▒▒▒▒▒    │
│       .    |==========~~~:..%......................││ food          .58  -.0010      ░░░░▓███    │
│   *       .|=========~~~~:.&'.....................%││ water         .26  -.0020      █▓▓▒▒▒░░    │
│    *     . |========~~~~~:".%..........#####...?...││ injury        .07  -.0001      ░░░░▒▒▒▒    │
│       +    |~~~~~~~~~~~~~:,......############......││                                            │
│            |~~~~~~~~~~~~:,,....###############.....││ hunger        .09  +.0015      ████▓░░░    │
│           .|~~~~~~~~~::::,"....################....││ thirst        .07  +.0008      ░░░░▒▒▒▒    │
│            |:::::::::::,,".....################....││ pain          .12  -.0046      █████▓░░    │
│           .|::........+,,,.....################....││ tiredness     .05  +.0004      ░░░░░░░░    │
│            |..........,%&.......###############....││ boredom       .01  +.0002      ░░░░░░░░    │
│            |......X&*%.%........###############....││ loneliness    .00              ░░░░░░░░    │
│     .      |.......%&...........###############....││ crowdedness   .00              ░░░░░░░░    │
│   +        |%.%%%..%%..........###############.....││ arousal       .00              ░░░░░░░░    │
│ .    .     |%&:&%::::::........##############......││ anxiety       .30  +.0188      ░░░░░▒██    │
│   .      . |%%%%%~~~~~::.......##############....*.││ reward        .00  felt +.00               │
│         + +|::~~~~~~~~~~:.....*..........##........││ punishment    .00                          │
│  +.      . |:~~~~====~~~~:....................+....││ HORMONES                                   │
│        *+  |~~~~======~~~~......................%%%││ h0   .26   h1   .00   h2   .00   h3   .00  │
│.           |~~~=========~~:......................&'││ h4   .00   h5   .00   h6   .00   h7   .00  │
└────────────────────────────────────────────────────┘└────────────────────────────────────────────┘
┌─ Events ────────────────────────────────────────────────────────────────── [all] selected major ─┐
│   6,047  Sprite #533 played with Sprite #550                                                     │
│   6,047  Sprite #537 played with Sprite #544 x2                                                  │
│   6,046  Sprite #533 played with Sprite #550                                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 (128,80) dirt · berry (fresh) │ SELECT                                        Tracking Sprite #536
```

### On 0 to 1, for comparison

The same moment's Body tab rows on 0 to 1, as triage's brief had them. The meal still shows, but the hurt easing and the thirst climbing don't, and the anxiety climbing only just:

```
│ hunger      █░░░░░░░░░ .09 ▲   ████▓░░░    │
│ thirst      █░░░░░░░░░ .07 ▲   ░░░░░░░░    │
│ pain        █░░░░░░░░░ .12 ▼   ░░░░░░░░    │
│ tiredness   ░░░░░░░░░░ .05 ▲   ░░░░░░░░    │
│ boredom     ░░░░░░░░░░ .01 ▲   ░░░░░░░░    │
│ loneliness  ░░░░░░░░░░ .00     ░░░░░░░░    │
│ crowdedness ░░░░░░░░░░ .00     ░░░░░░░░    │
│ arousal     ░░░░░░░░░░ .00     ░░░░░░░░    │
│ anxiety     ███░░░░░░░ .30 ▲   ░░░░░░░▒    │
```

and as this design draws them:

```
│ hunger      █░░░░░░░░░ .09 ▲   ████▓░░░    │
│ thirst      █░░░░░░░░░ .07 ▲   ░░░░▒▒▒▒    │
│ pain        █░░░░░░░░░ .12 ▼   █████▓░░    │
│ tiredness   ░░░░░░░░░░ .05 ▲   ░░░░░░░░    │
│ boredom     ░░░░░░░░░░ .01 ▲   ░░░░░░░░    │
│ loneliness  ░░░░░░░░░░ .00     ░░░░░░░░    │
│ crowdedness ░░░░░░░░░░ .00     ░░░░░░░░    │
│ arousal     ░░░░░░░░░░ .00     ░░░░░░░░    │
│ anxiety     ███░░░░░░░ .30 ▲   ░░░░░▒██    │
```
