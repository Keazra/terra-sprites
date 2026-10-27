Terra Sprites
=============

A terminal terrarium of little creatures that live, learn and breed on
their own.

This is the game for a Mac with Apple silicon (M1 or newer). It runs in
a terminal window, such as the Terminal app that comes with your Mac.


Running it
----------

1. Double-click the zip to unzip it, if Safari hasn't already.

2. Open Terminal (in Applications, then Utilities), and make its window
   bigger: double-click its title bar. The game is best at 100
   characters by 30 lines or more, and 140 by 40 is better.

3. Type cd and a space, drag this folder onto the Terminal window, and
   press Return. Then type this and press Return:

       ./terra-sprites

The first time, macOS won't open it, since it isn't signed by a
developer Apple knows. It says "terra-sprites" Not Opened, or that it
can't be opened because the developer can't be verified. To allow it:

4. Click Done, or Cancel (not Move to Trash).

5. Open System Settings, then Privacy & Security, and scroll down to
   Security, where it says "terra-sprites" was blocked. Click Allow
   Anyway (or Open Anyway), and enter your password if it asks.

6. Back in Terminal, type ./terra-sprites again and press Return. When
   macOS asks once more, click Open Anyway, or Open.

From then on it opens straight away. (If you're at home in Terminal,
this does the same as steps 4 to 6, from this folder:
xattr -d com.apple.quarantine terra-sprites)

If the game says "Terminal too small", make the window bigger.


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

The game keeps your saves, autosaves and replays in a folder of its own,
~/Library/Application Support/terra-sprites (the last line of the help
screen, ?, shows it too). A newer version finds them there, so to
update, unzip the new one and run that instead.


Removing it
-----------

1. Drag this folder to the Trash, and the zip too if Safari kept it.
   That removes the game.

2. To remove your worlds too, delete the game's own folder. In Finder,
   choose Go, then Go to Folder, type ~/Library/Application Support and
   press Return, then drag the terra-sprites folder there to the Trash.
   Or in Terminal:

       rm -rf ~/Library/Application\ Support/terra-sprites

That's everything: the game writes nothing anywhere else.


Licence
-------

Free, under your choice of the MIT licence (LICENSE-MIT) or the Apache
License 2.0 (LICENSE-APACHE).
