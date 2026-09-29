//! Commit message generation, staging, and commit creation with git hooks.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use cocogitto::command::commit::CommitOptions;
use cocogitto::{CocoGitto, CommitHook};
use gix::Repository;
use gix::bstr::{BString, ByteSlice};
use gix::status::index_worktree::Item;
use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};

/// Generates the conventional commit message.
///
/// # Errors
///
/// Returns an error if cocogitto rejects the message parts.
pub fn generate_commit_msg(
    commit_type: &str,
    scope: Option<&str>,
    summary: &str,
    body: Option<&str>,
    is_breaking_change: bool,
) -> Result<String> {
    let message = CocoGitto::get_conventional_message(
        commit_type,
        scope.map(str::to_owned),
        summary.to_owned(),
        body.map(str::to_owned),
        None,
        is_breaking_change,
    )?;

    Ok(message)
}

/// Writes the commit message to `COMMIT_EDITMSG` in the git directory.
///
/// # Errors
///
/// Returns an error if the message cannot be generated or the file cannot be
/// written.
pub fn write_commit_msg(
    repo: &Repository,
    commit_type: &str,
    scope: Option<&str>,
    summary: &str,
    body: Option<&str>,
    is_breaking_change: bool,
) -> Result<()> {
    let message = generate_commit_msg(commit_type, scope, summary, body, is_breaking_change)?;

    let commit_editmsg = repo.path().join("COMMIT_EDITMSG");
    let mut file = File::create(commit_editmsg)?;

    file.write_all(message.as_bytes())?;

    Ok(())
}

/// Creates a commit.
///
/// Staging is always performed by cucco (never delegated to cocogitto) so that
/// the same set of files is staged regardless of whether `no_verify` is set.
///
/// - `options.add_files` stages every change: tracked modifications,
///   deletions, and untracked files (matching `git add -A`). This backs
///   cucco's `--add-all` flag.
/// - `options.update_files` stages tracked modifications and deletions only,
///   never untracked files (matching `git commit -a`). This backs cucco's
///   `--all` flag.
/// - When both are set, `options.add_files` wins.
///
/// When `no_verify` is `false`, the `pre-commit` and `post-commit` git hooks
/// are invoked around the commit. `post-commit` failures are reported as a
/// warning and do not abort, matching `git commit`.
///
/// # Errors
///
/// Returns an error if staging fails, the `pre-commit` hook fails, or the
/// commit cannot be created.
pub fn commit(current_dir: PathBuf, mut options: CommitOptions, no_verify: bool) -> Result<()> {
    // Set config path before creating CocoGitto instance (required in 6.4.0+)
    let config_path = current_dir.join("cog.toml");
    cocogitto::set_config_path(config_path.to_string_lossy().to_string());

    let include_untracked = options.add_files;
    let stage_requested = options.add_files || options.update_files;
    options.add_files = false;
    options.update_files = false;

    if stage_requested {
        let repo = gix::discover(&current_dir)?;
        stage_changes(&repo, include_untracked)?;
    }

    let cocogitto = CocoGitto::get_at(current_dir)?;

    if !no_verify {
        cocogitto.run_commit_hook(CommitHook::PreCommit)?;
    }

    cocogitto.conventional_commit(options)?;

    if !no_verify && let Err(e) = cocogitto.run_commit_hook(CommitHook::PostCommit) {
        eprintln!("warning: post-commit hook failed: {e}");
    }

    Ok(())
}

/// Stages unstaged changes via gix.
///
/// Tracked modifications and deletions are always staged, equivalent to
/// `git add -u`. When `include_untracked` is `true`, untracked files (but not
/// ignored files) are staged too, equivalent to `git add -A`.
///
/// # Errors
///
/// Returns an error if the repository has no working directory, the worktree
/// status cannot be read, or the index cannot be read or written.
pub fn stage_changes(repo: &Repository, include_untracked: bool) -> Result<()> {
    use gix::status::UntrackedFiles;

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("repository has no working directory"))?
        .to_path_buf();

    let untracked_files = if include_untracked {
        UntrackedFiles::Files
    } else {
        UntrackedFiles::None
    };

    let iter = repo
        .status(gix::progress::Discard)?
        .untracked_files(untracked_files)
        .into_index_worktree_iter(Vec::new())?;

    let pending = iter
        .map(|item| item.map(|item| PendingChange::from_item(item, include_untracked)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut to_remove: Vec<BString> = Vec::new();
    let mut to_update: Vec<BString> = Vec::new();
    let mut to_add: Vec<UntrackedAdd> = Vec::new();
    for change in pending.into_iter().flatten() {
        match change {
            PendingChange::Remove(path) => to_remove.push(path),
            PendingChange::Update(path) => to_update.push(path),
            PendingChange::Add(add) => to_add.push(add),
        }
    }

    if to_remove.is_empty() && to_update.is_empty() && to_add.is_empty() {
        return Ok(());
    }

    // `at_or_default` gives us an empty in-memory index when `.git/index`
    // doesn't exist yet (fresh `git init` with no commits). The file is
    // created on `index.write()` below.
    let index_path = repo.index_path();
    let mut index =
        gix::index::File::at_or_default(&index_path, repo.object_hash(), false, Default::default())
            .map_err(|error| anyhow::Error::from(error.into_error()))?;

    if !to_remove.is_empty() {
        index.remove_entries(|_, path, _| {
            let path_bytes: &[u8] = path.as_ref();
            to_remove.iter().any(|p| p.as_slice() == path_bytes)
        });
    }

    for rela in to_update {
        let bstr = rela.as_ref();
        let abs = workdir.join(gix::path::from_bstr(bstr).as_ref());
        let bytes = std::fs::read(&abs)?;
        let oid = repo.write_blob(&bytes)?.detach();
        let metadata = gix::index::fs::Metadata::from_path_no_follow(&abs)?;
        let stat = gix::index::entry::Stat::from_fs(&metadata)?;
        if let Some(entry) =
            index.entry_mut_by_path_and_stage(bstr, gix::index::entry::Stage::Unconflicted)
        {
            entry.id = oid;
            // Refresh stat from disk so `git status` doesn't flag the file as
            // modified after commit due to a stale mtime/size in the index.
            entry.stat = stat;
        }
    }

    let mut needs_sort = false;
    for add in to_add {
        let abs = workdir.join(gix::path::from_bstr(add.rela_path.as_bstr()).as_ref());
        let metadata = gix::index::fs::Metadata::from_path_no_follow(&abs)?;
        let stat = gix::index::entry::Stat::from_fs(&metadata)?;
        let (oid, mode) = match add.kind {
            UntrackedKind::Symlink => {
                let target = std::fs::read_link(&abs)?;
                let bytes = gix::path::into_bstr(target).into_owned();
                let oid = repo.write_blob(bytes.as_slice())?.detach();
                (oid, gix::index::entry::Mode::SYMLINK)
            }
            UntrackedKind::File => {
                let bytes = std::fs::read(&abs)?;
                let oid = repo.write_blob(&bytes)?.detach();
                let mode = if is_executable(&abs)? {
                    gix::index::entry::Mode::FILE_EXECUTABLE
                } else {
                    gix::index::entry::Mode::FILE
                };
                (oid, mode)
            }
        };

        if let Some(entry) = index.entry_mut_by_path_and_stage(
            add.rela_path.as_bstr(),
            gix::index::entry::Stage::Unconflicted,
        ) {
            entry.id = oid;
            entry.mode = mode;
            entry.stat = stat;
        } else {
            index.dangerously_push_entry(
                stat,
                oid,
                gix::index::entry::Flags::empty(),
                mode,
                add.rela_path.as_bstr(),
            );
            needs_sort = true;
        }
    }

    if needs_sort {
        index.sort_entries();
    }

    // Drop the cached TREE extension. Mutating entries above leaves the cache
    // pointing at the pre-edit tree; libgit2's `index.write_tree()` honors that
    // cache and would otherwise return the old tree id, producing an empty
    // commit (same tree as HEAD) when cocogitto writes the commit.
    index.remove_tree();

    index
        .write(gix::index::write::Options::default())
        .map_err(|error| anyhow::Error::from(error.into_error()))?;
    Ok(())
}

/// A worktree change that has to be recorded in the index.
enum PendingChange {
    Remove(BString),
    Update(BString),
    Add(UntrackedAdd),
}

impl PendingChange {
    /// Classifies a worktree status item, skipping anything not stageable.
    fn from_item(item: Item, include_untracked: bool) -> Option<Self> {
        match item {
            Item::Modification {
                rela_path,
                status: EntryStatus::Change(Change::Removed),
                ..
            } => Some(Self::Remove(rela_path)),
            Item::Modification {
                rela_path,
                status: EntryStatus::Change(Change::Modification { .. } | Change::Type { .. }),
                ..
            } => Some(Self::Update(rela_path)),
            Item::DirectoryContents { entry, .. }
                if include_untracked && entry.status == gix::dir::entry::Status::Untracked =>
            {
                UntrackedAdd::from_dir_entry(entry).map(Self::Add)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
enum UntrackedKind {
    File,
    Symlink,
}

struct UntrackedAdd {
    rela_path: BString,
    kind: UntrackedKind,
}

impl UntrackedAdd {
    fn from_dir_entry(entry: gix::dir::Entry) -> Option<Self> {
        let kind = match entry.disk_kind? {
            gix::dir::entry::Kind::File => UntrackedKind::File,
            gix::dir::entry::Kind::Symlink => UntrackedKind::Symlink,
            // Directories should not appear in Matching emission for files,
            // and repos/untrackable entries are intentionally skipped.
            _ => return None,
        };
        Some(Self {
            rela_path: entry.rela_path,
            kind,
        })
    }
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)?.permissions().mode();
    Ok(mode & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_commit_msg() {
        let message =
            generate_commit_msg("feat", Some("space"), "add more space", Some("body"), true)
                .unwrap();

        assert_eq!(message, "feat(space)!: add more space\n\nbody");
    }

    #[test]
    fn test_generate_commit_msg_minimal() {
        let message = generate_commit_msg("fix", None, "patch it", None, false).unwrap();

        assert_eq!(message, "fix: patch it");
    }

    #[test]
    fn test_write_commit_msg() {
        let tempdir = tempfile::tempdir().unwrap();
        gix::init(tempdir.path()).unwrap();
        let repo = gix::discover(tempdir.path()).unwrap();

        write_commit_msg(&repo, "fix", None, "patch it", None, false).unwrap();

        let written = std::fs::read_to_string(repo.path().join("COMMIT_EDITMSG")).unwrap();
        assert_eq!(written, "fix: patch it");
    }

    /// Creates a repository whose only commit tracks `keep.txt` and `gone.txt`.
    fn init_repo_with_commit(dir: &std::path::Path) -> git2::Repository {
        let repo = git2::Repository::init(dir).unwrap();
        std::fs::write(dir.join("keep.txt"), "v1").unwrap();
        std::fs::write(dir.join("gone.txt"), "v1").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("keep.txt")).unwrap();
        index.add_path(std::path::Path::new("gone.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "chore: initial", &tree, &[])
            .unwrap();
        drop(tree);
        repo
    }

    fn blob_id(content: &str) -> git2::Oid {
        git2::Oid::hash_object(git2::ObjectType::Blob, content.as_bytes()).unwrap()
    }

    fn staged_id(repo: &git2::Repository, path: &str) -> Option<git2::Oid> {
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        index.get_path(std::path::Path::new(path), 0).map(|e| e.id)
    }

    #[test]
    fn test_stage_changes_stages_tracked_but_not_untracked() {
        let tempdir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tempdir.path());
        std::fs::write(tempdir.path().join("keep.txt"), "version two").unwrap();
        std::fs::remove_file(tempdir.path().join("gone.txt")).unwrap();
        std::fs::write(tempdir.path().join("new.txt"), "new").unwrap();

        let gix_repo = gix::discover(tempdir.path()).unwrap();
        stage_changes(&gix_repo, false).unwrap();

        assert_eq!(staged_id(&repo, "keep.txt"), Some(blob_id("version two")));
        assert_eq!(staged_id(&repo, "gone.txt"), None);
        assert_eq!(staged_id(&repo, "new.txt"), None);
    }

    #[test]
    fn test_stage_changes_includes_untracked_when_requested() {
        let tempdir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tempdir.path());
        std::fs::write(tempdir.path().join("new.txt"), "new").unwrap();
        std::fs::write(tempdir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(tempdir.path().join("ignored.txt"), "ignored").unwrap();

        let gix_repo = gix::discover(tempdir.path()).unwrap();
        stage_changes(&gix_repo, true).unwrap();

        assert_eq!(staged_id(&repo, "new.txt"), Some(blob_id("new")));
        assert_eq!(
            staged_id(&repo, ".gitignore"),
            Some(blob_id("ignored.txt\n"))
        );
        assert_eq!(staged_id(&repo, "ignored.txt"), None);
        assert_eq!(staged_id(&repo, "keep.txt"), Some(blob_id("v1")));
    }

    #[test]
    fn test_stage_changes_works_without_initial_commit() {
        let tempdir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tempdir.path()).unwrap();
        std::fs::write(tempdir.path().join("new.txt"), "new").unwrap();

        let gix_repo = gix::discover(tempdir.path()).unwrap();
        stage_changes(&gix_repo, true).unwrap();

        assert_eq!(staged_id(&repo, "new.txt"), Some(blob_id("new")));
    }

    #[test]
    fn test_stage_changes_without_changes_leaves_index_untouched() {
        let tempdir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tempdir.path());

        let gix_repo = gix::discover(tempdir.path()).unwrap();
        stage_changes(&gix_repo, true).unwrap();

        assert_eq!(staged_id(&repo, "keep.txt"), Some(blob_id("v1")));
        assert_eq!(staged_id(&repo, "gone.txt"), Some(blob_id("v1")));
    }
}
