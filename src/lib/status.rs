//! Inspection of the git staging area.

use anyhow::{Context, Result};
use gix::Repository;

/// State of the staging area relative to the working tree.
#[derive(Debug, PartialEq, Eq)]
pub enum StagingStatus {
    /// Nothing is staged.
    Empty,
    /// Some changes are staged, while other tracked files have unstaged changes.
    Partial {
        /// Number of staged changes.
        staged: usize,
        /// Number of tracked files with unstaged changes.
        unstaged: usize,
    },
    /// Changes are staged and no tracked file has unstaged changes.
    Ready {
        /// Number of staged changes.
        staged: usize,
    },
}

/// Counts staged entries (HEAD tree vs index) and unstaged changes to tracked
/// files (index vs worktree). Untracked files are never counted.
/// Uses an empty tree as the baseline for initial commits.
fn count_changes(repo: &Repository) -> Result<(usize, usize)> {
    let index = repo.index_or_empty().context("could not read index")?;
    let head_tree_id = repo
        .head_tree_id_or_empty()
        .context("could not resolve HEAD tree")?;

    let mut staged_count: usize = 0;
    repo.tree_index_status(
        &head_tree_id,
        &index,
        None,
        gix::status::tree_index::TrackRenames::Disabled,
        |_, _, _| {
            staged_count += 1;
            Ok(gix::diff::index::Action::Continue(()))
        },
    )
    .context("could not diff HEAD tree against index")?;

    let mut unstaged_count: usize = 0;
    let status_iter = repo
        .status(gix::progress::Discard)
        .context("could not initialize status")?
        .index_worktree_options_mut(|opts| {
            opts.dirwalk_options = None; // only tracked files, not untracked
        })
        .into_index_worktree_iter(Vec::<gix::bstr::BString>::new())
        .context("could not iterate worktree status")?;

    for entry in status_iter {
        entry.context("error reading worktree status entry")?;
        unstaged_count += 1;
    }

    Ok((staged_count, unstaged_count))
}

/// Compares HEAD tree vs index (staged) and index vs worktree (unstaged).
/// Uses an empty tree as the baseline for initial commits.
///
/// # Errors
///
/// Returns an error if the index, HEAD, or worktree status cannot be read.
pub fn check_staging(repo: &Repository) -> Result<StagingStatus> {
    match count_changes(repo)? {
        (0, _) => Ok(StagingStatus::Empty),
        (s, 0) => Ok(StagingStatus::Ready { staged: s }),
        (s, u) => Ok(StagingStatus::Partial {
            staged: s,
            unstaged: u,
        }),
    }
}

/// Tells whether `--all` has anything to commit.
///
/// Returns `true` when at least one change is already staged or at least one
/// tracked file is modified or deleted in the worktree. Untracked files do not
/// count, matching `git commit -a`.
///
/// # Errors
///
/// Returns an error if the index, HEAD, or worktree status cannot be read.
pub fn has_tracked_changes(repo: &Repository) -> Result<bool> {
    let (staged, unstaged) = count_changes(repo)?;
    Ok(staged > 0 || unstaged > 0)
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::Path;

    use super::*;

    /// Creates a repository with one commit that tracks `tracked.txt`.
    fn init_repo_with_commit(dir: &Path) -> Result<git2::Repository, Box<dyn Error>> {
        let repo = git2::Repository::init(dir)?;
        std::fs::write(dir.join("tracked.txt"), "v1")?;
        {
            let mut index = repo.index()?;
            index.add_path(Path::new("tracked.txt"))?;
            index.write()?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let sig = git2::Signature::now("Tester", "test@example.com")?;
            repo.commit(Some("HEAD"), &sig, &sig, "chore: initial", &tree, &[])?;
        }
        Ok(repo)
    }

    #[test]
    fn test_check_staging_reports_empty_ready_and_partial() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        let repo = init_repo_with_commit(tempdir.path())?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert_eq!(check_staging(&gix_repo)?, StagingStatus::Empty);

        std::fs::write(tempdir.path().join("staged.txt"), "s")?;
        let mut index = repo.index()?;
        index.add_path(Path::new("staged.txt"))?;
        index.write()?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert_eq!(
            check_staging(&gix_repo)?,
            StagingStatus::Ready { staged: 1 }
        );

        std::fs::write(tempdir.path().join("tracked.txt"), "version two")?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert_eq!(
            check_staging(&gix_repo)?,
            StagingStatus::Partial {
                staged: 1,
                unstaged: 1
            }
        );

        tempdir.close()?;
        Ok(())
    }

    #[test]
    fn test_has_tracked_changes_is_false_for_clean_repo() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        let _repo = init_repo_with_commit(tempdir.path())?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert!(!has_tracked_changes(&gix_repo)?);

        tempdir.close()?;
        Ok(())
    }

    #[test]
    fn test_has_tracked_changes_ignores_untracked_files() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        let _repo = init_repo_with_commit(tempdir.path())?;
        std::fs::write(tempdir.path().join("untracked.txt"), "u")?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert!(!has_tracked_changes(&gix_repo)?);

        tempdir.close()?;
        Ok(())
    }

    #[test]
    fn test_has_tracked_changes_sees_unstaged_modification() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        let _repo = init_repo_with_commit(tempdir.path())?;
        std::fs::write(tempdir.path().join("tracked.txt"), "version two")?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert!(has_tracked_changes(&gix_repo)?);

        tempdir.close()?;
        Ok(())
    }

    #[test]
    fn test_has_tracked_changes_sees_unstaged_deletion() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        let _repo = init_repo_with_commit(tempdir.path())?;
        std::fs::remove_file(tempdir.path().join("tracked.txt"))?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert!(has_tracked_changes(&gix_repo)?);

        tempdir.close()?;
        Ok(())
    }

    #[test]
    fn test_has_tracked_changes_sees_staged_change() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        let repo = init_repo_with_commit(tempdir.path())?;
        std::fs::write(tempdir.path().join("staged.txt"), "s")?;
        let mut index = repo.index()?;
        index.add_path(Path::new("staged.txt"))?;
        index.write()?;

        let gix_repo = gix::discover(tempdir.path())?;
        assert!(has_tracked_changes(&gix_repo)?);

        tempdir.close()?;
        Ok(())
    }
}
