---
name: build-slice
description: >
  Use whenever an approved ticket gets built: "build #NN", "go ahead and
  implement it", "execute the plan", "resume #NN", a ticket moved to `Build`
  on the board, or a `go` / `approved` comment on a `Your sign-off` ticket.
  Also for bugs and tweaks routed `needs: build`. Executes the plan task by
  task on ONE PR: branch, failing tests first, the check gate green per commit,
  CI watched after every push, then the ticket moves to `Your review`. Merging
  stays gated on the `ready to merge` label. Trigger even if the user doesn't
  say "skill".
---

You execute an approved plan from a ticket, task by task, on one PR.
Precondition: the issue's *Plan* exists and the maintainer has approved it (or
it's a bug/tweak routed `needs: build`). If not, stop and route to
`design-slice`: never invent a plan mid-build.

## Setup

- Set the card to `Build` (`.claude/scripts/board.sh state <n> "Build"`).
- Branch from an up-to-date `main`, named for the ticket. In a worktree, run
  every `git` command inside it; never `git checkout` in the shared checkout
  while another agent may be working there.
- Re-verify the plan against the current tree before the first commit: the
  named symbols still exist, and nothing merged since moved the ground. Surface
  drift rather than improvising.
- **Resuming?** The issue's ticked checkboxes plus the branch's commits are
  the progress record. Confirm they agree, and pick up at the first unticked
  task.

## The task loop

1. **Failing test first** where the plan says so, and confirm it fails for the
   right reason. A test that can never run (skipped, unreachable) is worse than
   none: check it actually ran.
2. Implement. Keep the invariants (CLAUDE.md): no game or ROM data is ever
   committed, and all 25 differential suites still match. A suite that moves
   is a deliberate, called-out decision, never a check adjusted to pass.
3. **Gate on the exit code, never on grepped output:**

   ```bash
if .claude/scripts/check.sh > "$TMPDIR/check.log" 2>&1; then echo "GATE PASS"; else echo "GATE FAIL"; tail -40 "$TMPDIR/check.log"; fi
```

   `check.sh` runs what CI runs, plus the differential suites when the game
   and ROM are in `assets/` — and they are the gate that matters, because CI
   cannot run them. The tool shell is zsh: never pipe the gate into `tail` and
   read `$?`, since that's the pipe's status.
4. One commit per task, with a message that says what and why. Push, then tick
   the task in the issue's *Plan*.
5. **Watch CI to completion in the foreground and read EVERY job**, for the
   commit you just pushed (a stale run shows for a few seconds after a push):

   ```bash
   while gh pr checks <n> --json bucket -q '.[].bucket' | grep -q pending; do sleep 30; done
   gh pr checks <n> --json name,bucket -q '.[] | "\(.bucket)\t\(.name)"'
   ```

   A subagent is never woken by its own background task, so don't background
   the watch and end. **A flaky test is a bug**: reproduce it, root-cause it,
   and fix the cause. A re-run unblocks a PR; it doesn't end the flake.

## The PR

Open it early as a draft, linked to the ticket. **`Closes #NN` only if this PR
completes every task in the ticket's plan**; otherwise `Part of #NN`. A task
left for the maintainer (a settings change, a manual step) counts as open: a
`Closes` would shut the ticket on merge with that task still undone (#22).

**Review is the bottleneck**: the maintainer reviews alone, so the body is a
guide to reviewing, not a defence.

0. **No attribution line on a PR body or a commit message** — the maintainer
   asked for those to stay clean. Issues and comments carry it; these do not.
1. **`## Where to look`**: the two or three judgement calls the maintainer
   might disagree with, each naming its file. None? Say so in one line.
2. `---`, then *Mechanically verified — skip unless curious.* and ONE short
   paragraph: which gates passed and what was checked.

Comments follow the same rule: the answer goes in the **first line**. Keep
mechanical churn (regenerated files, formatting) in its own commit.

## Finish

- Docs: update `README.md` / `CLAUDE.md` if anything they say changed. If the
  change touches what `sq-verify` proves, `README.md`'s *Verification* section
  says what the claim rests on and must stay true to it.
- **A mockup in the ticket?** Its image must reach `main` in this PR. Once
  merged, repoint the embed from `/raw/<branch>/` to `/raw/main/`, because
  merging deletes the branch and a branch embed 404s.
- `gh pr ready <n>`, watch CI green, then **STOP. Never merge.** The
  `ready to merge` label is the maintainer's; `merge-pr` lands it once they
  add it.
- **Re-read the ticket's plan before handing over.** Every box this PR covers
  is ticked. If any box is still open, including one that's the maintainer's,
  the PR body must say `Part of #NN`, not `Closes #NN`; fix it now.
- Move the card to **`Your review`** (NOT `Your sign-off`, which is the
  pre-build gate) and post a Next-steps comment: *Next: review the PR and add
  `ready to merge`.* List any task still open after the merge in that comment,
  with whose it is.
