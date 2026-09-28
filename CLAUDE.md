# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

`AGENTS.md` holds the full agent contract for this repository. Read it before
making changes; this file summarizes the parts needed most often.

## Commands

Every task runs through a [`just`](https://just.systems) recipe. Do not bypass a
recipe with an ad hoc `cargo` or tool command. Run `just` to list all recipes.

```sh
just build                 # cargo build --all-targets
just test                  # cargo test --all-targets, then --doc
just test "hook"           # run the tests whose name contains "hook"
just coverage              # cargo llvm-cov, writes lcov.info
just fmt                   # dprint fmt (Markdown, Rust, TOML, YAML)
just fmt_check             # dprint check
just lint "-- -D warnings" # alias of `just clippy`
just doc                   # cargo doc with RUSTDOCFLAGS="-D warnings"
just deny                  # cargo deny check
just zizmor                # audit .github/workflows
just actionlint            # lint .github/workflows
just shellcheck            # lint tracked shell scripts
just check                 # the full local quality gate
just setup_githooks        # point core.hooksPath at .githooks
just changelog_preview 3.5.0
just build_musl x86_64-unknown-linux-musl
```

`just check` is the required gate before declaring work done. If a required
tool is missing, say so. Never claim a check passed or swap in a weaker command.

Never request build or test parallelism above eight from the CLI.

## Architecture

- **Flow.** `src/bin/main.rs` parses arguments with clap, discovers the git
  repository with `gix`, checks the staging area (`status.rs`), loads the
  layered config (`config.rs`), detects scopes from the staged diff
  (`scope.rs`), runs the interactive prompts (`questions.rs`), extracts the
  answers (`answers.rs`), and either prints the message (`--stdout`), writes
  `COMMIT_EDITMSG` (`--hook`), or commits (`commit.rs`).
- **Commit and hooks.** `commit::commit` stages every change itself when
  `--all` is set (`git add -A` semantics through gix), then runs the
  repository `pre-commit` hook, creates the commit through cocogitto, and
  runs `post-commit`. `--no-verify` skips both hooks. A failing `pre-commit`
  aborts; a failing `post-commit` only warns.
- **Config.** Defaults are embedded from `meta/config/default.toml`. Overrides
  come from `<config dir>/cucco/config.toml`, `.cucco.toml` in the working
  directory, `--config <file>`, then CLI flags.
- **Tests.** Unit tests live next to the code. `tests/integration.rs` drives
  the binary through a PTY with `rexpect` on Unix and with `assert_cmd`
  everywhere; hook tests install real hook scripts into `.git/hooks`.
- **Three gates run the same recipes.** `just check` locally,
  `.githooks/pre-commit` on commit, and `.github/workflows/ci.yml` in CI.
- **Release path.** `.github/workflows/release.yml` is `workflow_dispatch`
  only; `dist/release/` holds the version bump and static musl build scripts;
  `install.sh`/`install.ps1` are served from GitHub Pages by
  `.github/workflows/pages.yml`; the Homebrew formula is generated into
  `veeso/homebrew-cucco`.

## Conventions

- Toolchain pinned to Rust 1.98.1, edition 2021. Keep `rust-toolchain.toml`
  and `package.rust-version` in sync.
- Format with `just fmt` only (dprint delegates `.rs` files to nightly rustfmt
  with `group_imports` and `imports_granularity` from `rustfmt.toml`).
- Conventional Commits, imperative and lower-case. No agent attribution,
  session links, or agent `Co-Authored-By` lines.
- Keep `Cargo.toml` dependency and feature entries alphabetically sorted, with
  bare minimal versions.
- Do not stage planning state. `docs/superpowers/`, `.superpowers/`, and
  `.claude/plans/` are gitignored and dprint-excluded.
- After editing a Markdown file that contains a table, run
  `fmt-md-tables -i <file>`.
- After any change under `.github/workflows/`, run `just zizmor` and
  `just actionlint` until both exit clean. Pin actions to a full commit SHA
  with the matching tag in a trailing comment, declare least-privilege
  permissions, and set `persist-credentials: false` on checkout.
