# starquake-recompiled

Starquake (Stephen Crow / Bubble Bus, 1985) reimplemented from scratch in Rust,
running natively rather than under emulation. **The repository contains no
original code or data**: every graphic, map, text and sound is read at startup
from the player's own copy of the game, which is required at runtime.

The rewrite is verified against the original by `tools/sq-verify`, which runs
the original's routines in a reference Z80 interpreter (`crates/zx-runtime`)
beside the rewritten code and compares the result byte for byte.

## Commands

- `.claude/scripts/check.sh` is the pre-PR gate: build, tests, clippy, docs,
  the no-frontend build, and then the differential suites. **Gate on the exit
  code, never on grepped output.**
- `cargo run --release -p sq-verify -- all assets/starquake.tap assets/48.rom`
  runs the 28 differential suites on their own.
- `cargo test -p zx-runtime --test fuse -- --nocapture` checks the interpreter
  against the Fuse Z80 corpus (1335 cases).
- `cargo run --release --all-features -p starquake -- assets/starquake.tap`
  plays it. Add `--headless <frames> <dir>` for screenshots.
- The tool shell is zsh: never name a variable `status`, and run anything
  loop-shaped as a `bash` script.

## Invariants

- **No game or ROM data is ever committed.** `assets/` is ignored except its
  README, and CI has a job that fails if anything slips through. This is what
  makes the project legal to publish; nothing is worth breaking it for.
- **The differential suites are the contract.** All 28 must match. A change
  that moves one is a deliberate, called-out decision, never a check adjusted
  to make it pass. The suites have twice rejected a plausible improvement, and
  both times they were right.
- **The interpreter is not self-certified.** Everything else rests on it, so it
  is checked against the Fuse corpus rather than against our own work. It must
  stay at 1335/1335. The remaining gap is ULA contention (#32), and
  `README.md` says so rather than overclaiming.
- **Fidelity first, and say so when it is not.** Where the rewrite cannot match
  the original exactly, the reason is written down (`README.md`, *Status*)
  rather than left to be discovered.

## How work lands

**The ticket is canonical.** Every conversation about a piece of work happens
in its GitHub issue. Chat is optional and the maintainer may not read it: an
answer given in chat is written back into the issue body before acting on it.

- **Everything lands via a pull request** with an issue behind it, including
  chores and docs. One issue, one deliverable; a ticket that needs several PRs
  in different states is split into sub-issues. A PR says `Closes #NN` only
  when it completes every task in the ticket's plan, maintainer steps included;
  otherwise `Part of #NN`, and the ticket stays open.
- **The board is the handoff baton**: the Status field of the "Starquake
  Recompiled" user Project
  (https://github.com/users/starquake/projects/5). Read and move it with
  `.claude/scripts/board.sh`.

  ```
  Backlog · Your input · Spec · Plan · Your sign-off · Build · Your review · Done
  ```

  **If a state says "your", it is the maintainer's gate** and work stops;
  `Spec`, `Plan` and `Build` are Claude's and proceed without re-asking.
  `Your input` (questions) can interrupt any stage. `Your sign-off` comes
  BEFORE a build (approve the spec, plan or mockup); `Your review` comes AFTER
  it (the PR is open, awaiting `ready to merge`). Cards move both ways and
  stages can be skipped: a bug or tweak goes straight to `Build`.

- **Approval** of a spec or plan is the maintainer dragging the card on, or a
  `go` / `approved` comment. Never proceed from plan to build without it.
- **A card dragged to `Spec` means "your call"**: first decide whether it needs
  a spec at all, and say so on the ticket. A bug or tweak goes on to `Build`.
- **The `Backlog` column's order is the priority.** Nothing leaves `Backlog`
  without the maintainer; "pick up the next one" means its top card.
- **Merging needs the `ready to merge` label** on the PR, re-read from the API
  at the moment of merging. Claude never adds it and never merges without it.
- **A position in the flow is a Status; a property of a ticket is a label**:
  `ready to merge`, `hold` (skip entirely), `needs: spec` / `needs: build`
  (the route, set when filing, with the reason in the body).
- **The body is the living spec; the comments are append-only history.** When
  a question is answered it moves into _Decisions_ and is deleted from _Open
  questions_. Every state change gets a NEW `> 🤖 **Next steps**` comment;
  never edit an old one.
- **Questions go in a copy-paste answer block**: a fenced block headed
  `# keep your pick, delete the rest`, one line per question, every line
  carrying a `(rec)`, ending with `notes =`. Posting one moves the ticket to
  `Your input` in the same step.
- **Visual work gets a mockup approved before the real UI is built**
  (`mockup` skill). On this project a mockup of anything the Spectrum draws is
  a real screenshot at 256×192, not an HTML sketch: the constraint is the
  design.

### Attribution: comments yes, commits and PR bodies no

`gh` acts as @starquake, so an unmarked Claude comment reads as the
maintainer's own answer — and the board monitor tells them apart by exactly
that prefix. So **every issue and comment Claude posts** opens with one of
these lines, posted via `--body-file`:

- `> 🤖 **Issue by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`
- `> 🤖 **Comment by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`

**Commit messages and pull request descriptions carry no attribution line**, by
the maintainer's standing instruction. Nothing reads those for provenance, so
nothing is lost.

The procedure behind each step lives in the skills: `work-the-board` (and its
`/board` alias), `design-slice`, `mockup`, `build-slice`, `merge-pr`,
`issue-comment-replies`.
