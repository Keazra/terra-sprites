Terra Sprites
=============

A terminal terrarium of little creatures that live, learn and breed on
their own.

This is the game for Linux on a 64-bit Intel or AMD PC, with glibc 2.35
or newer: Ubuntu 22.04, Debian 12, Fedora 36 or later, and most others
from 2022 on. It runs in any terminal.


Running it
----------

Unzip it, then in a terminal, in this folder:

    chmod +x terra-sprites
    ./terra-sprites

chmod +x makes it executable, in case what unzipped it didn't keep that.
To run it from anywhere as terra-sprites, move it to a folder on your
PATH, such as ~/.local/bin.

The terminal needs to be at least 100 characters wide and 30 lines tall,
and 140 by 40 is better. If the game says "Terminal too small", make the
window bigger.


Playing
-------

On the title screen, pick New world. Then:

- Click a sprite (the smiley faces) to select it, and press ] for its
  Brain tab: what it's paying attention to, and why.
- Space pauses and + speeds up. Lives are long at 1x, so try 4x.
- ? shows every key.

./terra-sprites --seed 7 goes straight into seed 7's world. The manual
has every key and flag:
https://github.com/Keazra/terra-sprites/blob/main/docs/manual.md

./terra-sprites --version says which version this is and which build.
Put that in any bug report.


Your worlds
-----------

The game keeps your saves, autosaves and replays in a folder of its own,
~/.local/share/terra-sprites, or $XDG_DATA_HOME/terra-sprites if you've
set that (the last line of the help screen, ?, shows it too). A newer
version finds them there, so to update, unzip the new one and run that
instead. To remove the game, delete this folder, and that one too if you
don't want your worlds.


Licence
-------

Free, under your choice of the MIT licence (LICENSE-MIT) or the Apache
License 2.0 (LICENSE-APACHE).
