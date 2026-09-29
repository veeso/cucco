# Releasing cucco

cucco is released under a `vX.Y.Z` tag. The whole process is driven by the
`Release` GitHub Actions workflow (`.github/workflows/release.yml`); there is no
manual `cargo publish` step.

## One-time setup

These steps only need doing once for the repository, not for every release.

1. Configure GitHub Pages under **Settings**, **Pages**, **Build and
   deployment** with **Source** set to **GitHub Actions**. The `Pages` workflow
   publishes the project page and installers at <https://veeso.github.io/cucco/>.
2. Create the `veeso/homebrew-cucco` repository so the release workflow can
   check it out and write `Formula/cucco.rb`.
3. Add the `RELEASE_PAT` repository secret. Its GitHub token must be able to
   push to `veeso/cucco` and `veeso/homebrew-cucco` and create releases.
4. Configure crates.io trusted publishing for the `cucco` crate and the
   `.github/workflows/release.yml` workflow. Publishing uses OpenID Connect, so
   no crates.io token secret is needed.

## Triggering a release

Go to the **Actions** tab, select the **Release** workflow, and choose **Run
workflow**. Run it from the default branch and provide both inputs:

- `version`: the version to release, such as `4.0.0`, without a leading `v`.
- `dry_run`: `true` by default. A dry run prepares and builds the release
  without pushing or publishing anything. Set it to `false` only for the real
  release.

The equivalent GitHub CLI commands are:

```sh
gh workflow run release.yml -f version=4.0.0 -f dry_run=true
gh run watch
```

Always complete a successful dry run first and inspect every job, including all
six target builds. Then repeat the command with `dry_run=false` to publish:

```sh
gh workflow run release.yml -f version=4.0.0 -f dry_run=false
gh run watch
```

## What the pipeline does

1. **Prepare** validates the semantic version and default branch, rejects an
   existing tag, updates `Cargo.toml` and `Cargo.lock`, regenerates
   `CHANGELOG.md`, and generates the release notes with `git-cliff`. A real
   release commits these changes as `chore: release vX.Y.Z` and pushes them to
   `main`. A dry run only prints the resulting diff.
2. **Build** creates cucco binaries for six targets:

   - `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` are static
     Linux builds produced in the pinned Alpine container through
     `just build_musl`.
   - `aarch64-apple-darwin` and `x86_64-apple-darwin` are release builds made
     on `macos-latest` runners.
   - `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` are release builds
     made on `windows-latest` runners.

   Each target is packaged as `cucco-vX.Y.Z-<target>.tar.gz` on Linux and macOS
   or `cucco-vX.Y.Z-<target>.zip` on Windows, with a matching `.sha256` file.
3. **Release** assembles the six target archives, their six checksums,
   `install.sh`, and `install.ps1`. A dry run uploads these 14 files as the
   `release-assets-dryrun` workflow artifact. A real run creates the GitHub
   release and `vX.Y.Z` tag with the generated release notes.
4. **Publish crate** authenticates through crates.io trusted publishing and
   runs `just publish`. This job is skipped during a dry run.
5. **Publish Homebrew formula** regenerates `Formula/cucco.rb` from the four
   macOS and Linux checksums, then commits and pushes it to
   `veeso/homebrew-cucco`. This job is skipped during a dry run.

The crate and Homebrew publishing jobs start independently after the GitHub
release succeeds.

## After a real release

1. Confirm the GitHub release contains all 14 assets: six archives, six
   checksum files, `install.sh`, and `install.ps1`.
2. Confirm crates.io shows the new `cucco` version.
3. Confirm the Homebrew formula was updated and install it on a clean Linux or
   macOS machine:

   ```sh
   brew install veeso/cucco/cucco
   ```

4. Confirm the POSIX installer works on a clean Linux or macOS machine:

   ```sh
   curl -sSLf https://veeso.github.io/cucco/install.sh | sh
   ```

5. Confirm the PowerShell installer works on a clean Windows machine:

   ```powershell
   irm https://veeso.github.io/cucco/install.ps1 | iex
   ```
