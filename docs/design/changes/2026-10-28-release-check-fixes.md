# What v0.2.1's release check found in the new look

v0.2.1's release check (2026-10-28, before tagging) played the new look and reviewed its code, and found two bugs from Looks 1 and 6 that the design already ruled out or didn't foresee. Both were fixed in the version PR, [#402](https://github.com/Keazra/terra-sprites/pull/402).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **An unnamed sprite whose label is too long for the inspector's title row gives its number alone,** "#108061", as v0.2.0 did, where Look 1 cut its label to "Sprite #10...". Every sprite in a world fast-forwarded past about 17 hours has a number that long. | The release check, 2026-10-28; #302 | §6.1 |
| 2 | **A palette the player picks draws its colours with `NO_COLOR` set,** as §6.2 already said: the game answers `NO_COLOR` itself, by starting in mono, so it tells crossterm, which would drop every colour and with them bold and reverse video, to write them. | The release check, 2026-10-28; #307 | §6.2 |
