# Fast-forward and the speed cap

Slice 27 ([#145](https://github.com/Keazra/terra-sprites/issues/145), with [#121](https://github.com/Keazra/terra-sprites/issues/121); [PR #207](https://github.com/Keazra/terra-sprites/pull/207)): Story mode tops out at 4× and Lab mode has every speed, picked in New world and kept for the world's life; a Lab world can be fast-forwarded with no screen, then opened in the game, with a population table and a genes table beside its save. Settled in a round of seven questions on 2026-10-14: the owner chose both for Q1, Lab mode and an end-game unlock in Story mode; asked for holding `?` to explain a mode (Q2); and kept the other recommendations.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Story mode and Lab mode:** New world gets a Mode row, Story first, `←` `→` picking, with what each means in a few words beside it. The world keeps its mode for its whole life, in world state, saves and replays; a save or replay from before is a Story world, and the save format stays 1. `--lab` makes a new world a Lab world from the command line, and the top bar says `lab` after a Lab world's speed. In M2 nothing in the simulation reads the mode. | Q2 (A); M4 design §2.2, §11 | §8.3, §8.5, §16.1 |
| 2 | **Holding `?` in New world explains the mode picked,** in the presets' place, until it's let go of (or, where the terminal never says so, the next other key). The inventory's line comes with M4, which brings the inventory. | The owner, Q2 (2026-10-14) | §16.1 |
| 3 | **Story mode's speed stops at 4×,** pressed or held, saying "4x is Story mode's fastest", on the map, with the population graph open and in a replay; loading a Story world slows a faster clock to 4×. Lab mode goes on to Max. | Q1 (C), Q2 (A) | §2, §16.2 |
| 4 | **The training aids still fit the cap:** a pet's reach back and a step while paused scale with the speed, so at 4× they're 10 and 5 ticks, as before. | #121's open question | §16.2 |
| 5 | **The end-game unlock:** once the entire ship is reactivated, a Story world may lift its cap if the player accepts. Built with M4. | The owner, Q1 (2026-10-14): "an end-game unlock in story mode, when the entire ship is reactivated completely and the user accepts it" | §16.2 |
| 6 | **Fast-forward is a command,** `--fast-forward <hours>`, hours of play at 1×, on a new Lab world or a Lab save (`--load`); a Story save is refused. It prints ten rows, a progress line between them and a closing line, saves under a name no save has (`seed-7-200h`, `--save` for another), and `Ctrl+C` stops it early and saves what has run. `--load <save>` alone opens a save in the game. | Q3 (A), Q4 (A), Q5 (A), Q6 | §8.5, §16.3 |
| 7 | **Its tables:** the population table, a row for each of the world's counts (§14.1) and the end, with deaths by cause; the genes table, the starter's gene values and their middle among the living each hour; and the values that moved most, at the end of the report. The gene-value code moves from the generations run into the library, with the run's report unchanged. | Q6 (A) | §14.8, §16.4 |
| 8 | **How we know it worked,** and what was measured: the cap, the same world and it opens, by tests; fast-forward costs under 1% over stepping the same world, though a world 20 hours old steps a quarter slower than the speed check's, so bar 4 isn't met as written, and the owner counted it met; the generations report byte for byte as on `main`; seeds 1–10 for 220 hours for information. | Q7 | §16.5 |
| 9 | **Later:** a key that blanks the map in a Lab world, runs measured in generations, and the uncapped population (#205) run with fast-forward. | Q3 (B), Q4 (B), #205 | §16.6 |

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The top bar says `lab`,** `+` stops at 4× in a Story world, and the flags gain `--lab`, `--load`, `--fast-forward` and `--save`, each pointing to M2 design §16. | #145 | §6.1, §6.6, §6.7 |

## M4 "Terra"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The end-game unlock:** once the entire ship is reactivated, the game offers to lift Story mode's speed cap, and if the player accepts, that world runs at every speed from then on, staying a Story world otherwise. When exactly the offer comes is settled with the Bridge. | The owner, Slice 27's round, Q1 (2026-10-14) | §2.2, §2.3, §11 |
