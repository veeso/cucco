use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use cucco::config::{CommitScope, Config};
use cucco::questions::ScopeAutocompleter;
use cucco::scope::detect_scope_matches;
use git2::{IndexAddOption, Repository};
use indexmap::IndexMap;
use inquire::autocompletion::Autocomplete;
#[cfg(not(target_os = "windows"))]
use rexpect::{
    process::WaitStatus,
    session::{PtySession, spawn_command},
};
use tempfile::TempDir;

#[cfg(not(target_os = "windows"))]
fn setup_config_home() -> Result<TempDir, Box<dyn Error>> {
    let temp_dir = tempfile::tempdir()?;
    std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());

    Ok(temp_dir)
}

fn setup_test_dir() -> Result<(PathBuf, TempDir, Repository), Box<dyn std::error::Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco").to_path_buf();
    let temp_dir = tempfile::tempdir()?;

    let repo = Repository::init(temp_dir.path())?;

    let mut config = repo.config()?;
    config.set_str("user.name", "test")?;
    config.set_str("user.email", "test@example.org")?;
    config.set_str("core.hooksPath", ".git/hooks")?;

    Ok((bin_path, temp_dir, repo))
}

#[cfg(not(target_os = "windows"))]
fn get_last_commit(repo: &Repository) -> Result<git2::Commit<'_>, git2::Error> {
    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    let oid = walk.next().expect("cannot get commit in revwalk")?;

    repo.find_commit(oid)
}

fn do_initial_commit(repo: &Repository, message: &'static str) -> Result<(), Box<dyn Error>> {
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?;
    Ok(())
}

fn git_add(repo: &Repository, pattern: &str) -> Result<(), Box<dyn Error>> {
    let mut index = repo.index()?;
    index.add_all([pattern].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
trait ExpectPromps {
    fn expect_commit_type(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_scope(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_summary(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_body(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_breaking(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_breaking_details(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_multiline_end(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_issues(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_issues_details(&mut self) -> Result<String, rexpect::error::Error>;
    fn expect_confirm(&mut self) -> Result<String, rexpect::error::Error>;
}

#[cfg(not(target_os = "windows"))]
impl ExpectPromps for PtySession {
    fn expect_commit_type(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("are you committing?")
    }

    fn expect_scope(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("scope of this change?")
    }

    fn expect_summary(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("description of the change")
    }

    fn expect_body(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("\x1b[>1u")?;
        self.exp_string("longer description of the change")
    }

    fn expect_breaking(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("\x1b[<1u")?;
        self.exp_string("breaking changes?")
    }

    fn expect_breaking_details(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("\x1b[>1u")?;
        self.exp_string("in detail:")
    }

    fn expect_multiline_end(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("\x1b[<1u")
    }

    fn expect_issues(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("any open issues?")
    }

    fn expect_issues_details(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("issue reference:")
    }

    fn expect_confirm(&mut self) -> Result<String, rexpect::error::Error> {
        self.exp_string("Proceed with this commit?")
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_everything_correct() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "foo")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs(readme): initial draft")?;

    fs::write(temp_dir.path().join("config.json"), "bar")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-a")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("feat")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("config")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("refactor config pairs")?;
    process.flush()?;
    process.expect_body()?;
    process.send("Removed and added a config pair each")?;
    process.send("\x1b[13;3u")?;
    process.send_line("Necessary for future compatibility.")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("Y")?;
    process.flush()?;
    process.expect_breaking_details()?;
    process.send("Something can't be configured anymore")?;
    process.send("\x1b\r")?;
    process.send_line("The old configuration is no longer supported.")?;
    process.flush()?;
    process.expect_multiline_end()?;
    process.expect_issues()?;
    process.send_line("Y")?;
    process.flush()?;
    process.expect_issues_details()?;
    process.send_line("closes #1")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(
        commit.summary(),
        Ok(Some("feat(config)!: refactor config pairs"))
    );
    assert_eq!(
        commit.body(),
        Ok(Some(
            "Removed and added a config pair each\n\
Necessary for future compatibility.\n\n\
closes #1\n\
BREAKING CHANGE: Something can't be configured anymore\n\
The old configuration is no longer supported."
        ))
    );

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_literal_backslash_n_is_preserved_in_body() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "changed")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-y")
        .arg("--autocomplete=false");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("docs")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("document an escape sequence")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line(r"Keep \n as literal text.")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(
        commit.summary(),
        Ok(Some("docs: document an escape sequence"))
    );
    assert_eq!(commit.body(), Ok(Some(r"Keep \n as literal text.")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_alt_enter_renders_each_new_input_line() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "changed")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-y")
        .arg("--autocomplete=false");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("docs")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("document multiline input")?;
    process.flush()?;
    process.expect_body()?;
    process.send("first line")?;
    process.flush()?;
    process.exp_string("first line")?;
    process.exp_string("\x1b[?25h")?;

    process.send("\x1b[13;3u")?;
    process.flush()?;
    let first_newline_frame = process.exp_string("\x1b[?25h")?;
    assert!(
        first_newline_frame.contains("first line\r\n\x1b7")
            && first_newline_frame.ends_with("\x1b8"),
        "cursor did not move to the first new input line: {first_newline_frame:?}"
    );

    process.send("\x1b[13;3u")?;
    process.flush()?;
    let second_newline_frame = process.exp_string("\x1b[?25h")?;
    assert!(
        second_newline_frame.contains("first line\r\n\r\n\x1b7"),
        "second new input line was not rendered: {second_newline_frame:?}"
    );
    assert!(
        second_newline_frame.ends_with("\x1b8"),
        "cursor did not move to the second new input line: {second_newline_frame:?}"
    );

    process.send_line("third line")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.body(), Ok(Some("first line\n\nthird line")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_hook_correct() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("config.json"), "abc")?;
    git_add(&repo, "*")?;
    fs::remove_file(temp_dir.path().join(".git").join("COMMIT_EDITMSG")).unwrap_or(());

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--hook")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("fix")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("some weird error")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let editmsg = temp_dir.path().join(".git").join("COMMIT_EDITMSG");
    assert!(editmsg.exists());
    assert_eq!(
        fs::read(editmsg)?,
        "fix: some weird error".as_bytes().to_vec()
    );

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_stdout_correct() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("config.json"), "abc")?;
    git_add(&repo, "*")?;
    fs::remove_file(temp_dir.path().join(".git").join("COMMIT_EDITMSG")).unwrap_or(());

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--stdout")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("fix")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("some weird error")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    let expected_output = "fix: some weird error";

    let _ = process
        .exp_string(expected_output)
        .expect("failed to match output");

    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let editmsg = temp_dir.path().join(".git").join("COMMIT_EDITMSG");
    assert!(!editmsg.exists());

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_empty_breaking_text_correct() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("Cargo.toml"), "bar")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-a")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("docs")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("cargo")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("rename project")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("Renamed the project to a new name.")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("Y")?;
    process.flush()?;
    process.expect_breaking_details()?;
    // `^[` is the same as <esc>
    process.send_control('[')?;
    process.flush()?;
    process.expect_multiline_end()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("docs(cargo)!: rename project")));
    assert_eq!(
        commit.body(),
        Ok(Some("Renamed the project to a new name."))
    );

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
fn test_non_repository_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");
    let temp_dir = tempfile::tempdir()?;

    let mut cmd = Command::new(bin_path);
    cmd.arg("-C").arg(temp_dir.path());

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("could not find git repository"));

    temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_empty_repository_error() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, _) = setup_test_dir()?;

    let mut cmd = Command::new(bin_path);
    cmd.arg("-C").arg(temp_dir.path()).arg("-y");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("no files staged for commit"));

    temp_dir.close()?;
    Ok(())
}

#[test]
fn test_all_hook_exclusive_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

    let mut cmd = Command::new(bin_path);
    cmd.arg("--hook");
    cmd.arg("--all");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("the argument '--hook' cannot be used with '--all'"));

    Ok(())
}

#[test]
fn test_all_stdout_exclusive_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

    let mut cmd = Command::new(bin_path);
    cmd.arg("--stdout");
    cmd.arg("--all");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("the argument '--stdout' cannot be used with '--all'"));

    Ok(())
}

#[test]
fn test_add_all_hook_exclusive_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

    let mut cmd = Command::new(bin_path);
    cmd.arg("--hook");
    cmd.arg("--add-all");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("cannot be used with"));
    assert!(stderr_out.contains("'--hook'"));
    assert!(stderr_out.contains("'--add-all'"));

    Ok(())
}

#[test]
fn test_add_all_stdout_exclusive_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

    let mut cmd = Command::new(bin_path);
    cmd.arg("--stdout");
    cmd.arg("--add-all");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("cannot be used with"));
    assert!(stderr_out.contains("'--stdout'"));
    assert!(stderr_out.contains("'--add-all'"));

    Ok(())
}

#[test]
fn test_all_add_all_exclusive_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

    let mut cmd = Command::new(bin_path);
    cmd.arg("-a");
    cmd.arg("-A");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("cannot be used with"));
    assert!(stderr_out.contains("'--all'"));
    assert!(stderr_out.contains("'--add-all'"));

    Ok(())
}

#[test]
fn test_hook_stdout_exclusive_error() -> Result<(), Box<dyn Error>> {
    let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

    let mut cmd = Command::new(bin_path);
    cmd.arg("--stdout");
    cmd.arg("--hook");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(stderr_out.contains("the argument '--stdout' cannot be used with '--hook'"));

    Ok(())
}

#[test]
fn test_completion_scripts_success() -> Result<(), Box<dyn Error>> {
    fn run_for(shell: &'static str, containing: &'static str) -> Result<(), Box<dyn Error>> {
        let bin_path = assert_cmd::cargo::cargo_bin!("cucco");

        let mut cmd = Command::new(bin_path);
        cmd.arg("completions").arg(shell);

        let cmd_out = cmd.output()?;
        let stdout = String::from_utf8(cmd_out.stdout)?;

        assert!(cmd_out.status.success());
        assert!(stdout.contains(containing));

        Ok(())
    }

    run_for("nushell", "def \"nu-complete cucco")?;
    run_for("fish", "complete -c cucco -n \"__fish_cucco_needs_command")?;
    run_for("bash", "complete -F _cucco -o bashdefault -o default cucco")?;
    run_for(
        "powershell",
        "Register-ArgumentCompleter -Native -CommandName 'cucco'",
    )?;
    run_for("zsh", "#compdef cucco")
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_xdg_config() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    let xdg_cfg_home = tempfile::tempdir()?;
    fs::create_dir(xdg_cfg_home.path().join("cucco"))?;
    fs::write(
        xdg_cfg_home.path().join("cucco/config.toml"),
        "[[commit_types]]\nname=\"wip\"\ndescription = \"Do not create PR with this commit\"",
    )?;

    fs::write(temp_dir.path().join("config.json"), "bar")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .env("XDG_CONFIG_HOME", xdg_cfg_home.path().as_os_str())
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--stdout")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("wip")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("some weird error")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    let expected_output = "wip: some weird error";

    let _ = process
        .exp_string(expected_output)
        .expect("failed to match output");

    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {:?}", eof_output);
    }

    temp_dir.close()?;
    config_temp_dir.close()?;
    xdg_cfg_home.close()?;

    Ok(())
}

#[test]
fn test_no_staged_files_error() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;

    fs::write(temp_dir.path().join("README.md"), "hello")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    // Modify a file but don't stage it
    fs::write(temp_dir.path().join("README.md"), "changed")?;

    let mut cmd = Command::new(bin_path);
    cmd.arg("-C").arg(temp_dir.path());

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(
        stderr_out.contains("no files staged for commit"),
        "expected 'no files staged for commit' in stderr, got: {stderr_out}"
    );

    temp_dir.close()?;

    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_confirmation_accept() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "foo")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-a");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("fix")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("patch a bug")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    // Expect the commit message preview and confirmation prompt
    process.exp_string("fix: patch a bug")?;
    process.expect_confirm()?;
    process.send_line("Y")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: patch a bug")));

    temp_dir.close()?;
    config_temp_dir.close()?;

    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_partial_staging_warning() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;

    fs::write(temp_dir.path().join("a.txt"), "aaa")?;
    fs::write(temp_dir.path().join("b.txt"), "bbb")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "chore: initial")?;

    // Modify both, stage only one
    fs::write(temp_dir.path().join("a.txt"), "aaa changed")?;
    fs::write(temp_dir.path().join("b.txt"), "bbb changed")?;
    let mut index = repo.index()?;
    index.add_all(["a.txt"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let mut cmd = Command::new(&bin_path);
    cmd.arg("-C").arg(temp_dir.path());

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(
        stderr_out.contains("file(s) staged for commit"),
        "expected partial staging warning in stderr, got: {stderr_out}"
    );
    assert!(
        stderr_out.contains("unstaged changes not included"),
        "expected unstaged warning in stderr, got: {stderr_out}"
    );
    assert!(!stderr_out.contains("no files staged for commit"));

    temp_dir.close()?;

    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_all_flag_skips_staging_check() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;

    fs::write(temp_dir.path().join("README.md"), "hello")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    // Modify a file but don't stage it
    fs::write(temp_dir.path().join("README.md"), "changed")?;

    let mut cmd = Command::new(bin_path);
    cmd.arg("-C").arg(temp_dir.path()).arg("--all");
    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!stderr_out.contains("no files staged for commit"));

    temp_dir.close()?;

    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_confirmation_decline() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "foo")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs(readme): initial draft")?;

    fs::write(temp_dir.path().join("config.json"), "bar")?;
    git_add(&repo, ".")?;
    repo.index()?.write()?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-a");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("fix")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("patch a bug")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    // Expect the commit message preview and decline the confirmation
    process.exp_string("fix: patch a bug")?;
    process.expect_confirm()?;
    process.send_line("N")?;
    process.flush()?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    // Verify the last commit is still the initial one (no new commit was made)
    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("docs(readme): initial draft")));

    temp_dir.close()?;
    config_temp_dir.close()?;

    Ok(())
}

#[test]
fn test_scope_autocompletion() -> Result<(), Box<dyn Error>> {
    let tempdir = tempfile::tempdir()?;
    let workdir = tempdir.path();

    // Init git repo and commit using git2
    let repo = git2::Repository::init(workdir)?;

    // Create a commit with a scope
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = git2::Signature::now("Tester", "test@example.com")?;

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "feat(database): initial db schema",
        &tree,
        &[],
    )?;

    // Create another commit with a different scope
    let head = repo.head()?.peel_to_commit()?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fix(api): fix login endpoint",
        &tree,
        &[&head],
    )?;

    // Setup config with a configured scope
    let mut commit_scopes = IndexMap::new();
    commit_scopes.insert(
        "frontend".into(),
        CommitScope {
            name: "frontend".into(),
            description: Some("Frontend code".into()),
            patterns: vec![],
            #[cfg(feature = "ast-grep")]
            ast_grep: None,
        },
    );

    let config = Config {
        commit_scopes,
        workdir: workdir.to_path_buf(),
        autocomplete: true,
        breaking_changes: false,
        commit_types: IndexMap::new(),
        emoji: false,
        issues: false,
        sign: false,
        force_config_scopes: false,
        allow_empty_scope: true,
    };

    let mut autocompleter = ScopeAutocompleter { config };

    // Test get_all_scopes
    let scopes = autocompleter.get_all_scopes();

    assert!(scopes.contains(&"database".to_string()));
    assert!(scopes.contains(&"api".to_string()));
    assert!(scopes.contains(&"frontend".to_string()));

    // Test get_suggestions (Autocomplete trait)
    let suggestions = autocompleter
        .get_suggestions("data")
        .expect("Failed to get suggestions");
    assert!(suggestions.contains(&"database".to_string()));
    assert!(!suggestions.contains(&"api".to_string()));

    let suggestions = autocompleter
        .get_suggestions("front")
        .expect("Failed to get suggestions");
    // "frontend" has a description, so the suggestion is formatted as "frontend: Frontend code"
    assert!(suggestions.iter().any(|s| s.starts_with("frontend")));

    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_force_config_scopes_integration() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(
        temp_dir.path().join(".cucco.toml"),
        "force_config_scopes = true\n[[commit_scopes]]\nname = \"app\"",
    )?;

    fs::write(temp_dir.path().join("a.txt"), "foo")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--stdout");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("feat")?;
    process.flush()?;
    process.expect_scope()?;
    // In Select mode, "app" should be the first and only option.
    // Pressing enter should select "app".
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("test force scope")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    let expected_output = "feat(app): test force scope";
    let _ = process.exp_string(expected_output)?;

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_require_scope_integration() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(
        temp_dir.path().join(".cucco.toml"),
        "allow_empty_scope = false",
    )?;

    fs::write(temp_dir.path().join("a.txt"), "foo")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--stdout");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("feat")?;
    process.flush()?;
    process.expect_scope()?;
    // Try to skip by sending empty line
    process.send_line("")?;
    process.flush()?;

    // It should NOT proceed to summary, but show error.
    // Inquire shows error message "A scope is required"
    process.exp_string("A scope is required")?;

    // Now provide a scope
    process.send_line("required-scope")?;
    process.flush()?;

    process.expect_summary()?;
    process.send_line("test require scope")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    let expected_output = "feat(required-scope): test require scope";
    let _ = process.exp_string(expected_output)?;

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_scope_pattern_auto_assigns_scope() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(
        temp_dir.path().join(".cucco.toml"),
        "[[commit_scopes]]\nname = \"config\"\npatterns = \"/config\\\\.json$\"",
    )?;
    fs::write(temp_dir.path().join("config.json"), "abc")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--stdout");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("feat")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line("wire auto scope")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    let _ = process.exp_string("feat(config): wire auto scope")?;
    let eof_output = process.exp_eof();
    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_force_config_scopes_prints_pre_assigned_scope() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(
        temp_dir.path().join(".cucco.toml"),
        "force_config_scopes = true\n[[commit_scopes]]\nname = \"config\"\npatterns = \"/config\\\\.json$\"",
    )?;
    fs::write(temp_dir.path().join("config.json"), "abc")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--stdout");

    let mut process = spawn_command(cmd, Some(5000))?;

    process.expect_commit_type()?;
    process.send_line("feat")?;
    process.flush()?;

    // Only one scope matched, so the select prompt is skipped, but the
    // pre-assigned scope must still be printed instead of being invisible.
    process.expect_scope()?;
    process.exp_string("config")?;

    process.expect_summary()?;
    process.send_line("wire pre-assigned scope")?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;

    let _ = process.exp_string("feat(config): wire pre-assigned scope")?;
    let eof_output = process.exp_eof();
    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));

    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
fn test_detect_scope_matches_from_scope_patterns() -> Result<(), Box<dyn Error>> {
    let temp_dir = tempfile::tempdir()?;
    let repo = Repository::init(temp_dir.path())?;
    let gix_repo = gix::discover(temp_dir.path())?;

    fs::write(temp_dir.path().join("config.json"), "abc")?;
    git_add(&repo, ".")?;
    fs::write(
        temp_dir.path().join(".cucco.toml"),
        "[[commit_scopes]]\nname = \"config\"\npatterns = \"/config\\\\.json$\"",
    )?;

    let config = Config::new(Some(cucco::config::ConfigArgs {
        _current_dir: Some(temp_dir.path().to_path_buf()),
        ..Default::default()
    }))?;

    let matches = detect_scope_matches(&gix_repo, &config)?;
    assert_eq!(matches.suggested(), Some("config"));

    temp_dir.close()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_hook(temp_dir: &TempDir, name: &str, body: &str) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    let hooks = temp_dir.path().join(".git").join("hooks");
    fs::create_dir_all(&hooks)?;
    let path = hooks.join(name);
    fs::write(&path, body)?;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn drive_simple_fix_commit(process: &mut PtySession, summary: &str) -> Result<(), Box<dyn Error>> {
    process.expect_commit_type()?;
    process.send_line("fix")?;
    process.flush()?;
    process.expect_scope()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_summary()?;
    process.send_line(summary)?;
    process.flush()?;
    process.expect_body()?;
    process.send_line("")?;
    process.flush()?;
    process.expect_breaking()?;
    process.send_line("N")?;
    process.flush()?;
    process.expect_issues()?;
    process.send_line("N")?;
    process.flush()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_pre_commit_hook_runs() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "x")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    let sentinel = temp_dir.path().join("hook-fired");
    let hook_body = format!("#!/bin/sh\ntouch {}\n", sentinel.display());
    install_hook(&temp_dir, "pre-commit", &hook_body)?;

    fs::write(temp_dir.path().join("a.txt"), "x")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "hooked")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    assert!(sentinel.exists(), "pre-commit hook did not run");
    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: hooked")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_pre_commit_hook_failure_aborts() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "x")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    install_hook(&temp_dir, "pre-commit", "#!/bin/sh\nexit 1\n")?;

    fs::write(temp_dir.path().join("a.txt"), "x")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "should fail")?;
    let _ = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    assert!(
        !success,
        "expected non-zero exit when pre-commit hook fails"
    );

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("docs: initial")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_no_verify_skips_pre_commit_hook() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "x")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    install_hook(&temp_dir, "pre-commit", "#!/bin/sh\nexit 1\n")?;

    fs::write(temp_dir.path().join("a.txt"), "x")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--no-verify")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "bypass")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("--no-verify path failed: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: bypass")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_post_commit_hook_runs() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "x")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    let sentinel = temp_dir.path().join("post-commit-fired");
    let hook_body = format!("#!/bin/sh\ntouch {}\n", sentinel.display());
    install_hook(&temp_dir, "post-commit", &hook_body)?;

    fs::write(temp_dir.path().join("a.txt"), "x")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "post hooked")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    assert!(sentinel.exists(), "post-commit hook did not run");
    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: post hooked")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_post_commit_hook_failure_does_not_abort() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "x")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    install_hook(&temp_dir, "post-commit", "#!/bin/sh\nexit 1\n")?;

    fs::write(temp_dir.path().join("a.txt"), "x")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "post fail tolerated")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("post-commit failure must not abort the commit: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: post fail tolerated")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_add_all_stages_modified_deleted_and_untracked() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("tracked.txt"), "v1")?;
    fs::write(temp_dir.path().join("removed.txt"), "r")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    fs::write(temp_dir.path().join("tracked.txt"), "version two")?;
    fs::remove_file(temp_dir.path().join("removed.txt"))?;
    fs::write(temp_dir.path().join("untracked.txt"), "u")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--add-all")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "stage everything")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: stage everything")));
    let tree = commit.tree()?;
    let tracked_entry = tree.get_name("tracked.txt").expect("tracked.txt missing");
    let tracked_blob = repo.find_blob(tracked_entry.id())?;
    assert_eq!(tracked_blob.content(), b"version two");
    assert!(
        tree.get_name("removed.txt").is_none(),
        "removed.txt deletion should be staged by --add-all"
    );
    let untracked_entry = tree
        .get_name("untracked.txt")
        .expect("untracked.txt should be staged by --add-all");
    let blob = repo.find_blob(untracked_entry.id())?;
    assert_eq!(blob.content(), b"u");

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_add_all_stages_in_fresh_repo_without_initial_commit() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("untracked.txt"), "u")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-A")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "stage in fresh repo")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    let tree = commit.tree()?;
    let untracked_entry = tree
        .get_name("untracked.txt")
        .expect("untracked.txt should be staged by -A");
    let blob = repo.find_blob(untracked_entry.id())?;
    assert_eq!(blob.content(), b"u");

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_all_stages_tracked_but_not_untracked() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("tracked.txt"), "v1")?;
    fs::write(temp_dir.path().join("removed.txt"), "r")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    // Unstaged modification and deletion of tracked files.
    fs::write(temp_dir.path().join("tracked.txt"), "version two")?;
    fs::remove_file(temp_dir.path().join("removed.txt"))?;
    // A new file the user staged on purpose.
    fs::write(temp_dir.path().join("staged_new.txt"), "s")?;
    git_add(&repo, "staged_new.txt")?;
    // A new file the user never staged.
    fs::write(temp_dir.path().join("untracked.txt"), "u")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("-a")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "stage tracked only")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: stage tracked only")));
    let tree = commit.tree()?;

    let tracked_entry = tree.get_name("tracked.txt").expect("tracked.txt missing");
    let tracked_blob = repo.find_blob(tracked_entry.id())?;
    assert_eq!(tracked_blob.content(), b"version two");
    assert!(
        tree.get_name("removed.txt").is_none(),
        "removed.txt deletion should be staged by -a"
    );
    assert!(
        tree.get_name("staged_new.txt").is_some(),
        "staged_new.txt was staged by the user and must be committed"
    );
    assert!(
        tree.get_name("untracked.txt").is_none(),
        "untracked.txt must not be staged by -a"
    );

    assert!(temp_dir.path().join("untracked.txt").exists());
    assert_eq!(
        repo.status_file(std::path::Path::new("untracked.txt"))?,
        git2::Status::WT_NEW
    );

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}

#[test]
fn test_all_with_only_untracked_files_errors() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;

    fs::write(temp_dir.path().join("README.md"), "hello")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    fs::write(temp_dir.path().join("untracked.txt"), "u")?;

    let mut cmd = Command::new(bin_path);
    cmd.arg("-C").arg(temp_dir.path()).arg("-a").arg("-y");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(
        stderr_out.contains("no tracked changes to commit"),
        "expected 'no tracked changes to commit' in stderr, got: {stderr_out}"
    );
    let head = repo.head()?.peel_to_commit()?;
    assert_eq!(head.summary(), Ok(Some("docs: initial")));

    temp_dir.close()?;
    Ok(())
}

#[test]
fn test_all_in_fresh_repo_with_only_untracked_errors() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;

    fs::write(temp_dir.path().join("untracked.txt"), "u")?;

    let mut cmd = Command::new(bin_path);
    cmd.arg("-C").arg(temp_dir.path()).arg("--all").arg("-y");

    let cmd_out = cmd.output()?;
    let stderr_out = String::from_utf8(cmd_out.stderr)?;

    assert!(!cmd_out.status.success());
    assert!(
        stderr_out.contains("no tracked changes to commit"),
        "expected 'no tracked changes to commit' in stderr, got: {stderr_out}"
    );
    assert!(repo.head().is_err(), "no commit must have been created");

    temp_dir.close()?;
    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_no_verify_skips_post_commit_hook() -> Result<(), Box<dyn Error>> {
    let (bin_path, temp_dir, repo) = setup_test_dir()?;
    let config_temp_dir = setup_config_home()?;

    fs::write(temp_dir.path().join("README.md"), "x")?;
    git_add(&repo, ".")?;
    do_initial_commit(&repo, "docs: initial")?;

    let sentinel = temp_dir.path().join("post-commit-fired");
    let hook_body = format!("#!/bin/sh\ntouch {}\n", sentinel.display());
    install_hook(&temp_dir, "post-commit", &hook_body)?;

    fs::write(temp_dir.path().join("a.txt"), "x")?;
    git_add(&repo, ".")?;

    let mut cmd = Command::new(bin_path);
    cmd.env("NO_COLOR", "1")
        .arg("-C")
        .arg(temp_dir.path())
        .arg("--no-verify")
        .arg("-y")
        .arg("--autocomplete=true");

    let mut process = spawn_command(cmd, Some(5000))?;
    drive_simple_fix_commit(&mut process, "no post hook")?;
    let eof_output = process.exp_eof();

    let exitcode = process.process().wait()?;
    let success = matches!(exitcode, WaitStatus::Exited(_, 0));
    if !success {
        panic!("Command exited non-zero, end of output: {eof_output:#?}");
    }

    assert!(
        !sentinel.exists(),
        "post-commit hook ran despite --no-verify"
    );
    let commit = get_last_commit(&repo)?;
    assert_eq!(commit.summary(), Ok(Some("fix: no post hook")));

    temp_dir.close()?;
    config_temp_dir.close()?;
    Ok(())
}
