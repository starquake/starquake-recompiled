#!/usr/bin/env bash
# check.sh — the pre-PR gate. Exits non-zero if anything fails.
#
# Runs what CI runs, and then the differential suites, which CI cannot: they
# need the player's own copy of the game and the Spectrum ROM, and those are
# never committed. So this script is the only place the whole gate exists.
#
# Gate on the EXIT CODE. Never grep the output.
set -uo pipefail

cd "$(dirname "$0")/../.." || exit 1
failed=()
run() {
  local name="$1"; shift
  echo "=== $name"
  if "$@"; then return 0; fi
  failed+=("$name")
}

run "fmt"          cargo fmt --all --check
run "build"        cargo build --workspace --all-targets --all-features --locked
run "test"         cargo test --workspace --locked
run "clippy"       cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" run "doc" cargo doc --workspace --no-deps --locked
# The game library is meant to have no platform dependencies at all.
run "no-frontend"  cargo build -p starquake --no-default-features --locked
# What the dependency tree is allowed to contain. Skipped rather than failed
# when the tool is absent, since it is the one check here that needs an
# install: `cargo install cargo-deny --locked`.
if command -v cargo-deny > /dev/null; then
  run "cargo-deny"  cargo deny check
else
  echo "!!! cargo-deny not installed; the dependency policy was NOT checked."
fi

# The attributions shipped with a binary. Same story: an install away, and
# worth saying loudly when it did not run.
if command -v cargo-about > /dev/null; then
  echo "=== third-party attributions"
  cargo about generate --all-features about.hbs -o "${TMPDIR:-/tmp}/THIRD-PARTY.md" 2> /dev/null
  if diff -q THIRD-PARTY.md "${TMPDIR:-/tmp}/THIRD-PARTY.md" > /dev/null; then
    echo "THIRD-PARTY.md is current"
  else
    failed+=("THIRD-PARTY.md is stale: cargo about generate --all-features about.hbs -o THIRD-PARTY.md")
  fi
else
  echo "!!! cargo-about not installed; THIRD-PARTY.md was NOT checked."
fi

# The differential suites: the rewrite against the original, byte for byte.
# Without the game and the ROM they cannot run, and that is worth saying
# loudly rather than passing quietly — a silent skip is how a gate rots.
if [ -f assets/starquake.tap ] && [ -f assets/48.rom ]; then
  echo "=== sq-verify"
  out=$(cargo run --release -q -p sq-verify -- all assets/starquake.tap assets/48.rom 2>&1)
  status=$?
  echo "$out" | tail -40
  suites=$(printf '%s\n' "$out" | grep -c 'cases match')
  if [ $status -ne 0 ] || [ "$suites" -lt 25 ]; then
    failed+=("sq-verify ($suites/25 suites reported)")
  fi
else
  echo "!!! sq-verify SKIPPED: assets/starquake.tap or assets/48.rom missing."
  echo "!!! The fidelity gate did NOT run. See assets/README.md."
fi

if [ ${#failed[@]} -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[@]}"
  exit 1
fi
echo "all checks passed"
