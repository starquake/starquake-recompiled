# Assets

This project contains no part of the original game. Put your own copies of
these files here (everything in this directory except this README is
ignored by git):

| File | What | Needed by |
|------|------|-----------|
| `starquake.tap` | Starquake (Bubble Bus, 1985), `.tap` tape, SHA-1 `65450d6f33692c2c2868c0b497037f2cfd0ef3bd` | the game |
| `starquake.z80` | The same game as a 48K `.z80` snapshot, SHA-1 `8cf0722b752f7fe1651734b32a240e714525e480` | development tools only |
| `48.rom` | ZX Spectrum 48K ROM, SHA-1 `5ea7c2b824672e914525d1d5c419d71b84a426a2` | development tools only (the reference interpreter) |

**Only `starquake.tap` is needed to play.** The other two are used by
`sq-verify`, which compares the rewrite against the original; you can ignore
them unless you are working on the code.

## Where to get them

**The game.** [World of Spectrum](https://worldofspectrum.net/) archives
Spectrum software under a permissions system, asking copyright holders for
permission to distribute and honouring their wishes. If you already own the
game on tape, a dump of your own copy works just as well.

**The ROM.** Amstrad bought Sinclair's computer business in 1986 and gave
permission for the Spectrum ROMs to be redistributed with emulators, so long
as the copyright notice is kept and they are not sold; the rights passed to
Sky when Amstrad was bought in 2007. That permission is why emulators ship
the ROM, and the easiest legitimate sources are:

- [Fuse](https://fuse-emulator.sourceforge.net/), which includes `48.rom`
- the `spectrum-roms` package in Debian and Ubuntu
- World of Spectrum, which also hosts it

This is an informal permission rather than a formal licence, but it is the
basis emulator projects and Linux distributions have relied on for years.

## Why a tape rather than a snapshot

The game reads all graphics, maps and text from the tape when it starts, and
refuses to run without it or with a different dump. The tape also carries the
loading screen, which the game shows before its own title screen; a snapshot
has no loading screen, having been saved long after it was overwritten.

A tape is the better source: it is the program exactly as it shipped, where a
snapshot is somebody's machine part-way through a game, carrying whatever
control keys and random-number state it had at the time. `sq-verify tape`
compares the two and reports where they differ.
