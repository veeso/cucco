//! Layered configuration loaded from defaults, config files, and CLI overrides.

use std::env::current_dir;
use std::fmt;
use std::path::PathBuf;

use anyhow::Result;
use config::FileFormat;
use dirs::config_dir;
use indexmap::IndexMap;
use serde::Deserialize;
#[cfg(any(unix, target_os = "redox"))]
use xdg::BaseDirectories;

/// Resolved cucco configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether the scope prompt autocompletes from commit history.
    pub autocomplete: bool,
    /// Whether to ask about breaking changes.
    pub breaking_changes: bool,
    /// Available commit types, keyed by name.
    pub commit_types: IndexMap<String, CommitType>,
    /// Configured commit scopes, keyed by name.
    pub commit_scopes: IndexMap<String, CommitScope>,
    /// Whether to prepend the type emoji to the summary.
    pub emoji: bool,
    /// Whether to ask about related issues.
    pub issues: bool,
    /// Whether to sign the commit.
    pub sign: bool,
    /// Whether the scope must be picked from `commit_scopes`.
    pub force_config_scopes: bool,
    /// Whether the scope may be left empty.
    pub allow_empty_scope: bool,
    /// Directory cucco was started in.
    pub workdir: PathBuf,
}

/// A commit type such as `feat` or `fix`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommitType {
    /// Human-readable description shown in the type prompt.
    pub description: String,
    /// Emoji prepended to the summary when emoji are enabled.
    pub emoji: Option<String>,
    /// Type name written in the commit message.
    pub name: String,
}

/// A configured commit scope, optionally with path patterns and/or an AST-grep rule
/// for automatic scope detection from staged changes.
#[derive(Clone, Deserialize)]
pub struct CommitScope {
    /// Scope name written in the commit message.
    pub name: String,
    /// Human-readable description shown in the scope prompt.
    pub description: Option<String>,

    /// Regex patterns matched against staged file paths (prefixed with `/`).
    /// In TOML this may be given as a single string or a list of strings.
    #[serde(default, deserialize_with = "string_or_seq")]
    pub patterns: Vec<String>,

    /// AST-grep rule that pre-assigns this scope when it matches staged file content.
    #[cfg(feature = "ast-grep")]
    #[serde(default)]
    pub ast_grep: Option<ast_grep_config::SerializableRuleConfig<ast_grep_language::SupportLang>>,
}

// `SerializableRuleConfig` implements neither `Debug` nor `PartialEq`, so we can't
// derive `Debug` for `CommitScope` (needed because `Config` derives it).
impl fmt::Debug for CommitScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommitScope")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("patterns", &self.patterns)
            .finish_non_exhaustive()
    }
}

/// Accept either a single string or a list of strings for a `Vec<String>` field.
fn string_or_seq<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }

    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(pattern) => vec![pattern],
        OneOrMany::Many(patterns) => patterns,
    })
}

#[derive(Clone, Debug, Deserialize)]
struct ConfigTOML {
    autocomplete: bool,
    breaking_changes: bool,
    #[serde(default)]
    commit_types: Vec<CommitType>,
    #[serde(default)]
    commit_scopes: Vec<CommitScope>,
    emoji: bool,
    issues: bool,
    sign: bool,
    force_config_scopes: bool,
    allow_empty_scope: bool,
}

/// Overrides applied on top of the config files, and where to look for them.
#[derive(Debug, Default)]
pub struct ConfigArgs {
    /// Extra config file, read last among the files.
    pub path: Option<PathBuf>,
    /// Overrides `autocomplete`.
    pub autocomplete: Option<bool>,
    /// Overrides `breaking_changes`.
    pub breaking_changes: Option<bool>,
    /// Overrides `emoji`.
    pub emoji: Option<bool>,
    /// Overrides `issues`.
    pub issues: Option<bool>,
    /// Overrides `sign`.
    pub sign: Option<bool>,
    /// Overrides `force_config_scopes`.
    pub force_scope: Option<bool>,
    /// Overrides `allow_empty_scope`.
    pub allow_empty_scope: Option<bool>,
    /// Extra directory searched for `cucco/config.toml`, meant for tests.
    pub _user_config_path: Option<PathBuf>,
    /// Working directory to use instead of the process one.
    pub _current_dir: Option<PathBuf>,
}

impl Config {
    /// Finds the config files, merges them, and applies the overrides in `args`.
    ///
    /// # Errors
    ///
    /// Returns an error if the working directory cannot be determined, a config
    /// file is malformed, or a scope pattern or ast-grep rule is invalid.
    pub fn new(args: Option<ConfigArgs>) -> Result<Self> {
        let ConfigArgs {
            path,
            autocomplete,
            breaking_changes,
            emoji,
            issues,
            sign,
            force_scope,
            allow_empty_scope,
            _user_config_path,
            _current_dir,
        } = args.unwrap_or_default();

        let mut settings = config::Config::builder();

        let workdir = match _current_dir {
            Some(dir) => dir,
            None => current_dir()?,
        };

        // Get the default config
        let default_str = include_str!("../../meta/config/default.toml");
        settings = settings.add_source(config::File::from_str(default_str, FileFormat::Toml));

        // Define the order in which configuration directories will be loaded
        let mut config_dirs = vec![config_dir()];
        #[cfg(any(unix, target_os = "redox"))]
        config_dirs.push(BaseDirectories::new().get_config_home());
        config_dirs.push(_user_config_path);

        settings = config_dirs
            .into_iter()
            .flatten()
            .map(|d| d.join("cucco/config.toml"))
            .map(|d| config::File::from(d).required(false))
            .fold(settings, |prev, cfg| prev.add_source(cfg));

        // Try to get config from working directory
        let working_dir_path = workdir.join(".cucco.toml");
        settings = settings.add_source(config::File::from(working_dir_path).required(false));

        // Try to get config from the file passed with --config
        if let Some(path) = path {
            settings = settings.add_source(config::File::from(path).required(false));
        }

        let config: ConfigTOML = settings.build()?.try_deserialize()?;

        // Gather up commit types
        let commit_types = config
            .commit_types
            .into_iter()
            .map(|commit_type| (commit_type.name.clone(), commit_type))
            .collect();

        // Gather up commit scopes (patterns and ast_grep are inline on each scope)
        let commit_scopes = config
            .commit_scopes
            .into_iter()
            .map(|commit_scope| (commit_scope.name.clone(), commit_scope))
            .collect();

        let config = Config {
            autocomplete: autocomplete.unwrap_or(config.autocomplete),
            breaking_changes: breaking_changes.unwrap_or(config.breaking_changes),
            commit_types,
            commit_scopes,
            emoji: emoji.unwrap_or(config.emoji),
            issues: issues.unwrap_or(config.issues),
            sign: sign.unwrap_or(config.sign),
            force_config_scopes: force_scope.unwrap_or(config.force_config_scopes),
            allow_empty_scope: allow_empty_scope.unwrap_or(config.allow_empty_scope),
            workdir,
        };

        config.validate_scope_patterns()?;
        #[cfg(feature = "ast-grep")]
        config.validate_ast_grep_rules()?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_from_path() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        std::fs::write(
            tempdir.path().join("my-cucco.toml"),
            "[[commit_types]]\nname=\"1234\"\ndescription=\"test\"",
        )?;

        let config = Config::new(Some(ConfigArgs {
            path: Some(tempdir.path().join("my-cucco.toml")),
            ..ConfigArgs::default()
        }));

        assert!(config.is_ok());
        assert!(config?.commit_types.get("1234").is_some());

        tempdir.close()?;

        Ok(())
    }

    #[test]
    fn test_local_config() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        std::fs::write(
            tempdir.path().join(".cucco.toml"),
            "[[commit_types]]\nname=\"123\"\ndescription=\"test\"",
        )?;

        let config = Config::new(Some(ConfigArgs {
            _current_dir: Some(tempdir.path().to_path_buf()),
            ..Default::default()
        }));

        assert!(config.is_ok());
        assert!(config?.commit_types.get("123").is_some());

        tempdir.close()?;

        Ok(())
    }

    #[test]
    fn test_user_config_config() -> Result<(), Box<dyn Error>> {
        let tempdir_current = tempfile::tempdir()?;
        let tempdir_config = tempfile::tempdir()?;
        std::fs::create_dir(tempdir_config.path().join("cucco"))?;
        std::fs::write(
            tempdir_config.path().join("cucco").join("config.toml"),
            "[[commit_types]]\nname=\"12345\"\ndescription=\"test\"",
        )?;

        let config = Config::new(Some(ConfigArgs {
            _user_config_path: Some(tempdir_config.path().to_path_buf()),
            _current_dir: Some(tempdir_current.path().to_path_buf()),
            ..Default::default()
        }));

        assert!(config.is_ok());
        assert!(config?.commit_types.get("12345").is_some());

        tempdir_current.close()?;
        tempdir_config.close()?;

        Ok(())
    }

    #[test]
    fn test_all_config_sources() -> Result<(), Box<dyn Error>> {
        let tempdir_config = tempfile::tempdir()?;
        std::fs::create_dir(tempdir_config.path().join("cucco"))?;
        std::fs::write(
            tempdir_config.path().join("cucco").join("config.toml"),
            "[[commit_types]]\nname=\"12345\"\ndescription=\"test\"",
        )?;
        let tempdir_current = tempfile::tempdir()?;
        std::fs::write(tempdir_current.path().join(".cucco.toml"), "emoji=\"true\"")?;
        let tempdir_path = tempfile::tempdir()?;
        std::fs::write(tempdir_path.path().join("custom.toml"), "autocomplete=true")?;

        let config = Config::new(Some(ConfigArgs {
            _user_config_path: Some(tempdir_config.path().to_path_buf()),
            _current_dir: Some(tempdir_current.path().to_path_buf()),
            path: Some(tempdir_path.path().join("custom.toml").to_path_buf()),
            emoji: Some(false),
            ..Default::default()
        }))?;

        // from user config dir
        assert!(config.commit_types.get("12345").is_some());
        assert!(config.commit_types.len() == 1);
        // set by current dir config and directly, which overwrites the former
        assert!(!config.emoji);
        // set by passed config path
        assert!(config.autocomplete);
        // a default
        assert!(!config.sign);

        tempdir_current.close()?;
        tempdir_config.close()?;
        tempdir_path.close()?;

        Ok(())
    }

    #[test]
    fn test_non_custom_use_defaults() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;

        let config = Config::new(Some(ConfigArgs {
            _current_dir: Some(tempdir.path().to_path_buf()),
            _user_config_path: Some(tempdir.path().to_path_buf()),
            ..Default::default()
        }));

        assert!(config.is_ok());
        assert!(!config?.commit_types.is_empty());

        tempdir.close()?;

        Ok(())
    }

    #[test]
    fn test_breaking_changes() -> Result<(), Box<dyn Error>> {
        let config = Config::new(None)?;
        assert!(config.breaking_changes);

        let config = Config::new(Some(ConfigArgs {
            breaking_changes: Some(false),
            ..Default::default()
        }))?;
        assert!(!config.breaking_changes);

        Ok(())
    }

    #[test]
    fn test_issues() -> Result<(), Box<dyn Error>> {
        let config = Config::new(None)?;
        assert!(config.issues);

        let config = Config::new(Some(ConfigArgs {
            issues: Some(false),
            ..Default::default()
        }))?;
        assert!(!config.issues);

        Ok(())
    }

    #[test]
    fn test_commit_types() -> Result<(), Box<dyn Error>> {
        let config = Config::new(None)?;
        let commit_types = config.commit_types;

        assert_eq!(
            commit_types.get("feat"),
            Some(&CommitType {
                name: "feat".into(),
                emoji: Some("✨".into()),
                description: "A new feature".into()
            })
        );

        Ok(())
    }

    #[test]
    fn test_commit_scopes() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        std::fs::write(
            tempdir.path().join(".cucco.toml"),
            "[[commit_scopes]]\nname=\"app\"\ndescription=\"Application code\"",
        )?;
        let config = Config::new(Some(ConfigArgs {
            _current_dir: Some(tempdir.path().to_path_buf()),
            ..Default::default()
        }))?;
        assert!(config.commit_scopes.get("app").is_some());
        let scope = config.commit_scopes.get("app").unwrap();
        assert_eq!(scope.name, "app");
        assert_eq!(scope.description, Some("Application code".into()));
        tempdir.close()?;
        Ok(())
    }

    #[test]
    fn test_commit_scopes_from_config() -> Result<(), Box<dyn Error>> {
        let tempdir_config = tempfile::tempdir()?;
        std::fs::create_dir(tempdir_config.path().join("cucco"))?;
        std::fs::write(
            tempdir_config.path().join("cucco").join("config.toml"),
            "[[commit_scopes]]\nname=\"server\"\ndescription=\"Server code\"\n[[commit_scopes]]\nname=\"shared\"",
        )?;
        let tempdir_current = tempfile::tempdir()?;
        let config = Config::new(Some(ConfigArgs {
            _user_config_path: Some(tempdir_config.path().to_path_buf()),
            _current_dir: Some(tempdir_current.path().to_path_buf()),
            ..Default::default()
        }))?;
        assert!(config.commit_scopes.get("server").is_some());
        assert!(config.commit_scopes.get("shared").is_some());
        assert_eq!(config.commit_scopes.len(), 2);
        tempdir_current.close()?;
        tempdir_config.close()?;
        Ok(())
    }

    #[test]
    fn test_scope_patterns_inline() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        std::fs::write(
            tempdir.path().join(".cucco.toml"),
            "[[commit_scopes]]\nname=\"core\"\ndescription=\"Core crate\"\npatterns = \"/crates/core/**/*.rs\"\n[[commit_scopes]]\nname=\"build\"\npatterns = [\"^/build\\\\.rs$\", \"/justfile\"]",
        )?;

        let config = Config::new(Some(ConfigArgs {
            _current_dir: Some(tempdir.path().to_path_buf()),
            ..Default::default()
        }))?;

        assert!(config.commit_scopes.contains_key("core"));
        assert!(config.commit_scopes.contains_key("build"));

        let core = config.commit_scopes.get("core").unwrap();
        assert_eq!(core.description, Some("Core crate".into()));
        assert_eq!(core.patterns, vec!["/crates/core/**/*.rs".to_string()]);

        let build = config.commit_scopes.get("build").unwrap();
        assert_eq!(build.patterns, vec!["^/build\\.rs$", "/justfile"]);

        tempdir.close()?;
        Ok(())
    }

    #[cfg(feature = "ast-grep")]
    #[test]
    fn test_scope_ast_grep_inline() -> Result<(), Box<dyn Error>> {
        let tempdir = tempfile::tempdir()?;
        // Use a subtable for ast_grep within the commit_scopes array entry
        std::fs::write(
            tempdir.path().join(".cucco.toml"),
            "[[commit_scopes]]\nname = \"test\"\ndescription = \"Test functions\"\n\n[commit_scopes.ast_grep]\nlanguage = \"Rust\"\nrule = { kind = \"function_item\" }\nfiles = [\"**/*.rs\"]\n",
        )?;

        let config = Config::new(Some(ConfigArgs {
            _current_dir: Some(tempdir.path().to_path_buf()),
            ..Default::default()
        }))?;

        assert!(config.commit_scopes.contains_key("test"));
        let scope = config.commit_scopes.get("test").unwrap();
        assert_eq!(scope.description, Some("Test functions".into()));
        assert!(scope.ast_grep.is_some());

        tempdir.close()?;
        Ok(())
    }
}
