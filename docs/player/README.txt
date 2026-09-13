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
  Gamepad              D-pad or left stick to move, any face or
                       shoulder button to fire, Start to pause, Select
                       for guidance (below). Works in every control
                       method, over USB or Bluetooth.
                       Some controllers need the right mode: an 8BitDo
                       in Switch mode is detected but sends no input,
                       so try one of its other modes.
  P                    Pause.
  A S D F G together   Abandon the game.
  Esc                  Guidance (below).

At the title screen, 1 to 5 choose how to play, 6 defines your own
keys, 0 starts the game and Q quits. Any key stops the tune.


GUIDANCE
--------

Beside the game is a panel for optional help. Esc, or Select on a
gamepad, opens the guidance menu and pauses the game:

  Up and down          Choose a row.
  Left and right       Change the guidance level or training mode.
  Enter, or A          End this game, or exit, after pressing twice.
  Esc, or B or Select  Close the menu.

Guidance has levels from 0, the original game, to 5; each adds to the
ones below it. Turning it up, or training mode on, shows on that
game's score, so the menu asks before keeping such a change. This
version has the menu and the panel; what the levels and training mode
do is still to come.


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
