# AGENTS.md

Guidance for coding agents working in this repository.

## Project context

cucco is an interactive CLI for creating conventional commits. It is a fork of
[koji](https://github.com/cococonscious/koji) that adds git hooks execution.
One Cargo package with a library (`src/lib/`) and a binary (`src/bin/main.rs`).

The toolchain is pinned to Rust 1.98.1 with edition 2024. Keep
`rust-toolchain.toml` and `package.rust-version` in `Cargo.toml` synchronized.

## Working rules

- Read the relevant source, documentation, configuration, and tests before
  making changes.
- Keep changes scoped to the requested task and preserve unrelated user work.
- Use existing project patterns before adding new abstractions.
- Add or update focused tests before changing behavior.
- Update user documentation when commands, public APIs, configuration, or
  workflows change.
- Use `git`; never use `jj` or git worktrees.
- Never open a GitHub issue without prior user approval.
- Do not stage or commit local plans. Planning state belongs under the ignored
  `docs/superpowers/`, `.superpowers/`, or `.claude/plans/` directories.

## Source layout

- `src/bin/main.rs`: clap argument parsing and the interactive flow.
- `src/lib/lib.rs`: library root exporting the modules below.
- `src/lib/answers.rs`: turns prompt answers into commit parts.
- `src/lib/commit.rs`: message generation, `COMMIT_EDITMSG` writing, staging
  (`git commit -a` semantics for `--all`, `git add -A` semantics for
  `--add-all`) and the commit itself, including `pre-commit` and
  `post-commit` hooks through cocogitto.
- `src/lib/config.rs`: layered configuration (`.cucco.toml`,
  `cucco/config.toml`, defaults embedded from `meta/config/default.toml`).
- `src/lib/emoji.rs`: emoji and shortcode handling.
- `src/lib/multiline.rs`: crate-private multi-line text prompt used for the
  body and breaking change prompts.
- `src/lib/questions.rs`: inquire prompts and scope autocompletion.
- `src/lib/scope.rs`: scope detection from staged paths and ast-grep rules.
- `src/lib/status.rs`: staging area inspection.
- `tests/integration.rs`: end-to-end tests driving the binary through a PTY
  (`rexpect`, Unix only) and plain `assert_cmd` tests.
- `dist/release/`: release scripts (version bump, static musl builds).
- `install.sh`, `install.ps1`, `dist/pages/`: installers published on GitHub
  Pages.

## Rust conventions

- Follow `rustfmt.toml` and format Rust through `just fmt`.
- Clippy must pass with warnings denied.
- Public library items require rustdoc documentation.
- Avoid `unsafe`.
- Use named format placeholders instead of positional `{}` arguments.
- Prefer `#[expect]` with a reason over `#[allow]` for local lint overrides.
- Keep dependency entries and feature definitions alphabetically sorted.
- Use bare, minimal dependency versions in `Cargo.toml`.

## Command interface

Use [`just`](https://just.systems) recipes when one exists. Do not bypass a
recipe with an ad hoc `cargo` or tool command. If a recurring task has no
recipe, add a focused recipe before using it.

Run `just` to list every command. The primary recipes are:

```sh
just build
just release
just test
just coverage
just fmt
just fmt_check
just lint "-- -D warnings"
just doc
just deny
just zizmor
just actionlint
just shellcheck
just scan_secrets
just check
just setup_githooks
just changelog_preview <version>
just changelog <version>
just bump_version <version>
just test_bump_version
just build_musl <target>
just build_release <target>
```

`just check` is the required local quality gate. It runs formatting checks,
Clippy with warnings denied, rustdoc with warnings denied, cargo-deny, and the
test suite.

Integration tests spawn the built binary inside a git repository and need a
git identity; the tests configure one in the temporary repository themselves.

When invoking compilation or test commands from the CLI, never request
parallelism greater than eight.

## Required tools

The local recipes expect:

- Rust and rustup (the pinned toolchain installs itself).
- just.
- dprint 0.56.1 and nightly rustfmt.
- cargo-deny.
- TruffleHog.
- git-cliff.
- cargo-llvm-cov for coverage.
- zizmor and actionlint for workflow changes.
- shellcheck for shell script changes.
- Docker for `just build_musl`.

If a required tool is unavailable, report it explicitly. Do not claim its check
passed or silently replace the repository command with a weaker check.

## Documentation

- Write task-oriented documentation with runnable examples.
- Use ATX headings and fenced code blocks with language identifiers.
- Keep Markdown lines readable, tables aligned, and files terminated by one
  newline.
- Run `fmt-md-tables -i <file>` after editing a Markdown file that contains a
  table.

## GitHub Actions

- Run `just zizmor` and `just actionlint` after every workflow change until
  both exit successfully with no findings.
- Pin every external action to the full commit SHA of its latest stable release
  and record the exact matching tag in a trailing comment.
- Declare explicit least-privilege permissions.
- Set `persist-credentials: false` on checkout steps unless later authenticated
  Git operations are explicitly required.
- Never interpolate attacker-controlled GitHub expressions directly into a
  shell script. Pass values through `env` and quote the shell variable.

## Git and releases

- Use Conventional Commits with an imperative, lower-case description.
- Do not add agent attribution, session links, or agent `Co-Authored-By` lines.
- Releases run through `.github/workflows/release.yml` (`workflow_dispatch`):
  it bumps the version, regenerates `CHANGELOG.md`, builds every target,
  creates the GitHub release, publishes to crates.io through trusted
  publishing, and updates the Homebrew formula in `veeso/homebrew-cucco`.
  Always run it with `dry_run = true` first.
- Preview release notes locally with `just changelog_preview <version>`.
