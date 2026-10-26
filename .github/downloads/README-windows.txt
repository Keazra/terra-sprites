Terra Sprites
=============

A terminal terrarium of little creatures that live, learn and breed on
their own.

This is the game for Windows (64-bit). It runs in a terminal window, and
it's made for Windows Terminal, which comes with Windows 11. On Windows
10, get Windows Terminal from the Microsoft Store first.


Running it
----------

1. Unzip it first, if you haven't: right-click the zip, then Extract All.

2. Open this folder, right-click an empty spot in it, and choose Open in
   Terminal. Then type this and press Enter:

       .\terra-sprites.exe

   On Windows 11 you can also just double-click terra-sprites.exe, which
   opens it in Windows Terminal.

3. If Windows says "Windows protected your PC", click More info, then
   Run anyway. It says that about programs it hasn't seen much of, and
   this one isn't signed. On a PC with Smart App Control turned on, which
   blocks every program that isn't signed, there's no Run anyway, and
   this download can't run there yet.

The window needs to be at least 100 characters wide and 30 lines tall,
and 140 by 40 is better. If the game says "Terminal too small", make the
window bigger or maximise it.


Playing
-------

On the title screen, pick New world. Press ? for every key, and the
manual has the rest:
https://github.com/Keazra/terra-sprites/blob/main/docs/manual.md

.\terra-sprites.exe --seed 7 goes straight into seed 7's world.

.\terra-sprites.exe --version says which version this is and which
build. Put that in any bug report.


Your worlds
-----------

The game keeps your saves, autosaves and replays in a folder of its own,
%APPDATA%\terra-sprites (the last line of the help screen, ?, shows it
too). A newer version finds them there, so to update, unzip the new one
and run that instead.


Removing it
-----------

1. Delete this folder, and the zip you downloaded if you kept it. That
   removes the game.

2. To remove your worlds too, delete the game's own folder. Type
   %APPDATA% in File Explorer's address bar, press Enter, and delete the
   terra-sprites folder there. Or in PowerShell:

       Remove-Item -Recurse "$env:APPDATA\terra-sprites"

That's everything: the game writes nothing anywhere else.


Licence
-------

Free, under your choice of the MIT licence (LICENSE-MIT) or the Apache
License 2.0 (LICENSE-APACHE).
