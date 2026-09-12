# Contributing

## Run the differential suite before opening a pull request

```sh
cargo run --release -p sq-verify
```

CI cannot do this: it needs your own copy of the game and the Spectrum ROM,
neither of which is ever committed. Every check must pass, and a check that
reports `no cases ran` counts as a failure, not a pass.

## When a lint argues with the code

Much of this source mirrors the original game instruction for instruction.
Some lints will object to that on style grounds: nesting that follows the Z80
control flow, arithmetic written the way the original writes it, a match arm
per original branch.

When that happens, the answer is a targeted `#[allow(...)]` with a comment
saying which original behaviour it protects — **not** rewriting the logic to
please the lint. A tidier shape that computes something subtly different is a
regression this project cannot afford, and the only thing standing between it
and the rest of the code is the differential suite.

If a lint fix does touch game logic, re-run `sq-verify` before pushing, and
say in the commit message that you did.

## Fidelity comes before tidiness

The point of the project is that the rewrite does exactly what the original
does, quirks and bugs included. Two rules follow:

- A "fix" that makes the game behave better than the original is a bug here.
  Reproduce the original, and note the oddity in a comment.
- If a change alters anything the suite compares, the suite decides whether
  the change is right. It has already rejected several plausible-looking
  improvements, including masking the room number to 9 bits: the original
  really does let it run past 511 when you walk off the edge of the map.

## Never commit the game

`assets/` holds your own copy of Starquake and the Spectrum ROM, and is
ignored except for its README. CI fails if any game dump reaches the tree.
