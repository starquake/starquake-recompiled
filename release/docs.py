#!/usr/bin/env python3
"""Builds the documents that go into a release archive.

The archive is for people playing the game, so it gets `release/README.md`
rather than the repository's README, which is mostly about working on the
code. The controls table is the exception: it changes, and two copies would
drift, so it is copied in from the repository README at packaging time.

It also refuses to write a README that mentions development material, so
that cannot creep back into what players download.

THIRD-PARTY.md goes in too, less the note at its top telling maintainers it
is generated, which means nothing to a player.

    python3 release/docs.py OUTPUT_DIR
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# Things that only matter to someone working on the code. A player's README
# that mentions any of them has picked up something it should not have.
DEVELOPMENT = [
    "48.rom", "sq-verify", ".z80", "zx-runtime", "zx-recomp", "zx-core",
    "cargo", "docs/re", "Fuse", "differential", "assets/README", "reference interpreter",
]


def region(text: str, name: str) -> str:
    """The part of `text` between `<!-- release:NAME -->` and its closing marker."""
    m = re.search(rf"<!-- release:{name} -->\n(.*?)\n<!-- /release:{name} -->", text, re.S)
    if not m:
        sys.exit(f"README.md has no <!-- release:{name} --> region; it has to be marked for the release README")
    return m.group(1).strip()


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: docs.py OUTPUT_DIR")
    out_dir = pathlib.Path(sys.argv[1])
    out_dir.mkdir(parents=True, exist_ok=True)
    repo_readme = (ROOT / "README.md").read_text(encoding="utf-8")
    template = (ROOT / "release" / "README.md").read_text(encoding="utf-8")

    def include(m: re.Match) -> str:
        return region(repo_readme, m.group(1))

    out = re.sub(r"<!-- include: ([a-z-]+) -->", include, template)
    if "<!-- include:" in out:
        sys.exit("an include was left unexpanded")

    found = [term for term in DEVELOPMENT if term.lower() in out.lower()]
    if found:
        sys.exit("the release README mentions development material: " + ", ".join(found))

    (out_dir / "README.md").write_text(out, encoding="utf-8")

    third = (ROOT / "THIRD-PARTY.md").read_text(encoding="utf-8")
    third = re.sub(r"\A<!--.*?-->\s*", "", third, flags=re.S)
    (out_dir / "THIRD-PARTY.md").write_text(third, encoding="utf-8")
    print(f"wrote README.md and THIRD-PARTY.md to {out_dir}")


if __name__ == "__main__":
    main()
