//! Application config types and loading logic.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Abstraction over environment variable access for testability.
pub trait Env {
    fn get(&self, key: &str) -> Option<String>;
}

/// Production implementation that reads real environment variables.
pub struct RealEnv;

impl Env for RealEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: Option<DatabaseConfig>,
    pub defaults: Option<DefaultsConfig>,
    pub history: Option<HistoryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    pub workspace: Option<String>,
    pub collection: Option<String>,
    pub environment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub retention_days: Option<u32>,
}

/// Returns the config path using the given `Env` for variable lookups.
pub fn config_path_with(env: &dyn Env) -> PathBuf {
    let base = env
        .get("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = env.get("HOME").expect("HOME not set");
            PathBuf::from(home).join(".config")
        });
    base.join("yapi").join("config.toml")
}

/// Returns the path to the global config file.
///
/// Uses `$XDG_CONFIG_HOME/yapi/config.toml` if set,
/// otherwise `$HOME/.config/yapi/config.toml`.
pub fn config_path() -> PathBuf {
    config_path_with(&RealEnv)
}

/// Returns the default database path using the given `Env` for variable lookups.
pub fn default_db_path_with(env: &dyn Env) -> PathBuf {
    let base = env
        .get("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = env.get("HOME").expect("HOME not set");
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("yapi").join("yapi.db")
}

/// Returns the default database path: `~/.local/share/yapi/yapi.db`.
pub fn default_db_path() -> PathBuf {
    default_db_path_with(&RealEnv)
}

/// Load the global config file. Returns defaults if the file doesn't exist.
pub fn load() -> Result<AppConfig> {
    load_from(&config_path())
}

/// Load config from a specific path. Returns defaults if the file doesn't exist.
pub fn load_from(path: &std::path::Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config: AppConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Save the config to the global config file. Creates parent dirs if needed.
pub fn save(config: &AppConfig) -> Result<()> {
    save_to(config, &config_path())
}

/// Save the config to a specific path. Creates parent dirs if needed.
pub fn save_to(config: &AppConfig, path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }
    let contents = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write config file: {}", path.display()))?;
    Ok(())
}

/// Get a config value by dotted key (e.g. "defaults.workspace").
/// Returns `Ok(None)` if the section or field is unset.
pub fn get_value(config: &AppConfig, key: &str) -> Result<Option<String>> {
    match key {
        "database.path" => Ok(config.database.as_ref().and_then(|d| d.path.clone())),
        "defaults.workspace" => Ok(config.defaults.as_ref().and_then(|d| d.workspace.clone())),
        "defaults.collection" => Ok(config.defaults.as_ref().and_then(|d| d.collection.clone())),
        "defaults.environment" => Ok(config.defaults.as_ref().and_then(|d| d.environment.clone())),
        "history.retention_days" => Ok(config
            .history
            .as_ref()
            .and_then(|h| h.retention_days)
            .map(|v| v.to_string())),
        _ => anyhow::bail!("unknown config key: {key}"),
    }
}

/// Set a config value by dotted key. Initializes parent sections if needed.
pub fn set_value(config: &mut AppConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "database.path" => {
            config
                .database
                .get_or_insert_with(|| DatabaseConfig { path: None })
                .path = Some(value.to_string());
        }
        "defaults.workspace" => {
            config
                .defaults
                .get_or_insert_with(|| DefaultsConfig {
                    workspace: None,
                    collection: None,
                    environment: None,
                })
                .workspace = Some(value.to_string());
        }
        "defaults.collection" => {
            config
                .defaults
                .get_or_insert_with(|| DefaultsConfig {
                    workspace: None,
                    collection: None,
                    environment: None,
                })
                .collection = Some(value.to_string());
        }
        "defaults.environment" => {
            config
                .defaults
                .get_or_insert_with(|| DefaultsConfig {
                    workspace: None,
                    collection: None,
                    environment: None,
                })
                .environment = Some(value.to_string());
        }
        "history.retention_days" => {
            let days: u32 = value
                .parse()
                .with_context(|| format!("invalid u32 value for {key}: {value}"))?;
            config
                .history
                .get_or_insert_with(|| HistoryConfig {
                    retention_days: None,
                })
                .retention_days = Some(days);
        }
        _ => anyhow::bail!("unknown config key: {key}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent").join("config.toml");
        let config = load_from(&path).unwrap();
        assert!(config.database.is_none());
        assert!(config.defaults.is_none());
        assert!(config.history.is_none());
    }

    #[test]
    fn test_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("yapi").join("config.toml");

        let config = AppConfig {
            database: Some(DatabaseConfig {
                path: Some("/tmp/test.db".into()),
            }),
            defaults: Some(DefaultsConfig {
                workspace: Some("personal".into()),
                collection: None,
                environment: None,
            }),
            history: Some(HistoryConfig {
                retention_days: Some(90),
            }),
        };

        save_to(&config, &path).unwrap();
        let loaded = load_from(&path).unwrap();

        let db = loaded.database.unwrap();
        assert_eq!(db.path.unwrap(), "/tmp/test.db");

        let defaults = loaded.defaults.unwrap();
        assert_eq!(defaults.workspace.unwrap(), "personal");
        assert!(defaults.collection.is_none());

        let history = loaded.history.unwrap();
        assert_eq!(history.retention_days.unwrap(), 90);
    }

    #[test]
    fn test_get_value_unset() {
        let config = AppConfig::default();
        assert_eq!(get_value(&config, "defaults.workspace").unwrap(), None);
        assert_eq!(get_value(&config, "database.path").unwrap(), None);
        assert_eq!(get_value(&config, "history.retention_days").unwrap(), None);
    }

    #[test]
    fn test_get_set_roundtrip() {
        let mut config = AppConfig::default();
        set_value(&mut config, "defaults.workspace", "personal").unwrap();
        assert_eq!(
            get_value(&config, "defaults.workspace").unwrap().as_deref(),
            Some("personal")
        );
    }

    #[test]
    fn test_set_retention_days_parses_u32() {
        let mut config = AppConfig::default();
        set_value(&mut config, "history.retention_days", "90").unwrap();
        assert_eq!(
            get_value(&config, "history.retention_days")
                .unwrap()
                .as_deref(),
            Some("90")
        );
    }

    #[test]
    fn test_set_retention_days_rejects_non_numeric() {
        let mut config = AppConfig::default();
        assert!(set_value(&mut config, "history.retention_days", "abc").is_err());
    }

    #[test]
    fn test_unknown_key_errors() {
        let config = AppConfig::default();
        assert!(get_value(&config, "bad.key").is_err());
        let mut config2 = AppConfig::default();
        assert!(set_value(&mut config2, "bad.key", "val").is_err());
    }

    #[test]
    fn test_partial_toml() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("yapi");
        fs::create_dir_all(&config_dir).unwrap();

        let path = config_dir.join("config.toml");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "[defaults]\nworkspace = \"work\"").unwrap();

        let config = load_from(&path).unwrap();
        assert!(config.database.is_none());
        assert_eq!(
            config.defaults.as_ref().unwrap().workspace.as_deref(),
            Some("work")
        );
        assert!(config.history.is_none());
    }
}
