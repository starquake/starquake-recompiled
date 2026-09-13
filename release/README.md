# Starquake

A reimplementation of **Starquake** (Stephen Crow, Bubble Bus Software, 1985) for modern computers. It is not an emulator: the game is rewritten as ordinary Rust.

## You need the original game

This program contains no code or data from the original game. It reads every graphic, map, piece of text and sound from your own copy of Starquake, a `.tap` tape, when it starts, and it will not run without one.

[World of Spectrum](https://worldofspectrum.net/) archives Spectrum software with the permission of its copyright holders. A dump of a tape you own works just as well.

## Running it

Put `starquake.tap` in the same folder as the program and run `starquake`, or `starquake.exe` on Windows. If your tape is somewhere else, give its path on the command line.

The program also looks in the usual place for application data: `~/.local/share/starquake-recompiled/` on Linux, `~/Library/Application Support/starquake-recompiled/` on macOS, and `%APPDATA%\starquake-recompiled\` on Windows. If it cannot find a tape it lists every place it looked.

**macOS:** the program is not signed, so macOS blocks it the first time. In Terminal, in this folder, run `xattr -d com.apple.quarantine starquake`. Or try to open it once, then allow it under System Settings, Privacy & Security.

## Controls

<!-- include: controls -->

## Legal

Starquake is copyright © 1985 Stephen Crow / Bubble Bus Software. This program is an independent reimplementation and is **not affiliated with, endorsed by, or approved by** the rights holders.

The program is licensed under either the [MIT](LICENSE-MIT) or the [Apache 2.0](LICENSE-APACHE) licence, at your option. That licence covers only this reimplementation and grants no rights in Starquake itself. [THIRD-PARTY.md](THIRD-PARTY.md) lists the libraries built into the program and their licences.

@starquake started this project and steered it, and Claude, Anthropic's AI assistant, wrote it. The source code, and a fuller account of how it was made, are at <https://github.com/starquake/starquake-recompiled>.
