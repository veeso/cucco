# Changelog

All notable changes to this project are documented in this file.


## 4.1.0

Released on 2026-10-06

### Added


- fill the summary suggestion with tab or right arrow (#6)
> The summary prompt showed the previous commit summary as a greyed-out placeholder that could not be accepted. The prompt now runs on the crate's own crossterm text prompt in single-line mode: while the input is empty, tab or the right arrow copy the suggestion into it, and a help line advertises both keys.
## 4.0.0

Released on 2026-09-29

### Breaking changes
- add ability to skip breaking changes and issues questions
> add ability to skip breaking changes and issues questions
- stage files, better args, deps (#102)
> Removed short argument alias of breaking_changes,
autocomplete, emoji, and issues.
- rename project to cucco
> .koji.toml and koji/config.toml are no longer read; rename them to .cucco.toml and cucco/config.toml.
- migrate koji fork to cucco (#1)
> .koji.toml and koji/config.toml are no longer read; rename them to .cucco.toml and cucco/config.toml.

* docs(license): add fork copyright notice

Keep the original koji copyright lines and add the cucco maintainer as a further copyright holder under the same MIT license.

* build: adopt just, dprint, cargo-deny and git-cliff tooling

Replace the pre-commit framework with a tracked git hook, pin the toolchain to 1.98.1, add the dependency policy, changelog template and command layer used across veeso projects, and turn on the shared rustc lints.

* style: format the tree with dprint and nightly rustfmt

* docs: add agent guidance, AI policy and contribution templates

Import AI_POLICY.md and the pull request template, add AGENTS.md, CLAUDE.md, CONTRIBUTING.md, the funding file and veeso-style issue templates.

* ci: replace koji pipelines with veeso workflows

Drop release-please, tarpaulin, codecov and actionlint-as-a-job in favour of the CI, TruffleHog and zizmor workflows shared across veeso projects. Coverage is produced by cargo-llvm-cov and uploaded to Coveralls.

* ci(release): add release workflow with static binaries, installers and homebrew tap

A workflow_dispatch release bumps the version, regenerates the changelog with git-cliff, builds static musl, macOS and Windows binaries, creates the GitHub release with SHA-256 checksums and installer scripts, publishes to crates.io through trusted publishing and rewrites the formula in veeso/homebrew-cucco. GitHub Pages serves install.sh and install.ps1.

* chore: remove renovate
- make -a stage tracked changes only and add -A to stage everything (#2)
> -a/--all no longer stages untracked files. Use -A/--add-all to stage them.

### Added


- initial commit
- add validation to questions
- add support for commit types with no emoji
- allow passing path to a config file
- add option to run as git hook
> this writes the commit message to COMMIT_EDITMSG instead of creating a commit
- add optional autocomplete for scope prompt
- add support for emoji shortcodes
- use message passed in via -m flag
- return early if commit message is already conventional
- **config:** better config handling
> - use standard config locations (closes #49)
> - add options for emoji and autocomplete in config file (closes #50)
- finish cleaning up config
- Breaking: add ability to skip breaking changes and issues questions
- allow signing commits
- type filtering, multi-line body support (#99)
> Switches requestty with inquire, which is still active and supports
> filtering by typing out of the box.
> Pinned cocogitto more precisely to prevent lower versions and updated
> all locked dependencies.
> Allow adding line breaks in the body with '\n'.
- breaking change footers (#101)
> Adds a prompt, and the functionality, for adding a breaking change
> footer describing the breaking changes in detail, as described in the
> Conventional Commits specification. I decided against using a dash (-)
> between "BREAKING" and "CHANGE" as I fear that not every tool supports
> it, but it may be configurable in the future.
> Also fixes the hint of the commit body prompt having a double comma.
- Breaking: stage files, better args, deps (#102)
> Adds a new command argument "--all|-a" with the same functionality as
> git's "commit -a". Revamped the arguments a little, changing the short
> aliases to avoid misunderstandings with git's equivalents and upgraded
> the Cargo dependencies.
- add shell completions subcommand (#106)
> Adds a subcommand "completions" which can generate shell completions for
> a few supported shells, namely bash, zsh and Nushell.
> You can use it like this: `koji completions <SHELL> >
> <CUSTOM_COMPLETIONS_DIR>/_koji`.
> I consider this feature necessary for eventually publishing koji to
> other package registries, such as the AUR.
- git-like -C argument, integration tests (#103)
> Added an argument almost equivalent to Git's "-C", mainly for the
> purpose of making integration tests isolated. This feature depends on
> cocogitto/cocogitto#428, and therefore v6.2.0, as the commit will
> otherwise still be made in the current working directory.
> This commit reduces code test coverage as many skips were removed.
- **args:** adding 'stdout' flag (#129)
> Added a '--stdout' flag to output the message to stdout instead of
> continuing on with the git workflow. Allows koji to be used with non-git
> workflows.
- **config:** allow reading configuration from xdg directories (#155)
> On MacOS the "default" configuration directories do not follow the XDG
> standards.
> This change allows the use of those standards on a mac if the
> environment variable for XDG_CONFIG_HOME has been set.
- replace git2 with gix (#174)
> Keeping cocogitto for the commits for now but uses Gix for traversal, etc., and at some point, when Gix is ready, replace cocogitto with it.
- staging area warn and error (#182)
> Warns when not all files in the index have been added to the staging
> area and throws and error if no files have been added.
- confirmation prompt (#181)
> Always prints out the built commit message and prompts for confirmation,
> unless --stdout or --yes is passed
> 
> Resolves one of the suggested improvements in #128
- configurable scopes with semantic matching (#179)
> In this PR I added scope configuration, which allows you to assign rules
> for the scopes to be added when you use koji, under specific rules.
> 
> The ruleset is simple:
> - Either you regex match over the files in the change.
> - Or you more creatively setup an
> [ast-grep](https://ast-grep.github.io/) match, where you can configure
> based on the patterns in your code (example, a test-case change.)
- **commit:** run pre-commit and post-commit hooks
> Wraps the cocogitto commit with `pre-commit` and `post-commit` git
> hooks, matching `git commit` behavior. A failing `pre-commit` hook
> aborts the commit; a failing `post-commit` hook is reported as a
> warning and does not abort, matching `git commit`.
> 
> Staging is always performed by koji (via gix) regardless of whether
> hooks are enabled, so `--all` and `--all --no-verify` produce the
> same staged set: tracked modified/deleted files only, matching the
> documented `--all` flag and `git commit -a` semantics. Cocogitto's
> own staging is bypassed.
> 
> Adds `--no-verify` to skip both hooks (conflicts with `--hook`).
- **commit:** merge git hooks support from feat/git-hooks
> Run the repository pre-commit and post-commit hooks around the commit, add --no-verify to bypass them, and make --all stage every change like git add -A.
- Breaking: rename project to cucco
> The crate, binary and library are now named cucco. The project config file is .cucco.toml and the user config lives under cucco/config.toml.
- Breaking: migrate koji fork to cucco (#1)
> * feat(commit): run pre-commit and post-commit hooks
> 
> Wraps the cocogitto commit with `pre-commit` and `post-commit` git
> hooks, matching `git commit` behavior. A failing `pre-commit` hook
> aborts the commit; a failing `post-commit` hook is reported as a
> warning and does not abort, matching `git commit`.
> 
> Staging is always performed by koji (via gix) regardless of whether
> hooks are enabled, so `--all` and `--all --no-verify` produce the
> same staged set: tracked modified/deleted files only, matching the
> documented `--all` flag and `git commit -a` semantics. Cocogitto's
> own staging is bypassed.
> 
> Adds `--no-verify` to skip both hooks (conflicts with `--hook`).
> 
> * fix(commit): refresh stat and invalidate TREE cache when staging via gix
> 
> `stage_tracked` updates index entries' blob ids in-place but left two
> gix-index pieces stale, which manifested only when cocogitto (libgit2)
> wrote the commit afterwards:
> 
> - The TREE extension still pointed at the pre-edit root tree, so
>   libgit2's `index.write_tree()` returned the cached old tree id —
>   producing a commit whose tree equals its parent's (empty diff).
> - The entry `stat` (mtime/size/ino) was not refreshed from disk, so
>   `git status` reported the file as modified after commit even though
>   index, HEAD and worktree blobs all matched.
> 
> Drop the TREE extension and rewrite `entry.stat` from the worktree
> file's metadata before writing the index.
> 
> * fix(commit): make --all match git add -A and handle missing index
> 
> Stage untracked files (matching documented `git add -A` semantics) and
> gracefully open `.git/index` via `at_or_default` so `koji -a` works in
> a fresh repo before any commit exists.
> 
> * feat!: rename project to cucco
> 
> The crate, binary and library are now named cucco. The project config file is .cucco.toml and the user config lives under cucco/config.toml.
- Breaking: make -a stage tracked changes only and add -A to stage everything (#2)
> The -a/--all flag used to stage every change, untracked files included, which does not match git. It now stages modified and deleted tracked files only, like git commit -a, and fails early when there is no tracked change to commit. The new -A/--add-all flag stages every change, untracked files included, like git add -A. The two flags are mutually exclusive and both conflict with --hook and --stdout.
- support multiline commit descriptions (#3)
> * feat: support multiline commit descriptions
> 
> Enable inquire's Alt+Enter multiline input for commit bodies and breaking-change details. Preserve literal backslash-n text and document the new prompt behavior.
> 
> * fix: enable reliable alt-enter multiline input
> 
> * fix: fix multiline

### Changed


- put config file handling into its own file
- little bit of some cleanup
- remove unnecessary `Error`s from `Result`s
- clean up render_commit_type_choice
- clean up get_amended_body
- replace config loading with a single load_config function
- use const strings for answer keys
- move answer functions to their own file
- restructure app a bit
- clean up main func
- load default commit types from config
- little bit of code cleanup
- move some stuff around
- clean up load_config
- destructure get_extracted_answers return value
- clippy cleanup
- replace linked-hash-map with indexmap
- disable default features of cocogitto
- clean up comments
- move commit code to its own file
- start cleaning up config
- clean up emoji handling
- **config:** clean up config
- deduplicate staging and borrow commit message parts
> Pre-staging for --all and --add-all now reuses commit::stage_changes, replacing scope::stage_tracked_changes. Commit message helpers take references, loops use iterator combinators, and stage_changes and the multiline prompt edit operations gain unit tests.

### Fixed


- make error messages consistent
- fix typo in help
- use git2 to get repo dir
- only early return with message if we're in hook mode
- **autocomplete:** check for empty repo before revwalk (#105)
> `push_head` on Revwalk fails when the repository had just been
> initialized as the commit reference doesn't exist yet. A simple check
> for if the repository is empty fixes this.
- **config:** handled better using config-rs (#120)
- **args:** mutually exclusive hook and all (#121)
> As the "all" argument doesn't do anything in hook mode, we might as well
> just make them mutually exclusive for more clarity.
- xdg crate not supporting windows (#159)
> Fixes the failing Windows builds when releasing
- **deps:** update rust crate gix to 0.80.0 (#180)
> This PR contains the following updates:
> 
> | Package | Type | Update | Change |
> |---|---|---|---|
> | [gix](https://redirect.github.com/GitoxideLabs/gitoxide) |
> dependencies | minor | `0.78.0` → `0.80.0` |
> 
> ---
> 
> ### Release Notes
> 
> <details>
> <summary>GitoxideLabs/gitoxide (gix)</summary>
> 
> ###
> [`v0.80.0`](https://redirect.github.com/GitoxideLabs/gitoxide/releases/tag/gix-v0.80.0):
> gix v0.80.0
> 
> [Compare
> Source](https://redirect.github.com/GitoxideLabs/gitoxide/compare/gix-v0.79.0...gix-v0.80.0)
> 
> ##### Bug Fixes
> 
> - Correctly use `$COMMON_DIR/info/exclude` to make excludes work in
> worktrees.
>   It turns out there is no per-worktree excludes file either.
> - make `status` work despite broken or invalid symlinks.
> Related to
> [gitbutlerapp/gitbutler#12399](https://redirect.github.com/gitbutlerapp/gitbutler/issues/12399)
> 
> ##### Chore (BREAKING)
> 
> - <csr-id-2358b1d250d3d2348210fb61dcb95ebe7aa6314b/> Upgrade `prodash`
> and `crosstermion` to the latest version.
> This will fix the `cargo deny` issue as it brings in a newer `lru`
> crate.
> 
> ##### New Features (BREAKING)
> 
> - encode shallow commit lists as non-empty, and introduced `nonempty` in
> `gix-protocol`
> - model merge-bases as a non-empty type in `gix-revision` and
> `gix-merge`
>   Adapt `gix` accordingly (even though it's nonbreaking).
> 
> ##### Commit Statistics
> 
> - 9 commits contributed to the release over the course of 10 calendar
> days.
> - 12 days passed between releases.
> - 3 commits were understood as
> [conventional](https://www.conventionalcommits.org).
> - 0 issues like '(#ID)' were seen in commit messages
> 
> ##### Commit Details
> 
> <csr-read-only-do-not-edit/>
> 
> <details><summary>view details</summary>
> 
> - **Uncategorized**
> - Merge pull request
> [#&#8203;2440](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2440)
> from GitoxideLabs/improvements
> ([`93f39fb`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/93f39fb))
> - Make `status` work despite broken or invalid symlinks.
> ([`94b35a1`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/94b35a1))
> - Merge pull request
> [#&#8203;2433](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2433)
> from GitoxideLabs/codex/nonempty-rewrite
> ([`29040a8`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/29040a8))
> - Adopt to changes related to the introduction of `nonempty`.
> ([`e033441`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/e033441))
> - Encode shallow commit lists as non-empty, and introduced `nonempty` in
> `gix-protocol`
> ([`78b0a6f`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/78b0a6f))
> - Model merge-bases as a non-empty type in `gix-revision` and
> `gix-merge`
> ([`231fda4`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/231fda4))
> - Merge pull request
> [#&#8203;2377](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2377)
> from cruessler/add-sha-256-to-gix-commitgraph
> ([`228caf7`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/228caf7))
> - Use `GIX_TEST_FIXTURE_HASH` for `gix-commitgraph` and `gix-pack`.
> ([`d51b858`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/d51b858))
> - Merge branch 'release'
> ([`9327b73`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/9327b73))
> 
> </details>
> 
> ###
> [`v0.79.0`](https://redirect.github.com/GitoxideLabs/gitoxide/releases/tag/gix-v0.79.0):
> gix v0.79.0
> 
> [Compare
> Source](https://redirect.github.com/GitoxideLabs/gitoxide/compare/gix-v0.78.0...gix-v0.79.0)
> 
> ##### Bug Fixes
> 
> - Correctly use `$COMMON_DIR/info/exclude` to make excludes work in
> worktrees.
>   It turns out there is no per-worktree excludes file either.
> - Differentiate between `core.bare` being known or not.
>   This allows `Repository::is_bare()` to only make assumptions based
>   on the configuration value, which is what Git does.
> 
> ##### Chore (BREAKING)
> 
> - <csr-id-2358b1d250d3d2348210fb61dcb95ebe7aa6314b/> Upgrade `prodash`
> and `crosstermion` to the latest version.
> This will fix the `cargo deny` issue as it brings in a newer `lru`
> crate.
> 
> ##### Bug Fixes (BREAKING)
> 
> - Improve `Repository::kind()` for better classification
>   Previously it wasn't clear what it really is as it chose the wrong
>   categories.
> 
> ##### New Features (BREAKING)
> 
> - `gix-error` instead of `thiserror`
> - `gix-error` instead of `thiserror`
> 
> ##### Commit Statistics
> 
> - 21 commits contributed to the release over the course of 18 calendar
> days.
> - 18 days passed between releases.
> - 4 commits were understood as
> [conventional](https://www.conventionalcommits.org).
> - 1 unique issue was worked on:
> [#&#8203;2402](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2402)
> 
> ##### Commit Details
> 
> <csr-read-only-do-not-edit/>
> 
> <details><summary>view details</summary>
> 
> -
> **[#&#8203;2402](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2402)**
> - Differentiate between `core.bare` being known or not.
> ([`02fe02b`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/02fe02b))
> - **Uncategorized**
> - Merge pull request
> [#&#8203;2420](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2420)
> from cruessler/remove-imara-diff-0-1-in-gix-blame
> ([`28fbeb8`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/28fbeb8))
> - Merge pull request
> [#&#8203;2400](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2400)
> from GitoxideLabs/gix-error
> ([`e4f016b`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/e4f016b))
> - Address Copilot review
> ([`0b0e9f8`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/0b0e9f8))
> - Refactor
> ([`c7f84c2`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/c7f84c2))
> - `gix-error` instead of `thiserror`
> ([`0128df7`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/0128df7))
> - `gix-error` instead of `thiserror`
> ([`b8059ab`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/b8059ab))
> - Remove feature flag blame-experimental
> ([`44e447b`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/44e447b))
> - Merge pull request
> [#&#8203;2415](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2415)
> from GitoxideLabs/improvements
> ([`5c9e6ae`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/5c9e6ae))
> - Adapt to changes in `gix-testtools`
> ([`fb60c8a`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/fb60c8a))
> - Merge pull request
> [#&#8203;2407](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2407)
> from GitoxideLabs/dependabot/cargo/cargo-fb4135702f
> ([`8bceefb`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/8bceefb))
> - Bump the cargo group with 59 updates
> ([`7ce3c55`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/7ce3c55))
> - Merge pull request
> [#&#8203;2404](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2404)
> from fenhl/patch-1
> ([`aaf0a76`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/aaf0a76))
> - Refactor
> ([`d820846`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/d820846))
> - Clarify that `blocking-http-transport-reqwest` doesn't enable
> `reqwest`'s default features
> ([`c45cdaf`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/c45cdaf))
> - Merge pull request
> [#&#8203;2403](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2403)
> from GitoxideLabs/improvements
> ([`75ac1d0`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/75ac1d0))
> - Improve `Repository::kind()` for better classification
> ([`8ea42ea`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/8ea42ea))
> - Merge pull request
> [#&#8203;2396](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2396)
> from GitoxideLabs/gix-error
> ([`e8612b5`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/e8612b5))
> - Adapt to changes in `gix-error`
> ([`a304f13`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/a304f13))
> - Adapt to changes in `gix-actor`
> ([`b80d026`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/b80d026))
> - Merge pull request
> [#&#8203;2393](https://redirect.github.com/GitoxideLabs/gitoxide/issues/2393)
> from GitoxideLabs/report
> ([`f7d0975`](https://redirect.github.com/GitoxideLabs/gitoxide/commit/f7d0975))
> 
> </details>
> 
> </details>
> 
> ---
> 
> ### Configuration
> 
> 📅 **Schedule**: Branch creation - Between 12:00 AM and 03:59 AM, only on
> Monday ( * 0-3 * * 1 ) (UTC), Automerge - At any time (no schedule
> defined).
> 
> 🚦 **Automerge**: Disabled by config. Please merge this manually once you
> are satisfied.
> 
> ♻ **Rebasing**: Whenever PR is behind base branch, or you tick the
> rebase/retry checkbox.
> 
> 🔕 **Ignore**: Close this PR and you won't be reminded about this update
> again.
> 
> ---
> 
> - [ ] <!-- rebase-check -->If you want to rebase/retry this PR, check
> this box
> 
> ---
> 
> This PR was generated by [Mend Renovate](https://mend.io/renovate/).
> View the [repository job
> log](https://developer.mend.io/github/cococonscious/koji).
> 
> <!--renovate-debug:eyJjcmVhdGVkSW5WZXIiOiI0My4yNi41IiwidXBkYXRlZEluVmVyIjoiNDMuMzYuMiIsInRhcmdldEJyYW5jaCI6Im1haW4iLCJsYWJlbHMiOltdfQ==-->
- **deps:** update rust crate inquire to v0.9.4 (#171)
> This PR contains the following updates:
> 
> | Package | Type | Update | Change |
> |---|---|---|---|
> | [inquire](https://redirect.github.com/mikaelmello/inquire) |
> dependencies | patch | `0.9.1` → `0.9.4` |
> 
> ---
> 
> ### Release Notes
> 
> <details>
> <summary>mikaelmello/inquire (inquire)</summary>
> 
> ###
> [`v0.9.4`](https://redirect.github.com/mikaelmello/inquire/blob/HEAD/CHANGELOG.md#094---2026-02-24)
> 
> [Compare
> Source](https://redirect.github.com/mikaelmello/inquire/compare/v0.9.3...v0.9.4)
> 
> ##### Features
- **deps:** update all non-major dependencies (#169)
> This PR contains the following updates:
> 
> | Package | Type | Update | Change |
> |---|---|---|---|
> | [anyhow](https://redirect.github.com/dtolnay/anyhow) | dependencies |
> patch | `1.0.100` → `1.0.102` |
> | [assert_cmd](https://redirect.github.com/assert-rs/assert_cmd) |
> dev-dependencies | patch | `2.1.1` → `2.1.2` |
> | [clap](https://redirect.github.com/clap-rs/clap) | dependencies |
> patch | `4.5.53` → `4.5.60` |
> | [indexmap](https://redirect.github.com/indexmap-rs/indexmap) |
> dependencies | minor | `2.12.1` → `2.13.0` |
> | [predicates](https://redirect.github.com/assert-rs/predicates-rs) |
> dev-dependencies | patch | `3.1.3` → `3.1.4` |
> | [tempfile](https://stebalien.com/projects/tempfile-rs/)
> ([source](https://redirect.github.com/Stebalien/tempfile)) |
> dev-dependencies | minor | `3.24.0` → `3.26.0` |
> 
> ---
> 
> ### Release Notes
> 
> <details>
> <summary>dtolnay/anyhow (anyhow)</summary>
> 
> ###
> [`v1.0.102`](https://redirect.github.com/dtolnay/anyhow/releases/tag/1.0.102)
> 
> [Compare
> Source](https://redirect.github.com/dtolnay/anyhow/compare/1.0.101...1.0.102)
> 
> - Remove backtrace dependency
> ([#&#8203;438](https://redirect.github.com/dtolnay/anyhow/issues/438),
> [#&#8203;439](https://redirect.github.com/dtolnay/anyhow/issues/439),
> [#&#8203;440](https://redirect.github.com/dtolnay/anyhow/issues/440),
> [#&#8203;441](https://redirect.github.com/dtolnay/anyhow/issues/441),
> [#&#8203;442](https://redirect.github.com/dtolnay/anyhow/issues/442))
> 
> ###
> [`v1.0.101`](https://redirect.github.com/dtolnay/anyhow/releases/tag/1.0.101)
> 
> [Compare
> Source](https://redirect.github.com/dtolnay/anyhow/compare/1.0.100...1.0.101)
> 
> - Add #\[inline] to anyhow::Ok helper
> ([#&#8203;437](https://redirect.github.com/dtolnay/anyhow/issues/437),
> thanks [@&#8203;Ibitier](https://redirect.github.com/Ibitier))
> 
> </details>
> 
> <details>
> <summary>assert-rs/assert_cmd (assert_cmd)</summary>
> 
> ###
> [`v2.1.2`](https://redirect.github.com/assert-rs/assert_cmd/blob/HEAD/CHANGELOG.md#212---2026-01-09)
> 
> [Compare
> Source](https://redirect.github.com/assert-rs/assert_cmd/compare/v2.1.1...v2.1.2)
> 
> ##### Fixes
> 
> - Add `#[must_use]` to help catch missing assertions
> 
> </details>
> 
> <details>
> <summary>clap-rs/clap (clap)</summary>
> 
> ###
> [`v4.5.60`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4560---2026-02-19)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.59...v4.5.60)
> 
> ##### Fixes
> 
> - *(help)* Quote empty default values, possible values
> 
> ###
> [`v4.5.59`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4559---2026-02-16)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.58...v4.5.59)
> 
> ##### Fixes
> 
> - `Command::ignore_errors` no longer masks help/version on subcommands
> 
> ###
> [`v4.5.58`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4558---2026-02-11)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.57...v4.5.58)
> 
> ###
> [`v4.5.57`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4557---2026-02-03)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.56...v4.5.57)
> 
> ##### Fixes
> 
> - Regression from 4.5.55 where having an argument with
> `.value_terminator("--")` caused problems with an argument with
> `.last(true)`
> 
> ###
> [`v4.5.56`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4556---2026-01-29)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.55...v4.5.56)
> 
> ##### Fixes
> 
> - On conflict error, don't show conflicting arguments in the usage
> 
> ###
> [`v4.5.55`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4555---2026-01-27)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.54...v4.5.55)
> 
> ##### Fixes
> 
> - Fix inconsistency in precedence between positionals with a
> `value_terminator("--")` and escapes (`--`) where `./foo -- bar` means
> the first arg is empty, rather than escaping future args
> 
> ###
> [`v4.5.54`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4554---2026-01-02)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.53...v4.5.54)
> 
> ##### Fixes
> 
> - *(help)* Move `[default]` to its own paragraph when
> `PossibleValue::help` is present in `--help`
> 
> </details>
> 
> <details>
> <summary>indexmap-rs/indexmap (indexmap)</summary>
> 
> ###
> [`v2.13.0`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2130-2026-01-07)
> 
> [Compare
> Source](https://redirect.github.com/indexmap-rs/indexmap/compare/2.12.1...2.13.0)
> 
> - Implemented `Clone` for `IntoKeys` and `IntoValues`.
> - Added `map::Slice::split_at_checked` and `split_at_mut_checked`.
> - Added `set::Slice::split_at_checked`.
> 
> </details>
> 
> <details>
> <summary>assert-rs/predicates-rs (predicates)</summary>
> 
> ###
> [`v3.1.4`](https://redirect.github.com/assert-rs/predicates-rs/blob/HEAD/CHANGELOG.md#314---2026-02-11)
> 
> [Compare
> Source](https://redirect.github.com/assert-rs/predicates-rs/compare/v3.1.3...v3.1.4)
> 
> - Make `BoxPredicate::find_case` use the inner `find_case`
> implementation
> 
> </details>
> 
> <details>
> <summary>Stebalien/tempfile (tempfile)</summary>
> 
> ###
> [`v3.26.0`](https://redirect.github.com/Stebalien/tempfile/blob/HEAD/CHANGELOG.md#3260)
> 
> - Support `NamedTempFile::persist` on RedoxOS
> ([#&#8203;393](https://redirect.github.com/Stebalien/tempfile/issues/393))
> (thanks to
> [@&#8203;Andy-Python-Programmer](https://redirect.github.com/Andy-Python-Programmer)).
> 
> ###
> [`v3.25.0`](https://redirect.github.com/Stebalien/tempfile/blob/HEAD/CHANGELOG.md#3250)
> 
> - Allow `getrandom` 0.4.x while retaining support for `getrandom` 0.3.x.
> 
> </details>
> 
> ---
> 
> ### Configuration
> 
> 📅 **Schedule**: Branch creation - Between 12:00 AM and 03:59 AM, only on
> Monday ( * 0-3 * * 1 ) (UTC), Automerge - At any time (no schedule
> defined).
> 
> 🚦 **Automerge**: Disabled by config. Please merge this manually once you
> are satisfied.
> 
> ♻ **Rebasing**: Whenever PR is behind base branch, or you tick the
> rebase/retry checkbox.
> 
> 👻 **Immortal**: This PR will be recreated if closed unmerged. Get
> [config
> help](https://redirect.github.com/renovatebot/renovate/discussions) if
> that's undesired.
> 
> ---
> 
> - [ ] <!-- rebase-check -->If you want to rebase/retry this PR, check
> this box
> 
> ---
> 
> This PR was generated by [Mend Renovate](https://mend.io/renovate/).
> View the [repository job
> log](https://developer.mend.io/github/cococonscious/koji).
> 
> <!--renovate-debug:eyJjcmVhdGVkSW5WZXIiOiI0Mi41OS4wIiwidXBkYXRlZEluVmVyIjoiNDMuMzYuMiIsInRhcmdldEJyYW5jaCI6Im1haW4iLCJsYWJlbHMiOltdfQ==-->
- **deps:** pin and upgrade deps (cargo & gh actions)
- **commit:** refresh stat and invalidate TREE cache when staging via gix
> `stage_tracked` updates index entries' blob ids in-place but left two
> gix-index pieces stale, which manifested only when cocogitto (libgit2)
> wrote the commit afterwards:
> 
> - The TREE extension still pointed at the pre-edit root tree, so
>   libgit2's `index.write_tree()` returned the cached old tree id —
>   producing a commit whose tree equals its parent's (empty diff).
> - The entry `stat` (mtime/size/ino) was not refreshed from disk, so
>   `git status` reported the file as modified after commit even though
>   index, HEAD and worktree blobs all matched.
> 
> Drop the TREE extension and rewrite `entry.stat` from the worktree
> file's metadata before writing the index.
- **commit:** make --all match git add -A and handle missing index
> Stage untracked files (matching documented `git add -A` semantics) and
> gracefully open `.git/index` via `at_or_default` so `koji -a` works in
> a fresh repo before any commit exists.

### Build


- **deps-dev :** get derive as a feature from serde
- make release bin smaller
- **deps:** update deps
- **deps:** update clap to 3.0.0-rc.8
- **deps:** update requestty to 0.2.1
- **deps:** update clap to 3.0.0
- **deps:** update rust to 1.58.0
- **deps:** update rust to 1.58.1
- **cog:** add post bump hooks
- **deps:** update all the deps
- **deps:** update rust to 1.61.0
- remove rust-toolchain file
- **deps:** update all the deps
- **deps:** update all the deps
- **deps:** update clap and emojis
- **deps:** update deps, remove patch constraints
- **deps:** update toml to 0.8
- **deps:** update git2 to 0.18
- **deps:** update indexmap to 2.1
- **deps:** update cocogitto to 6.0
- use devenv
- use asdf
- adopt just, dprint, cargo-deny and git-cliff tooling
> Replace the pre-commit framework with a tracked git hook, pin the toolchain to 1.98.1, add the dependency policy, changelog template and command layer used across veeso projects, and turn on the shared rustc lints.
- migrate to rust edition 2024
> Integration tests pass XDG_CONFIG_HOME to each spawned command instead of mutating the shared process environment, which is unsafe in edition 2024 and raced across parallel tests.

### Deps


- non-default vendored-openssl feature, pin cocogitto (#98)
> Makes the change introduced in 70d5b2e optional and only be used in
> Windows builds. Pinned cocogitto because it doesn't respect semantic
> versioning yet.
- update all non-major dependencies (#130)
> This PR contains the following updates:
> 
> | Package | Type | Update | Change |
> |---|---|---|---|
> | [clap](https://redirect.github.com/clap-rs/clap) | dependencies |
> patch | `4.5.23` -> `4.5.27` |
> | [indexmap](https://redirect.github.com/indexmap-rs/indexmap) |
> dependencies | patch | `2.7.0` -> `2.7.1` |
> 
> ---
> 
> ### Release Notes
> 
> <details>
> <summary>clap-rs/clap (clap)</summary>
> 
> ###
> [`v4.5.27`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4527---2025-01-20)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.26...v4.5.27)
> 
> ##### Documentation
> 
> -   Iterate on tutorials and reference based on feedback
> 
> ###
> [`v4.5.26`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4526---2025-01-09)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.25...v4.5.26)
> 
> ##### Fixes
> 
> -   *(error)* Reduce binary size with the `suggestions` feature
> 
> ###
> [`v4.5.25`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4525---2025-01-09)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.24...v4.5.25)
> 
> ##### Fixes
> 
> -   *(help)* Reduce binary size
> 
> ###
> [`v4.5.24`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4524---2025-01-07)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.23...v4.5.24)
> 
> ##### Fixes
> 
> - *(parser)* Correctly handle defaults with `ignore_errors(true)` and
> when a suggestion is provided for an unknown argument
> 
> </details>
> 
> <details>
> <summary>indexmap-rs/indexmap (indexmap)</summary>
> 
> ###
> [`v2.7.1`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#271-2025-01-19)
> 
> [Compare
> Source](https://redirect.github.com/indexmap-rs/indexmap/compare/2.7.0...2.7.1)
> 
> -   Added `#[track_caller]` to functions that may panic.
> -   Improved memory reservation for `insert_entry`.
> 
> </details>
> 
> ---
> 
> ### Configuration
> 
> 📅 **Schedule**: Branch creation - "* 0-3 * * 1" (UTC), Automerge - At
> any time (no schedule defined).
> 
> 🚦 **Automerge**: Disabled by config. Please merge this manually once you
> are satisfied.
> 
> ♻ **Rebasing**: Whenever PR becomes conflicted, or you tick the
> rebase/retry checkbox.
> 
> 👻 **Immortal**: This PR will be recreated if closed unmerged. Get
> [config
> help](https://redirect.github.com/renovatebot/renovate/discussions) if
> that's undesired.
> 
> ---
> 
> - [x] <!-- rebase-check -->If you want to rebase/retry this PR, check
> this box
> 
> ---
> 
> This PR was generated by [Mend Renovate](https://mend.io/renovate/).
> View the [repository job
> log](https://developer.mend.io/github/cococonscious/koji).
> 
> <!--renovate-debug:eyJjcmVhdGVkSW5WZXIiOiIzOS45Mi4wIiwidXBkYXRlZEluVmVyIjoiMzkuMTA3LjAiLCJ0YXJnZXRCcmFuY2giOiJtYWluIiwibGFiZWxzIjpbXX0=-->
- update all non-major dependencies (#136)
> This PR contains the following updates:
> 
> | Package | Type | Update | Change |
> |---|---|---|---|
> | [clap](https://redirect.github.com/clap-rs/clap) | dependencies |
> patch | `4.5.27` -> `4.5.30` |
> | [tempfile](https://stebalien.com/projects/tempfile-rs/)
> ([source](https://redirect.github.com/Stebalien/tempfile)) |
> dev-dependencies | minor | `3.15.0` -> `3.17.1` |
> 
> ---
> 
> ### Release Notes
> 
> <details>
> <summary>clap-rs/clap (clap)</summary>
> 
> ###
> [`v4.5.30`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.5.29...clap_complete-v4.5.30)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.29...v4.5.30)
> 
> ###
> [`v4.5.29`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4529---2025-02-11)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.28...v4.5.29)
> 
> ##### Fixes
> 
> - Change `ArgMatches::args_present` so not-present flags are considered
> not-present (matching the documentation)
> 
> ###
> [`v4.5.28`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#4528---2025-02-03)
> 
> [Compare
> Source](https://redirect.github.com/clap-rs/clap/compare/v4.5.27...v4.5.28)
> 
> ##### Features
> 
> - *(derive)* Unstable support for full markdown syntax for doc comments,
> enabled with `unstable-markdown`
> 
> </details>
> 
> <details>
> <summary>Stebalien/tempfile (tempfile)</summary>
> 
> ###
> [`v3.17.1`](https://redirect.github.com/Stebalien/tempfile/blob/HEAD/CHANGELOG.md#3171)
> 
> [Compare
> Source](https://redirect.github.com/Stebalien/tempfile/compare/v3.17.0...v3.17.1)
> 
> - Fix build with `windows-sys` 0.52. Unfortunately, we have no CI for
> older `windows-sys` versions at the moment...
> 
> ###
> [`v3.17.0`](https://redirect.github.com/Stebalien/tempfile/blob/HEAD/CHANGELOG.md#3170)
> 
> [Compare
> Source](https://redirect.github.com/Stebalien/tempfile/compare/v3.16.0...v3.17.0)
> 
> - Make sure to use absolute paths in when creating unnamed temporary
> files (avoids a small race in the "immediate unlink" logic) and in
> `Builder::make_in` (when creating temporary files of arbitrary types).
> - Prevent a theoretical crash that could (maybe) happen when a temporary
> file is created from a drop function run in a TLS destructor. Nobody has
> actually reported a case of this happening in practice and I have been
> unable to create this scenario in a test.
> - When reseeding with `getrandom`, use platform (e.g., CPU) specific
> randomness sources where possible.
> -   Clarify some documentation.
> - Unlink unnamed temporary files on windows *immediately* when possible
> instead of waiting for the handle to be closed. We open files with
> "Unix" semantics, so this is generally possible.
> 
> ###
> [`v3.16.0`](https://redirect.github.com/Stebalien/tempfile/blob/HEAD/CHANGELOG.md#3160)
> 
> [Compare
> Source](https://redirect.github.com/Stebalien/tempfile/compare/v3.15.0...v3.16.0)
> 
> - Update `getrandom` to `0.3.0` (thanks to
> [@&#8203;paolobarbolini](https://redirect.github.com/paolobarbolini)).
> - Allow `windows-sys` versions `0.59.x` in addition to `0.59.0` (thanks
> [@&#8203;ErichDonGubler](https://redirect.github.com/ErichDonGubler)).
> - Improved security documentation (thanks to
> [@&#8203;n0toose](https://redirect.github.com/n0toose) for collaborating
> with me on this).
> 
> </details>
> 
> ---
> 
> ### Configuration
> 
> 📅 **Schedule**: Branch creation - "* 0-3 * * 1" (UTC), Automerge - At
> any time (no schedule defined).
> 
> 🚦 **Automerge**: Disabled by config. Please merge this manually once you
> are satisfied.
> 
> ♻ **Rebasing**: Whenever PR becomes conflicted, or you tick the
> rebase/retry checkbox.
> 
> 👻 **Immortal**: This PR will be recreated if closed unmerged. Get
> [config
> help](https://redirect.github.com/renovatebot/renovate/discussions) if
> that's undesired.
> 
> ---
> 
> - [ ] <!-- rebase-check -->If you want to rebase/retry this PR, check
> this box
> 
> ---
> 
> This PR was generated by [Mend Renovate](https://mend.io/renovate/).
> View the [repository job
> log](https://developer.mend.io/github/cococonscious/koji).
> 
> <!--renovate-debug:eyJjcmVhdGVkSW5WZXIiOiIzOS4xNjQuMSIsInVwZGF0ZWRJblZlciI6IjM5LjE2Ny4xIiwidGFyZ2V0QnJhbmNoIjoibWFpbiIsImxhYmVscyI6W119-->

### Style


- format the tree with dprint and nightly rustfmt