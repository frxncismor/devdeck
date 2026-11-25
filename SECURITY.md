# Security Policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |
| < 0.1   | No        |

## Reporting a vulnerability

**Please do not open a public issue** for security problems.

Use GitHub's private vulnerability reporting instead: go to the repository's **Security** tab and choose **Report a vulnerability**, or open <https://github.com/frxncismor/devdeck/security/advisories/new>.

Please include:

- A description of the issue and its impact.
- The DevDeck version and operating system.
- Steps to reproduce, or a proof of concept.
- Any suggested fix, if you have one.

Do not include real secrets, tokens or personal data in the report.

## What to expect

DevDeck is maintained in spare time, so these are goals, not guarantees:

- An initial acknowledgement within about a week.
- A fix or mitigation for confirmed issues as soon as practical, followed by a release and a note in the [changelog](CHANGELOG.md).
- Credit in the advisory if you want it.

Please give us reasonable time to fix the issue before disclosing it publicly.

## Existing safeguards

- A pinned gitleaks pre-commit hook (fail closed) and a gitleaks scan in CI over each PR or push.
- Minimal Tauri capabilities (only the dialog and clipboard-write permissions the frontend needs; no shell or opener access) and a strict Content Security Policy.
- Backend path validation: worktree actions only accept known worktrees, the main worktree cannot be removed, and folder deletion only accepts allow-listed names.
- Destructive actions always require confirmation.
