#!/usr/bin/env python3
"""Check a pull request's commits are a person's, with no AI session metadata.

For every commit in BASE..HEAD:

- its author and committer must not be an AI agent, such as
  `Claude <noreply@anthropic.com>`: commit as the person responsible;
- its message must not carry a generated-session trailer, session link,
  AI co-author trailer or "Generated with" footer.

It runs on every pull request, over the commits it adds, and on every push to
`main`, over the commits pushed. So whatever reaches `main` is checked, however
it was merged: a pull request's own commits, a merge commit, or a squash commit
whose message was written or edited when merging. The pull request body isn't
checked as such; where it becomes a commit message, that commit is. Exits 1 and
lists each offence.

Usage: check-commit-metadata.py BASE HEAD
"""

import re
import subprocess
import sys

AGENT_NAME = re.compile(r"\bclaude\b", re.IGNORECASE)
AGENT_EMAIL = re.compile(r"@anthropic\.com$", re.IGNORECASE)

PATTERNS = [
    ("session trailer", re.compile(r"^\s*Claude-Session\s*:", re.IGNORECASE)),
    ("session link", re.compile(r"\bclaude\.ai/", re.IGNORECASE)),
    (
        "co-author trailer",
        re.compile(
            r"^\s*Co-Authored-By\s*:.*(\bClaude\b|@anthropic\.com)",
            re.IGNORECASE,
        ),
    ),
    (
        "generated-by footer",
        re.compile(r"Generated\s+(with|by)\s+\[?Claude\b", re.IGNORECASE),
    ),
]


def findings(text: str) -> list[tuple[str, str]]:
    """Return (rule, line) for every line of text that matches a rule."""
    found = []
    for line in text.splitlines():
        for rule, pattern in PATTERNS:
            if pattern.search(line):
                found.append((rule, line.strip()))
                break
    return found


def identity_findings(role: str, name: str, email: str) -> list[tuple[str, str]]:
    """Return (rule, identity) if an author or committer is an AI agent."""
    if AGENT_NAME.search(name) or AGENT_EMAIL.search(email):
        return [(f"{role} is an AI agent", f"{name} <{email}>")]
    return []


FIELDS = ["%H", "%an", "%ae", "%cn", "%ce", "%B"]


def commits(base: str, head: str) -> list[dict[str, str]]:
    output = subprocess.run(
        ["git", "log", "--format=" + "%x00".join(FIELDS) + "%x1e", f"{base}..{head}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    found = []
    for record in output.split("\x1e"):
        record = record.strip("\n")
        if record:
            sha, an, ae, cn, ce, message = record.split("\x00", 5)
            found.append(
                {"sha": sha, "an": an, "ae": ae, "cn": cn, "ce": ce, "message": message}
            )
    return found


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    base, head = argv[1], argv[2]
    failures = []
    for commit in commits(base, head):
        where = f"commit {commit['sha'][:12]}"
        failures += [
            (where, rule, found)
            for rule, found in identity_findings("author", commit["an"], commit["ae"])
            + identity_findings("committer", commit["cn"], commit["ce"])
            + findings(commit["message"])
        ]
    for where, rule, line in failures:
        print(f"::error::{where}: {rule}: {line}")
    if failures:
        print(
            "Commit as yourself (git config user.name and user.email) and "
            "remove these lines: for example `git rebase -x 'git commit --amend "
            "--no-edit --reset-author' BASE`, then reword the messages and push.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
