# Contributing

Thank you for considering a contribution to cucco. Please read this page and
the [AI Policy](AI_POLICY.md) before opening an issue or a pull request.

## Project mission

cucco is an interactive CLI for writing
[conventional commits](https://www.conventionalcommits.org). It should stay a
small, dependable replacement for `git commit`: interactive prompts, scope
detection, and the same hook behaviour as git itself.

## Open an issue

Open an issue when you have a question, a bug to report, or a feature to
suggest. Use the matching issue template and fill it in completely.

- **Bug reports** must include your operating system, the cucco version
  (`cucco --version`), the git version, and the steps to reproduce.
- **Feature requests** should explain the problem you are trying to solve
  before proposing a solution. Wait for maintainer feedback before writing
  code for a new feature.

## Pull request process

1. Fork the repository and create a branch from `main`.
2. Install the tooling listed in [AGENTS.md](AGENTS.md) and run
   `just setup_githooks` once.
3. Write tests for your change. Behaviour changes need a test that fails
   before the change and passes after it.
4. Run `just check`. It must pass locally before you open the pull request.
5. Write your commits following
   [Conventional Commits](https://www.conventionalcommits.org). The changelog
   is generated from commit messages at release time, so the subject line
   matters.
6. Open the pull request using the template and complete the checklist,
   including the AI disclosure.
7. Wait for review. Address feedback with new commits; do not force-push over
   reviewed history.

## Software guidelines

- Keep dependencies to a minimum. Every new dependency must satisfy the policy
  in `deny.toml`.
- Public library items need rustdoc documentation.
- Prefer small, focused changes over sweeping refactors.

Thank you for any contribution!
