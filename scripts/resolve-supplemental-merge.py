#!/usr/bin/env python3
"""Resolve standard supplemental-lane merge conflicts on master."""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def git_show(ref: str, path: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(ROOT), "show", f"{ref}:{path}"], text=True
    )


def merge_catalog(ours_ref: str, theirs_ref: str) -> None:
    ours = json.loads(git_show(ours_ref, "data/rules/catalog.json"))
    theirs = json.loads(git_show(theirs_ref, "data/rules/catalog.json"))
    seen = {r["ruleId"] for r in ours["rules"]}
    for rule in theirs["rules"]:
        if rule["ruleId"] not in seen:
            ours["rules"].append(rule)
            seen.add(rule["ruleId"])
    ours["rustSupportedCount"] = sum(
        1 for r in ours["rules"] if r.get("implementationStatus") == "rust-supported"
    )
    ours["typescriptSupportedCount"] = sum(
        1
        for r in ours["rules"]
        if r.get("implementationStatus") == "typescript-supported"
    )
    path = ROOT / "data/rules/catalog.json"
    path.write_text(json.dumps(ours, indent=2) + "\n")
    print(f"catalog -> {ours['rustSupportedCount']} rust-supported")


def merge_eligibility(ours_ref: str, theirs_ref: str) -> None:
    ours = git_show(ours_ref, "crates/sorcery-engine/tests/eligibility_rules.rs")
    theirs = git_show(theirs_ref, "crates/sorcery-engine/tests/eligibility_rules.rs")
    for match in re.finditer(
        r"(#\[test\]\nfn rule_catalog_\d+.*?)(?=\n#\[test\]|\Z)", theirs, re.S
    ):
        block = match.group(1).strip()
        fn = re.search(r"fn (rule_catalog_\d+_\w+)", block)
        if fn and fn.group(1) not in ours:
            ours = ours.rstrip() + "\n\n" + block + "\n"
    path = ROOT / "crates/sorcery-engine/tests/eligibility_rules.rs"
    path.write_text(ours)


def merge_handoff(ours_ref: str, theirs_ref: str) -> None:
    ours = git_show(ours_ref, "OVERNIGHT-HANDOFF.md")
    theirs = git_show(theirs_ref, "OVERNIGHT-HANDOFF.md")
    count_match = re.search(r'"rustSupportedCount": (\d+)', git_show(theirs_ref, "data/rules/catalog.json"))
    count = count_match.group(1) if count_match else "?"
    next_match = re.search(r"next IDs start at `(\d+)`", theirs)
    next_id = next_match.group(1) if next_match else "?"
    # Take theirs opening clause (newest lane summary) but keep master's long tail if longer
    theirs_line1 = theirs.split("\n", 2)[2] if theirs.startswith("#") else theirs
    ours_lines = ours.split("\n")
    if len(theirs_line1) > len(ours_lines[2]):
        ours_lines[2] = theirs_line1.split("\n")[0]
    ours = "\n".join(ours_lines)
    ours = re.sub(
        r"`data/rules/catalog\.json`: \*\*\d+ rust-supported / 0 typescript-supported\*\* out of \d+\.",
        f"`data/rules/catalog.json`: **{count} rust-supported / 0 typescript-supported** out of {count}.",
        ours,
    )
    ours = re.sub(r"next IDs start at `\d+`", f"next IDs start at `{next_id}`", ours)
    (ROOT / "OVERNIGHT-HANDOFF.md").write_text(ours)


def main() -> None:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <theirs-ref>", file=sys.stderr)
        sys.exit(1)
    theirs = sys.argv[1]
    ours = "HEAD"
    merge_catalog(ours, theirs)
    merge_eligibility(ours, theirs)
    merge_handoff(ours, theirs)


if __name__ == "__main__":
    main()
