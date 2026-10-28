Terra Sprites
=============

A terminal terrarium of little creatures that live, learn and breed on
their own.

This is the game for Linux on a 64-bit Intel or AMD PC, with glibc 2.35
or newer: Ubuntu 22.04, Debian 12, Fedora 36 or later, and most others
from 2022 on. It runs in a terminal. If the map's characters look wrong
in yours, ./terra-sprites --ascii draws it in plain ASCII instead.


Running it
----------

Unzip it, then in a terminal, in this folder:

    chmod +x terra-sprites
    ./terra-sprites

chmod +x makes it executable, in case what unzipped it didn't keep that.
To run it from anywhere as terra-sprites, move it to a folder on your
PATH, such as ~/.local/bin.

The game is best in a terminal at least 100 characters wide and 30 lines
tall, and 140 by 40 is better. A smaller one still plays, with less
beside the map: press i to see the inspector. The title screen needs 60
by 20; if the game says "Terminal too small", make the window bigger.


Playing
-------

On the title screen, pick New world. Press ? for every key, and the
manual has the rest:
https://github.com/Keazra/terra-sprites/blob/main/docs/manual.md

./terra-sprites --seed 7 goes straight into seed 7's world.

./terra-sprites --version says which version this is and which build.
Put that in any bug report.


Your worlds
-----------

The game keeps your saves, autosaves, replays and settings in a folder
of its own, ~/.local/share/terra-sprites, or $XDG_DATA_HOME/terra-sprites
if you've set that (the last line of the help screen, ?, shows it too).
A newer version finds them there, so to update, unzip the new one and
run that instead.


Removing it
-----------

1. Delete this folder, the zip if you kept it, and terra-sprites from
   the folder on your PATH if you moved it there
   (rm ~/.local/bin/terra-sprites). That removes the game.

2. To remove your worlds and settings too, delete the game's own folder:

       rm -rf ~/.local/share/terra-sprites

   or $XDG_DATA_HOME/terra-sprites if you've set that.

That's everything: the game writes nothing anywhere else.


Licence
-------

Free, under your choice of the MIT licence (LICENSE-MIT) or the Apache
License 2.0 (LICENSE-APACHE).
