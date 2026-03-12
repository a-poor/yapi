//! The core app code

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::*;
use crate::conf::{self, AppConfig, Env, RealEnv};
use crate::db::DBClient;

/// The core app type that owns config and database.
pub struct App {
    pub config: AppConfig,
    pub db: DBClient,
}

impl App {
    /// Production constructor. Accepts an optional config path override (e.g. from `--config`).
    pub fn new(config_path: Option<PathBuf>) -> Result<Self> {
        Self::new_with_env(&RealEnv, config_path)
    }

    /// Testable constructor with a custom `Env` and optional config path override.
    pub fn new_with_env(env: &dyn Env, config_path: Option<PathBuf>) -> Result<Self> {
        let config = Self::load_config(env, config_path.as_deref())?;
        let db = Self::connect_db(env, &config)?;
        Ok(Self { config, db })
    }

    /// Load config from the given path, or fall back to the env-derived default path.
    /// Returns `AppConfig::default()` if the file doesn't exist.
    fn load_config(env: &dyn Env, config_path: Option<&std::path::Path>) -> Result<AppConfig> {
        let path = match config_path {
            Some(p) => p.to_path_buf(),
            None => conf::config_path_with(env),
        };
        conf::load_from(&path)
    }

    /// Open (and migrate) the database. Resolution order:
    /// 1. `config.database.path` from the loaded config
    /// 2. `default_db_path_with(env)` fallback
    fn connect_db(env: &dyn Env, config: &AppConfig) -> Result<DBClient> {
        let db_path = config
            .database
            .as_ref()
            .and_then(|d| d.path.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| conf::default_db_path_with(env));

        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create database directory: {}",
                    parent.display()
                )
            })?;
        }

        let db = DBClient::new(Some(db_path.to_str().expect("invalid db path")))?;
        db.migrate()?;
        Ok(db)
    }

    /// Dispatch CLI commands to handler methods.
    pub fn run(self, cli: Cli) -> Result<()> {
        match cli.cmd {
            RootCmds::Req(args) => self.run_req(args),
            RootCmds::Coll(args) => self.run_coll(args),
            RootCmds::Env(args) => self.run_env(args),
            RootCmds::Work(args) => self.run_work(args),
            RootCmds::Spec => self.run_spec(),
            RootCmds::Hist(args) => self.run_hist(args),
            RootCmds::Conf(args) => self.run_conf(args),
        }
    }

    // ── Request handlers ────────────────────────────────────────

    fn run_req(self, args: ReqArgs) -> Result<()> {
        match args.cmd {
            ReqCmds::List(_) => todo!("req list"),
            ReqCmds::Create(_) => todo!("req create"),
            ReqCmds::Show(_) => todo!("req show"),
            ReqCmds::Run(_) => todo!("req run"),
            ReqCmds::Update(_) => todo!("req update"),
            ReqCmds::Del(_) => todo!("req del"),
        }
    }

    // ── Collection handlers ─────────────────────────────────────

    fn run_coll(self, args: CollArgs) -> Result<()> {
        match args.cmd {
            CollCmds::List(_) => todo!("coll list"),
            CollCmds::Create(_) => todo!("coll create"),
            CollCmds::Show(_) => todo!("coll show"),
            CollCmds::Update(_) => todo!("coll update"),
            CollCmds::Del(_) => todo!("coll del"),
        }
    }

    // ── Environment handlers ────────────────────────────────────

    fn run_env(self, args: EnvArgs) -> Result<()> {
        match args.cmd {
            EnvCmds::List(_) => todo!("env list"),
            EnvCmds::Create(_) => todo!("env create"),
            EnvCmds::Show(_) => todo!("env show"),
            EnvCmds::Update(_) => todo!("env update"),
            EnvCmds::Del(_) => todo!("env del"),
            EnvCmds::Vars(vars) => self.run_env_vars(vars),
        }
    }

    fn run_env_vars(self, args: EnvVarArgs) -> Result<()> {
        match args.cmd {
            EnvVarCmds::List(_) => todo!("env vars list"),
            EnvVarCmds::Create(_) => todo!("env vars create"),
            EnvVarCmds::Show(_) => todo!("env vars show"),
            EnvVarCmds::Update(_) => todo!("env vars update"),
            EnvVarCmds::Del(_) => todo!("env vars del"),
        }
    }

    // ── Workspace handlers ──────────────────────────────────────

    fn run_work(self, args: WorkArgs) -> Result<()> {
        match args.cmd {
            WorkCmds::List(_) => todo!("work list"),
            WorkCmds::Create(_) => todo!("work create"),
            WorkCmds::Show(_) => todo!("work show"),
            WorkCmds::Update(_) => todo!("work update"),
            WorkCmds::Del(_) => todo!("work del"),
        }
    }

    // ── Spec handler ────────────────────────────────────────────

    fn run_spec(self) -> Result<()> {
        todo!("spec")
    }

    // ── History handlers ────────────────────────────────────────

    fn run_hist(self, args: HistArgs) -> Result<()> {
        match args.cmd {
            HistCmds::List(_) => todo!("hist list"),
            HistCmds::Show(_) => todo!("hist show"),
            HistCmds::Del(_) => todo!("hist del"),
        }
    }

    // ── Config handlers ─────────────────────────────────────────

    fn run_conf(self, args: ConfArgs) -> Result<()> {
        match args.cmd {
            ConfCmds::Show => todo!("conf show"),
            ConfCmds::Set(_) => todo!("conf set"),
            ConfCmds::Get(_) => todo!("conf get"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeEnv(HashMap<String, String>);

    impl Env for FakeEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
    }

    #[test]
    fn test_missing_config_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("nonexistent").join("config.toml");

        let env = FakeEnv(HashMap::from([(
            "HOME".into(),
            dir.path().to_str().unwrap().into(),
        )]));
        let app = App::new_with_env(&env, Some(config_path)).unwrap();

        assert!(app.config.database.is_none());
        assert!(app.config.defaults.is_none());
        assert!(app.config.history.is_none());
    }

    #[test]
    fn test_config_db_path_used() {
        let dir = tempfile::tempdir().unwrap();

        // Write a config that specifies a db path
        let db_path = dir.path().join("from-config.db");
        let config_path = dir.path().join("config.toml");
        std::fs::write(
            &config_path,
            format!("[database]\npath = {:?}\n", db_path.to_str().unwrap()),
        )
        .unwrap();

        let env = FakeEnv(HashMap::from([(
            "HOME".into(),
            dir.path().to_str().unwrap().into(),
        )]));
        let app = App::new_with_env(&env, Some(config_path)).unwrap();

        assert!(db_path.exists());
        assert_eq!(
            app.config.database.unwrap().path.unwrap(),
            db_path.to_str().unwrap()
        );
    }

    #[test]
    fn test_default_db_path_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let env = FakeEnv(HashMap::from([(
            "HOME".into(),
            dir.path().to_str().unwrap().into(),
        )]));
        let app = App::new_with_env(&env, Some(config_path)).unwrap();

        // Should have created the db at the default XDG path under HOME
        let expected = dir
            .path()
            .join(".local")
            .join("share")
            .join("yapi")
            .join("yapi.db");
        assert!(expected.exists());
        assert!(app.config.database.is_none());
    }
}
