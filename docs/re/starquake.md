# Starquake reverse-engineering notes

Working notes for the rewrite. Everything here is a description in our own
words of how the original program works; no original code or data is
reproduced. Addresses refer to the 48K snapshot with SHA-1
`8cf0722b752f7fe1651734b32a240e714525e480`.

Status legend: **confirmed** (verified by running the original), **read**
(understood from the disassembly), **guess**.

## Screen layout (read)

- Character rows 0–5: HUD panel.
- Character rows 6–23: the room, 32 × 18 characters.

## Rooms (read)

- 512 rooms. Current room number is a 9-bit value at `D2C8`/`D2C9`.
- Room table at `7530`: 12 bytes per room, a 4 × 3 grid of *big block*
  indices, row-major, top row first.
- Big block table at `9840`: 4 bytes per block, a 2 × 2 grid of *tile*
  indices. A big block covers 8 × 6 characters; each tile slot is 4 × 3.
- Tile slots inside a big block are drawn right-to-left, top row first.
- Tile info table at `9740`, one byte per tile index:
  - `0`: nothing extra.
  - high nibble `1`–`4`: sets the current drawing colour to one of the
    room's four random colours (`A7F8`–`A7FB`).
  - otherwise bits 0–1 / 2–3 are x / y character offsets and the high
    nibble is an object type (`0x90`: records a position in the list at
    `96CB`, count at `96CA`).
- Room colours: on entry the room's RNG is seeded from the room's table
  entry, then four distinct values in 2–6 are drawn into `A7F8`–`A7FB`.

Verified: the rewrite's room builder matches the original byte for byte
(screen, RNG, colours, restore list, objects) in all 512 rooms.

## Pickups (read)

- Item table at `94E8`: 45 × 4 bytes `[colour<<5 | col, roomhi<<7 | row,
  room low, graphic]`. Row 0 = not yet put in its room. Randomised by the
  new-game setup (`6351`).
- On room entry (`AA30`, skipped in room 199): the first unplaced item for
  the room is put on a random spawn point with a random colour; then, if the
  room's bit in the bonus set (`A350`, 512 bits, MSB first) is set and the
  RNG allows, a bonus pickup (graphics `11`–`19`) goes on another spawn
  point; then every placed item in the room is drawn (markers `14`+index).
- "Random index" uses subtract-then-compare, which wraps: not plain modulo.
- The spawn index used for the item lives inside the code at `AA9F` and
  carries over between rooms.
- First visit to a room (bit in `A390`) clears the bit and adds 250 points.
- Inactive teleporter pads (entry low 7 bits 0) are blanked with bright
  spaces.

## Text (read)

All text goes through the ROM PRINT routine (`15F2`): AT, INK/PAPER/
BRIGHT/FLASH as temporary colours (with 8 = keep), OVER. Font is the
game's own at `ACD4` (glyph `c` at `ACD4 + 8c`); UDGs at `AEAC` (and
`5E71` for the title only). Verified against the ROM for the panel.

## Status panel (read, verified)

- Score: 6 digits at `D413`, pending additions per digit at `D419`.
- Lives at `D2CC`. Three bars at `D2CD`–`D2CF` (0–127; `D2CD` reaching 0
  kills BLOB). Bars are drawn with a full glyph `(` per 32 and a partial
  glyph `' '`..`'''` for eighths, in ink 8 over pre-coloured cells.
- Inventory: 4 slots at `D2D2` (graphic, attribute), drawn as XOR'd 2 × 2
  graphics from `9088 + 32n`.

## Tiles (read)

Drawn by `EA65` (L = tile index, B = character row, C = character column).

- Pointer table at `EB23`, 2 bytes per tile index.
- A tile definition pointer P points at 6 bytes of row masks (8 columns
  per row, bit 7 leftmost). Character bitmaps (8 bytes each) follow at
  P+6 in cell order; attributes are stored *backwards* from P−1.
- An attribute whose low 6 bits are `0` or `0x36` is a placeholder: the
  current room colour (`EA63`) is used, keeping the placeholder's bright
  and flash bits.
- Bright cells whose paper/ink bits are not `0x20` are appended to a
  restore list (3 bytes: address, attribute; pointer at `EA60`) so their
  colours can be re-applied after sprites are drawn.

## Random number generator (read)

State: 6 bytes at `DAC0`–`DAC5`; step routine `DAC6`. Deterministic, and
reseeded on room entry from the room table pointer, so room decoration is
fixed per room.

## Entities (read)

Six 32-byte slots at `DD18`. Slot 0 is BLOB (the player).

- `+05`/`+06`: x / y in pixels (x from the left, y from the bottom).
- `+07`/`+08`: graphics pointer.
- For BLOB: `DD1D`/`DD1E` position, `DD21` colour, `DD22` state,
  `DD23` input bits.

## Per-frame update (read)

`D9C8` calls, in order: sound tick (`A57B`), sprite draw (`DF70`, XOR,
uses SP as a table pointer), sprite attributes (`D8B1`), `DBEC`, RNG step,
`DCE6` (star field, guess), `A66C`.

## Map and room exits (read)

The map is 16 rooms wide and 32 tall: leaving through the left/right edge
changes the room by ∓1, through the top/bottom by −16/+16 (`C8F4`). BLOB's
y is snapped to the character grid on arrival.

## Enemy spawning (`9C47`, read, not yet rewritten)

- Room 199 (the core) and 198 are special-cased.
- Enemies occupy entity slots 1–4 (`9C43` = how many, normally 4).
- The previous room's enemies are cached at `959C` (4 × 21 bytes, swapped
  with slots 1–4 on each spawn). Going back to the room just left while a
  timer (`9C40`, set to 180) runs restores them instead of rolling new ones.
- New enemies get random parameters (`9DBA`–`9DC1`); graphic set
  `B208 + 192·type`. Each is placed at a random character cell whose 2 × 2
  neighbourhood is empty (attributes read through `9FFC`), up to 100 tries.
- Tile type 8 positions become stationary entities; the kind-12 marker
  (and BLOB state 2) put a special entity in slot 5 (`DD9D`).

## BLOB control (`C54F`, read, not yet rewritten)

- Input is read from the chosen controls (Kempston flag at `C567`) into
  direction bits at `DD23`: 0 right, 1 left, 2 up, 3 down, fire → `DD27`;
  opposite directions cancel.
- States (`DD22`): 0 walking/falling, 1 and 2 other modes (2 = riding the
  hover platform, moves freely).
- Walking moves 2 px and advances animation every 3 steps (`DD28`); facing
  0–4 at `DD26`, graphics `E074`/`E374`/`E674`.
- Falling accelerates by the table at `C751` (counter `DD29`).
- Pressing up with bar 2 (`D2CE`) non-zero builds a platform (up to 12,
  list at `DBBC`).
- Fire spends bar 3 (`D2CF`) and launches a shot (entity slot at `DDB8`)
  that travels until it hits something or leaves the room.
- Collision tests (`D2F0` horizontal, `D2F4` vertical) read the attribute
  map: a cell below `0x40` (not bright) is solid.

## Quirks the rewrite reproduces

- `D91A`: in this dump the check for one sprite-colour cell is `RST 38`
  instead of `JR NZ`. It runs the ROM interrupt routine mid-frame (one extra
  `FRAMES` count), always paints that cell, and leaves the colour mask at
  `F9` for the rest of the sprite.
- Behaviour-6 (stationary) enemies fall into a subroutine's `RET` when they
  turn, which ends the whole enemy update for that frame.
- Removing a pickup's colour records from the restore list removes four
  records but moves the list end back by eight.
- The 32 attribute bytes after the display (`5B00`) are a guard row that
  sprite colouring writes into; the game fills it with `40` at the start.

## Frame structure (verified)

Main loop `A523`: sound tick (`A57B`; its tone loop waits for the next
interrupt, so it is also the frame sync) → sprites → sprite colours →
platforms → RNG → sparkles → force fields → BLOB (`C552`) → out-of-energy
and force-field deaths → enemies (`A01B`).

## Sound (read)

- Per-frame tick `A57B`: two request bytes (`A41B` BLOB, `A41C` others),
  effect table `A607` (4 bytes: frames|prescale, pitch, step, wobble).
  Tone half-period = 35·e + 37 T-states.
- Blocking effects `D7C0`, table `D839` (5 bytes: start pitch, end pitch,
  step, xor mask, count|flags). They stall the game while playing.
- Tone loop `A5BA`: toggle, then count E down, polling `FRAMES` (`5C78`)
  each time round and returning when it changes. That poll is in contended
  memory, so the half period is longer while the picture is drawn than in
  the border.
- The effects' delay loop is `push ix` / `pop ix` / `dec d` / `jr nz`, on
  the stack at `5Dxx` (contended), so an effect's pitch depends on where in
  the frame it plays. An interrupt breaks into one that crosses a boundary
  (the ROM routine costs 895 T with no keys held).
- In play, the frame is silent until the work is done: the tone loop
  starts between about 40,000 and 54,000 T in, depending on the work.

## New game (`629D`, verified)

Control method (`5E58`) patches the input routine's key operands from key
name tables (`5E52`+5·method, player-defined at `5E6B`, pause `5E70`,
names in matrix order at `62D3`). Start values from the pair table `6343`
(room 8, 4 lives, bars FF/32/FF). Seed = `FRAMES`. Two special items
(graphics 0F, 10 = teleporter key), five core pieces in nine slots
(`D2DE`), 18 more items from room pairs at `5E2C`.

## Death (`C350`, read)

Reason ≥ 10: restart where BLOB entered. Reason 2 (energy): 45 flashes.
Then four fragments are launched and animated by the enemy code for 80
frames, a 50-frame pause, and a life is taken (or the game ends; final
scoring runs first).

## Other routines (read)

| Address | What it does |
|---------|--------------|
| `D3C1`  | Prints the `FF`-terminated string that follows the CALL (via the ROM print routine, so it uses AT/INK control codes). |
| `D5C8`  | Keyboard scan: returns a key code from the table at `D5A0` when exactly one key is down, else 0. |
| `D7C0`  | Sound effect A (5-byte parameter entries at `D839`). |
| `DB24`  | XORs a 2 × 2 character block onto the screen and sets its attribute. |
| `CE68`  | Address of 32-byte graphic A: `9088 + 32·A`. |
| `30A9`  | ROM HL-MULT (HL = HL·DE), used for table indexing. |

## Memory map (partial)

| Range | Contents |
|-------|----------|
| `5B20`–`5BBF` | Attribute restore list. |
| `5E2C`–`5E96` | Menu data. `5E58` = control method. |
| `5E97`–`6727` | Menu, intro text, new-game setup (`629D`, `6351`). |
| `7530`–`8D2F` | Room table. |
| `94E9`–      | 45 × 4-byte item placements, randomised at new game. |
| `9600`–`973F` | Per-room state, cleared on room entry. |
| `9740`–`983F` | Tile info. |
| `9840`–      | Big blocks. |
| `9C47`–`ABF9` | Game code: enemies, room setup (`A426`), room builder (`A80A`). |
| `C350`–`DD0C` | Game code: BLOB control, collision, HUD, sound. |
| `D2BE`–`D2DF` | Game state (room, lives, energy …). |
| `DD18`–`DDD7` | Entity slots. |
| `DF70`–`E070` | Sprite drawing. |
| `EA65`–`EB22` | Tile drawing; `EB23` tile pointer table. |
