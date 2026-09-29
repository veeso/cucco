# cucco

<p align="center">~ An interactive CLI for creating conventional commits ~</p>

<p align="center">Developed by <a href="https://veeso.me/" target="_blank">@veeso</a></p>

<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-teal.svg" alt="License-MIT" /></a>
  <a href="https://github.com/veeso/cucco/stargazers"><img src="https://img.shields.io/github/stars/veeso/cucco.svg?style=plain" alt="Repo stars" /></a>
  <a href="https://crates.io/crates/cucco"><img src="https://img.shields.io/crates/d/cucco.svg" alt="Downloads counter" /></a>
  <a href="https://crates.io/crates/cucco"><img src="https://img.shields.io/crates/v/cucco.svg" alt="Latest version" /></a>
  <a href="https://www.conventionalcommits.org"><img src="https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white" alt="Conventional Commits" /></a>
  <a href="https://ko-fi.com/veeso"><img src="https://img.shields.io/badge/donate-ko--fi-red" alt="Ko-fi" /></a>
</p>

<p align="center">
  <a href="https://github.com/veeso/cucco/actions/workflows/ci.yml"><img src="https://github.com/veeso/cucco/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/veeso/cucco/actions/workflows/trufflehog.yml"><img src="https://github.com/veeso/cucco/actions/workflows/trufflehog.yml/badge.svg" alt="TruffleHog" /></a>
  <a href="https://github.com/veeso/cucco/actions/workflows/zizmor.yml"><img src="https://github.com/veeso/cucco/actions/workflows/zizmor.yml/badge.svg" alt="zizmor" /></a>
  <a href="https://coveralls.io/github/veeso/cucco"><img src="https://coveralls.io/repos/github/veeso/cucco/badge.svg" alt="Coveralls" /></a>
</p>

---

## About cucco 🐔

cucco is an interactive CLI for creating
[conventional commits](https://www.conventionalcommits.org/en/v1.0.0/), built
on [cocogitto](https://github.com/oknozor/cocogitto) and inspired by
[cz-cli](https://github.com/commitizen/cz-cli).

cucco is a fork of [koji](https://github.com/cococonscious/koji) by Danny Tatom
and Finley Thomalla. It keeps everything koji does and adds git hooks
execution, so that `pre-commit` and `post-commit` hooks run exactly as they do
with `git commit`.

## Features

- Create conventional commits with ease
- Run the repository `pre-commit` and `post-commit` hooks, with `--no-verify`
  to skip them
- Use alongside [cocogitto](https://github.com/oknozor/cocogitto) for automatic
  versioning, changelog generation, and more
- Use emoji 👋 (or, [shortcodes](https://github.com/ikatyang/emoji-cheat-sheet))
- Autocomplete for commit scope, with automatic scope detection from staged
  files
- Run as a git hook
- Custom commit types

## Installation

### Linux and macOS

```sh
curl -sSLf https://veeso.github.io/cucco/install.sh | sh
```

The script uses Homebrew when it is available and falls back to downloading
the release binary, verifying its SHA-256 checksum. Pass `--version X.Y.Z` for
a specific release, `--yes` to skip the prompt, or set `BIN_DIR` to change the
install directory (default `/usr/local/bin`).

### Windows

```powershell
irm https://veeso.github.io/cucco/install.ps1 | iex
```

### Homebrew

```sh
brew install veeso/cucco/cucco
```

Be sure to have [git](https://git-scm.com/) installed first.

## Usage

The basic way to use cucco is as a replacement for `git commit`, enforcing the
[conventional commit](https://www.conventionalcommits.org/en/v1.0.0/) standard
by writing your commit through an interactive prompt.

```sh
# Do some work
cd dev/cucco
git add README.md

# Commit your work
cucco
```

See `cucco --help` for more options.

Like `git commit`, cucco can stage changes for you:

```sh
# Stage modified and deleted tracked files, like `git commit -a`
cucco -a

# Stage everything, untracked files included, like `git add -A`
cucco -A
```

`-a`/`--all` never stages untracked files. `-A`/`--add-all` stages them, but
not ignored files. The two flags cannot be combined, and neither can be used
with `--hook` or `--stdout`.

Use `cucco completions <SHELL>` to generate completion scripts for your shell.

### Git hooks

When you commit with cucco, the repository's `pre-commit` and `post-commit`
hooks run around the commit, mirroring `git commit` behavior. If a
`pre-commit` hook exits non-zero, the commit is aborted. A failing
`post-commit` hook is reported as a warning and does not abort, matching
`git commit`.

Pass `--no-verify` to bypass both hooks:

```sh
cucco --no-verify
```

When `--all` or `--add-all` is set, cucco stages the changes before
`pre-commit` runs, regardless of whether hooks are enabled.

## Using as a git hook

An alternative way to use cucco is as a
[git hook](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks), running
it any time you run `git commit`.

### Manually

Update `.git/hooks/prepare-commit-msg` with the following code:

```sh
#!/bin/bash
exec < /dev/tty && cucco --hook || true
```

### [husky](https://github.com/typicode/husky)

```sh
npx husky add .husky/prepare-commit-msg "exec < /dev/tty && cucco --hook || true"
```

### [prek](https://github.com/j178/prek) / [pre-commit](https://pre-commit.com/)

Add the `prepare-commit-msg` hook type and the cucco hook to
`.pre-commit-config.yaml`:

```yaml
default_install_hook_types: [prepare-commit-msg, ...]
repos:
  - repo: local
    hooks:
      - id: prepare-msg
        name: prepare commit message
        entry: bash -c "exec < /dev/tty && cucco --hook || true"
        language: system
        stages: [prepare-commit-msg]
```

Similar should work for any hook runner, just make sure you're using it with
the `prepare-commit-msg` hook.

When using it as a hook, any message passed to `git commit -m` will be used for
the commit summary. Writing your commit as a conventional commit, e.g.
`git commit -m "feat(space): delete some stars"`, will bypass cucco altogether.

## Configuration

Config values are prioritized in the following order:

- Passed in as arguments (see: `cucco --help`)
- Read from file passed in via `--config`
- `.cucco.toml` in the working directory
- Unix/Redox:
  - `$XDG_CONFIG_HOME/cucco/config.toml`
  - `~/.config/cucco/config.toml`
- Windows:
  - `%USERPROFILE%\AppData\Roaming\cucco\config.toml`
- The [default](meta/config/default.toml) config

### Options

#### `autocomplete`

- Type: `bool`
- Optional: `true`
- Description: Enables auto-complete for scope prompt via scanning commit history.

```toml
autocomplete = true
```

#### `breaking-changes`

- Type: `bool`
- Optional: `true`
- Description: Enables breaking change prompt.

```toml
breaking_changes = true
```

#### `commit-types`

- Type: `Vec<CommitType>`
- Optional: `true`
- Description: A list of commit types to use instead of the [default](meta/config/default.toml).

```toml
[[commit_types]]
name = "feat"
emoji = "✨"
description = "A new feature"
```

#### `emoji`

- Type: `bool`
- Optional: `true`
- Description: Prepend the commit summary with relevant emoji based on commit type.

```toml
emoji = true
```

#### `issues`

- Type: `bool`
- Optional: `true`
- Description: Enables issue prompt, which will append a reference to an issue in the commit body.

```toml
issues = true
```

#### `force_config_scopes`

- Type: `bool`
- Optional: `true`
- Description: When `true`, the scope prompt becomes a selection list restricted to the configured `commit_scopes`. If a single scope is auto-detected from staged changes it is pre-selected automatically.

```toml
force_config_scopes = true
```

#### `allow_empty_scope`

- Type: `bool`
- Optional: `true`
- Description: When `false`, a scope is required and cannot be left blank. Pressing `<esc>` without a detected scope will abort the commit. Defaults to `true`.

```toml
allow_empty_scope = false
```

#### `commit_scopes`

- Type: `Vec<CommitScope>`
- Optional: `true`
- Description: A list of named commit scopes. Each scope can carry a human-readable `description` (for CLI use), path `patterns` for automatic detection, and an `ast_grep` rule for content-based detection.

_For automatically assigning scopes based on the context of the change_

```toml
[[commit_scopes]]
name = "core"
description = "Changes to the core library"
patterns = "/crates/core/**/*.rs"

[[commit_scopes]]
name = "build"
patterns = ["^/build\\.rs$", "/justfile"]

[[commit_scopes]]
name = "test"
description = "Test-only changes"
patterns = "/tests/**"

# In reference the above scope \/ (TOML is weird)
[commit_scopes.ast_grep]
language = "Rust"
files = ["**/*.rs"]
rule = { kind = "function_item", has = { stopBy = "end", pattern = "#[test]" } }
```

**`patterns`** -- one or more regex strings matched against staged file paths
(prefixed with `/`).

**`ast_grep`** -- an [ast-grep](https://ast-grep.github.io/) rule. When any
staged file matches both the `files` filter and the structural rule, this scope
is pre-assigned. Requires the `ast-grep` feature (included by default).
As a warning: the rule is read from the **staged blob only**, so renamed and
deleted paths are matched against their pre-change content.

## Development

Every task runs through a [`just`](https://just.systems) recipe. Run `just` to
list them. `just check` is the local quality gate (formatting, Clippy with
warnings denied, rustdoc, cargo-deny and the test suite). Install the tracked
git hooks with `just setup_githooks`. See [CONTRIBUTING.md](CONTRIBUTING.md)
and [AGENTS.md](AGENTS.md).

## Credits

cucco is a fork of [koji](https://github.com/cococonscious/koji), created by
Danny Tatom and maintained by Finley Thomalla, released under the MIT license.
The original copyright notices are preserved in [LICENSE](LICENSE). Thanks to
the koji contributors for the foundation this project builds on.

## License

Licensed under the MIT license. See [LICENSE](LICENSE).
