# Help for the screen you're on, and a command palette

The key hints, the help and a new command palette are built from one list of controls, each a key or keys and what they do. Fauxpaw settled it in the UI lessons round of 2026-10-19 (question 3, answer A), as Look 2 ([#303](https://github.com/Keazra/terra-sprites/issues/303)) of the Look slices ([#301](https://github.com/Keazra/terra-sprites/issues/301)), and it was built in [#384](https://github.com/Keazra/terra-sprites/pull/384).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **One list of controls makes the key hints, the help and the command palette,** so they can't drift apart: each control's keys, what it does, its help headings, its hint, and what it needs to do anything. The title screen's menu, a box's ` esc close ` corner and the sprite list's ` esc clear ` come from it too. | #303 | §6.1 |
| 2 | **A hint whose key would do nothing just now is greyed** rather than hidden, its key muted like its word: `. step` while time runs, `↑↓ choose` with nothing to choose, and in a replay the keys it refuses, such as `X train`. The top bar's `? help` loses its bold while `?` would type or answer a question. | #303 | §6.1 |
| 3 | **The help screen has five pages,** named in its top edge: World, every key and the legends as before, with `:` added and the files' keys moved to the middle column; Inspector, its keys, the pointer's and the open tab's legend; and a page each for the sprite list, the population graph and the family tree. A page too long to fit scrolls. | #303 | §6.1 |
| 4 | **`?` opens help at the page for where the player is:** the open overlay's, the inspector's once the player last pointed at it or used its keys, otherwise the world's. `←` `→` and `Tab` `Shift+Tab` turn the pages, and closing it goes back to what was open. | #303 | §6.1 |
| 5 | **Typing in help searches every page,** each match under its page's name; `Esc` clears the search before it closes the help, the hints and the corner saying `esc clear` meanwhile. Keys type rather than act while help is open, where before they did nothing. | #303 | §6.1, §6.5 |
| 6 | **`:` opens a command palette** near the top of the map view: typing finds any control that works in the world by the starts of the words of what it does, each with its key, and `Enter` does the one chosen. The pointer's controls are listed to teach their keys, which it can't press. | #303 | §6.1, §6.5 |
| 7 | **The hints and help write keys in lower case,** `esc` and `tab`, as the hints did, and `+/-` for the speed keys. | #303 | §6.1 |
| 8 | **A menu has hints of its own,** `↑↓ choose  enter pick  esc close`, where the world's showed, and `?` opens help over it. Every screen's hints drop whole from the end when short of room, as the world's do. | #303 | §6.1, §6.5 |
