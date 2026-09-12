# Starquake in Rust

A from-scratch reimplementation of **Starquake** (Stephen Crow, Bubble Bus
Software, 1985) for modern systems, written in Rust. It is not an emulator:
the game logic is rewritten as ordinary Rust, and the original is only used
as a reference while developing.

## You need the original game

This repository contains **no code or data from the original game**. The
game loads every graphic, map and piece of text at startup from your own
copy of Starquake — a `.tap` tape, see [assets/README.md](assets/README.md) —
and will not run without it. A tape also carries the loading screen, which
is shown before the title screen.

[World of Spectrum](https://worldofspectrum.net/) archives Spectrum software
with the permission of its copyright holders; a dump of your own tape works
equally well. See [assets/README.md](assets/README.md) for that and for where
the development tools' ZX Spectrum ROM can legitimately be had.

## Playing

```sh
cargo run --release -p starquake -- assets/starquake.tap
```

| Key | Action |
|-----|--------|
| Arrow keys | Move. They drive the Kempston joystick and press `5` `6` `7` `8`, which are the Spectrum's own cursor keys, so they work in the Kempston, cursor and UDK methods alike |
| Up | Pick up / swap items; with the hover platform, rise |
| Down | Build a platform |
| Alt, `.` or `,` | Fire (Kempston) |
| USB gamepad | Drives the Kempston joystick: d-pad or left stick to move, any face or shoulder button to fire, Start to pause |
| P | Pause |
| A S D F G together | Abandon the game |

At the title screen, **1**–**5** choose how to play, **6** defines your own
keys, **0** starts the game and **Q** quits. Any key stops the tune.

## Status

The whole game is here: the title screen and its menu, the intro, play,
security doors, teleporter booths and the Cheops pyramid, losing a life,
delivering pieces to the planet's core, the ending, the game-over screen and
the high-score table, with the original's sound and music.

Verified against the original, byte for byte, by running the same situations
through both — and the interpreter that runs the original is itself checked
against an independent description of the processor (see *Verification*):

- Room building (all 512 rooms), the status panel and its text printing,
  pickups, entering rooms, enemy spawning.
- Every frame of play: sprites, their colours, platforms, sparkles, force
  fields, enemy behaviour, BLOB's movement, shooting, lifts, hover platform,
  pickups and inventory, hazards, room exits, and the sound state.
- New-game setup for every control method, and the menu screens.
- Losing a life, the game-over screen, security doors, and the core room.
- The music: every tune matches the original's timing to the T-state, which
  is what its pitch, its buzz and its tempo are made of.

Rewritten but not checked against the original on their own: the intro and
high-score screens, teleporter booths, the Cheops pyramid, and the ending
screen. They are built out of the drawing, printing and scoring code the
checks above do cover.

Two things cannot match exactly, by their nature: anything derived from how
long the player took (the frame counter seeds a room's random numbers, and
the time is shown at the end), and screens whose loops run faster than 50 Hz
in the original, which here take one step per frame.

## Layout

| Path | What |
|------|------|
| `games/starquake` | The game (library) and the playable program (`frontend` feature: pixels, winit, cpal). |
| `tools/sq-verify` | Differential tests against the original (development only). |
| `crates/zx-runtime` | Reference ZX Spectrum/Z80 interpreter that runs the original for comparison (development only; not part of the game). |
| `crates/zx-core` | Z80 decoder, `.z80` loader, PNG writer. |
| `crates/zx-recomp` | Tracing and disassembly-listing tool used for reverse engineering (development only). |
| `docs/re` | Reverse-engineering notes. |

## Verification

With `starquake.z80` and `48.rom` in `assets/`:

```sh
cargo run --release -p sq-verify
```

Each check runs a routine of the original in the reference interpreter and
the rewritten code from the same starting state (thousands of states, from
real play and from a tour of the map), then compares the resulting screen
and game state byte for byte.

### What the reference interpreter rests on

Those checks only prove the rewrite matches our interpreter, and the rewrite
was written by checking against that interpreter — so an interpreter that
got an opcode wrong would have the mistake copied into the rewrite, and
every check above would still pass.

So the interpreter is checked too, against something nobody here wrote: the
[Fuse](https://fuse-emulator.sourceforge.net/) project's Z80 test corpus,
which states for 1335 cases what the registers, memory and T-state count
should be afterwards, undocumented behaviour included.

```sh
cargo test -p zx-runtime --test fuse -- --nocapture
```

It found three real faults, none of which Starquake happened to depend on:
`BIT n,(IX+d)` took flag bits 3 and 5 from the byte tested instead of from
the high byte of the address, `HALT` left PC past the instruction instead of
on it, and writes below 0x4000 were dropped even with no ROM loaded. All
1335 cases pass now.

The corpus does not check bus timing cycle by cycle (the contention pattern
the ULA imposes), which this interpreter accounts for one instruction at a
time rather than one cycle at a time. That is the gap that remains.

The checks read the `.z80` snapshot and `48.rom`, which are needed only for
development: the reference interpreter needs a running machine to compare
against. The tape and the snapshot hold the same program — `sq-verify tape`
compares them and reports that they differ only in the bytes the game itself
writes while running (control method, keys, random-number state and such),
with no difference in any instruction.

`starquake --headless FRAMES DIR` plays with random input and writes
screenshots, for testing without a window.

`starquake --bench SECONDS` plays with its real sound and pacing but no
window, and reports how long frames actually took — the game is paced by the
clock at the Spectrum's own frame rate (19.968ms), so a median away from that
means the pacing is off rather than the game being slow.

## Legal

Starquake is copyright © 1985 Stephen Crow / Bubble Bus Software. This project
is an independent reimplementation and is **not affiliated with, endorsed by,
or approved by** the rights holders.

It contains **no code, graphics, maps, text or sound from the original game**.
All of that is read at startup from a copy of the original that you supply
yourself, and the program will not run without one. The reverse-engineering
notes in `docs/re` are descriptions in our own words; no disassembly of the
original is reproduced here.

The Rust code in this repository is licensed under either [MIT](LICENSE-MIT)
or [Apache 2.0](LICENSE-APACHE), at your option. That licence covers only this
reimplementation and grants no rights in Starquake itself.
