# Assets

This project contains no part of the original game. Put your own copies of
these files here (everything in this directory except this README is
ignored by git):

| File | What | Needed by |
|------|------|-----------|
| `starquake.tap` | Starquake (Bubble Bus, 1985), `.tap` tape, SHA-1 `65450d6f33692c2c2868c0b497037f2cfd0ef3bd` | the game |
| `48.rom` | ZX Spectrum 48K ROM, SHA-1 `5ea7c2b824672e914525d1d5c419d71b84a426a2` | development tools only (the reference interpreter) |
| `tests.in`, `tests.expected` | The Fuse project's Z80 test corpus | development tools only (the processor conformance test) |

**Only `starquake.tap` is needed to play.** The ROM is used by `sq-verify`,
which runs the original from the same tape and compares the rewrite against
it, and the corpus by the processor test; you can ignore them unless you are
working on the code.

## Where to get them

**The game.** [World of Spectrum](https://worldofspectrum.net/) and
[Spectrum Computing](https://spectrumcomputing.co.uk/) keep Spectrum software
available and remove titles whose rights holders object. Both list Starquake
as available, which means nobody has objected, not that its rights holders
gave permission: World of Spectrum's own
[permissions page](https://worldofspectrum.net/permits/) says only a minority
of software houses ever did, and whether Bubble Bus or Stephen Crow were among
them is not known here. If you already own the game on tape, a dump of your
own copy works just as well.

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

**The Z80 test corpus.** Two text files from the
[Fuse](https://fuse-emulator.sourceforge.net/) project, stating for 1335
cases what a Z80's registers, memory and T-state count should be after
running. They are GPL-licensed, which is why they are fetched rather than
copied in here:

```sh
base='https://sourceforge.net/p/fuse-emulator/code/HEAD/tree/trunk/fuse/z80/tests'
curl -L -o assets/tests.in "$base/tests.in?format=raw"
curl -L -o assets/tests.expected "$base/tests.expected?format=raw"
```

Without them `cargo test` says the conformance test was skipped, and every
other check still runs.

## Why a tape rather than a snapshot

The game reads all graphics, maps and text from the tape when it starts, and
refuses to run without it or with a different dump. The tape also carries the
loading screen, which the game shows before its own title screen.

A tape is the program exactly as it shipped, where a snapshot is somebody's
machine part-way through a game, carrying whatever it had at the time. The
`.z80` snapshot this project was first checked against carried one damaged
byte in the game's code (`D91A`), which the rewrite copied and the checks,
running the same snapshot, agreed with: lifts could not be boarded walking
right, and green scenery turned white under BLOB
(starquake-recompiled#117). Nothing reads a snapshot now; `sq-verify` and
`zx-recomp` start the original from the tape too.
