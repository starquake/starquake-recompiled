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

World of Spectrum archives Spectrum software with the permission of its
copyright holders. A dump of a tape you own works just as well.

  https://worldofspectrum.net/


RUNNING IT
----------

Put starquake.tap in the same folder as the program and run starquake,
or starquake.exe on Windows. If your tape is somewhere else, give its
path on the command line.

The program also looks in the usual place for application data:

  Linux     ~/.local/share/starquake-recompiled/
  macOS     ~/Library/Application Support/starquake-recompiled/
  Windows   %APPDATA%\starquake-recompiled\

If it cannot find a tape, it lists every place it looked.

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
                       shoulder button to fire, Start to pause. Works
                       in every control method, over USB or Bluetooth.
                       Some controllers need the right mode: an 8BitDo
                       in Switch mode is detected but sends no input,
                       so try one of its other modes.
  P                    Pause.
  A S D F G together   Abandon the game.

At the title screen, 1 to 5 choose how to play, 6 defines your own
keys, 0 starts the game and Q quits. Any key stops the tune.


LEGAL
-----

Starquake is copyright (c) 1985 Stephen Crow / Bubble Bus Software.
This program is an independent reimplementation and is not affiliated
with, endorsed by, or approved by the rights holders.

The program is licensed under either the MIT licence (LICENSE-MIT) or
the Apache 2.0 licence (LICENSE-APACHE), at your option. That licence
covers only this reimplementation and grants no rights in Starquake
itself. THIRD-PARTY.txt lists the libraries built into the program and
their licences.

@starquake started this project and steered it, and Claude,
Anthropic's AI assistant, wrote it. The source code, and a fuller
account of how it was made, are at:

  https://github.com/starquake/starquake-recompiled
