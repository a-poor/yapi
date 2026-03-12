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
    /// Production constructor using real environment and default paths.
    pub fn new() -> Result<Self> {
        Self::create(&RealEnv, None, None)
    }

    /// Testable constructor with full control over environment and paths.
    pub fn create(
        env: &dyn Env,
        config_path: Option<PathBuf>,
        db_path: Option<PathBuf>,
    ) -> Result<Self> {
        let config_path = config_path.unwrap_or_else(|| conf::config_path_with(env));
        let config = conf::load_from(&config_path)?;

        let db_path = db_path
            .or_else(|| {
                config
                    .database
                    .as_ref()
                    .and_then(|d| d.path.as_ref())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| conf::default_db_path_with(env));

        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create database directory: {}", parent.display()))?;
        }
        let db = DBClient::new(Some(db_path.to_str().expect("invalid db path")))?;
        db.migrate()?;

        Ok(Self { config, db })
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
    fn test_create_with_missing_config_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config_path = dir.path().join("nonexistent").join("config.toml");

        let env = FakeEnv(HashMap::new());
        let app = App::create(&env, Some(config_path), Some(db_path)).unwrap();

        assert!(app.config.database.is_none());
        assert!(app.config.defaults.is_none());
        assert!(app.config.history.is_none());
    }

    #[test]
    fn test_create_with_explicit_db_path() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("explicit.db");
        let config_path = dir.path().join("config.toml");

        let env = FakeEnv(HashMap::new());
        let app = App::create(&env, Some(config_path), Some(db_path.clone())).unwrap();

        // DB should have been created at the explicit path
        assert!(db_path.exists());
        // Config should be defaults since file doesn't exist
        assert!(app.config.database.is_none());
    }
}
