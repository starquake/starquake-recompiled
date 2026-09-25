STARQUAKE
=========

A reimplementation of Starquake (Stephen Crow, Bubble Bus Software,
1985) for modern computers. It is not an emulator: the game is
rewritten as ordinary Rust.


YOU NEED THE ORIGINAL GAME
--------------------------

This program contains no code or data from the original game. It reads
every graphic, map, piece of text and sound from your own copy of
Starquake, a .tap tape, when it starts, and it will not run without one.

World of Spectrum keeps Spectrum software available and removes titles
whose rights holders object. It lists Starquake as available. A dump of
a tape you own works just as well.

  https://worldofspectrum.net/


RUNNING IT
----------

Run starquake, or starquake.exe on Windows. If it cannot find your
tape it asks for it: pick the file, or drop it onto the window. The zip
World of Spectrum serves (Starquake.tap.zip) works as it is, with no
need to unpack it. The tape is then kept for next time in the usual
place for application data:

  Linux     ~/.local/share/starquake-recompiled/
  macOS     ~/Library/Application Support/starquake-recompiled/
  Windows   %APPDATA%\starquake-recompiled\

It also finds the tape, or the zip, if you put it in the same folder as
the program, named starquake.tap or STARQUAK.TAP in any case. You can
name it on the command line as well.

macOS: the program is not signed, so macOS blocks it the first time.
In Terminal, in this folder, run:

  xattr -d com.apple.quarantine starquake

Or try to open it once, then allow it under System Settings, Privacy &
Security.


CONTROLS
--------

  Arrow keys           Move, with the Kempston or cursor key control
                       methods. They press 5 6 7 8, the Spectrum's own
                       cursor keys.
  Up                   Pick up or swap items. With the hover platform,
                       rise.
  Down                 Build a platform.
  Alt, full stop or    Fire.
  comma
  Gamepad              D-pad or left stick to move. Up boards the
                       hover platform and flies it up, down flies it
                       down; neither picks up or builds.
                       The bottom face button only builds a platform,
                       the right one only picks up or swaps an item,
                       and the left one fires. A and B are wherever
                       the pad's maker puts them: on an Xbox pad the
                       platform is A and the item B, on a Switch pad
                       the other way round, on a PlayStation pad the
                       cross and the circle. Start pauses, Select
                       opens guidance (below). Works in
                       every control method, over USB or Bluetooth.
                       Some controllers need the right mode: an 8BitDo
                       in Switch mode is detected but sends no input,
                       so try one of its other modes.
  P                    Pause.
  A S D F G together   Abandon the game.
  Esc                  Guidance (below).
  F11                  Leave fullscreen for a window, or go back.
                       On a Mac: Control-Command-F.

The game starts fullscreen. The picture is always scaled by a whole
number, so fullscreen is often a step larger than any window fits.

At the title screen, 1 to 5 choose how to play, 6 defines your own
keys, 0 starts the game and Q quits. Any key stops the tune. On a
gamepad, Start or the fire button starts the game.

The high-score table, the CORE OF HEROES, is kept between runs in
high-scores.txt, in the folder the tape is kept in. Beside it the panel
shows how much guidance each entry's game had. A game played with
training mode on is shown in the table and then left out of it.


GUIDANCE
--------

Beside the game is a panel for optional help. Esc, or Select on a
gamepad, opens the guidance menu and pauses the game:

  Up and down          Choose a row.
  Left and right       Change the guidance level or training mode.
  Enter, or A          End this game, or exit, after pressing twice.
  Esc, or B or Select  Close the menu.

A and B are the pad's own letters, wherever its maker puts them: A is
the bottom button on an Xbox pad and the right one on a Switch pad. A
PlayStation pad uses the cross and the circle. The menu shows the
letters or marks of the pad that is connected.

Guidance has levels from 0, the original game, to 6; each adds to the
ones below it. Levels 1 to 3 show only what you could have written
down yourself; 4 and up tell you things you could not have known.
Turning it up, or training mode on, shows on that game's score, so
the menu asks before keeping such a change. So far:

  Level 1  Codes and the core: the codes of the teleporters you have
           seen, in a column at the panel's right, and the core's nine
           slots at its top left in the game's own pictures: bright
           while still wanted, dim once delivered, outlined while you
           carry that piece.
  Level 2  The map you have walked: the rooms you have visited. Walls are lines, and
           a gap in one is a way on; the square is where you are and
           the diamonds are the teleporters you have seen. A line
           inside a room divides it: the ways on either side of it do
           not meet, and a dashed one is a security door. A way on
           can still need a lift.
  Level 3  What you have seen: the rooms you have walked through
           that hold a core piece the core still needs, marked in pink.
           There are usually two of each piece; either will do, and
           while you carry one the other is not marked.
  Level 4  What you have not: the same in rooms you have not reached,
           outlined.

Levels 5 (routes) and 6 (every code and the whole planet), and
training mode, are still to come.


LEGAL
-----

Starquake is copyright (c) 1985 Stephen Crow / Bubble Bus Software.
This program is an independent reimplementation and is not affiliated
with, endorsed by, or approved by the rights holders.

The program is licensed under either the MIT licence (LICENSE-MIT) or
the Apache 2.0 licence (LICENSE-APACHE), at your option. That licence
covers only this reimplementation and grants no rights in Starquake
itself. THIRD-PARTY.txt lists the libraries built into the program and
their licences. The text is set in Inter, under the SIL Open Font
License (LICENSE-Inter.txt).

@starquake started this project and steered it, and Claude,
Anthropic's AI assistant, wrote it. The source code, and a fuller
account of how it was made, are at:

  https://github.com/starquake/starquake-recompiled
