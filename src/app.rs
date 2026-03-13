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
            WorkCmds::List(args) => {
                let workspaces = self.db.list_workspaces()?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&workspaces)?);
                } else {
                    for ws in &workspaces {
                        println!("{}\t{}", ws.name, ws.description);
                    }
                }
                Ok(())
            }
            WorkCmds::Create(args) => {
                if args.name.is_empty() {
                    anyhow::bail!("workspace name must not be empty");
                }
                let desc = args.description.as_deref().unwrap_or("");
                let ws = self.db.create_workspace(&args.name, desc)?;
                println!("Created workspace: {}", ws.name);
                Ok(())
            }
            WorkCmds::Show(args) => {
                let ws = self
                    .db
                    .get_workspace_by_name(&args.name)?
                    .ok_or_else(|| anyhow::anyhow!("workspace '{}' not found", args.name))?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&ws)?);
                } else {
                    println!("Name:        {}", ws.name);
                    println!("Description: {}", ws.description);
                    println!("Default Env: {}", ws.default_env.map_or("(none)".into(), |id| id.to_string()));
                    println!("Created:     {}", ws.created_at);
                    println!("Updated:     {}", ws.updated_at);
                }
                Ok(())
            }
            WorkCmds::Update(args) => {
                let ws = self
                    .db
                    .get_workspace_by_name(&args.name)?
                    .ok_or_else(|| anyhow::anyhow!("workspace '{}' not found", args.name))?;
                let new_name = args.new_name.as_deref().unwrap_or(&ws.name);
                let new_desc = args.new_description.as_deref().unwrap_or(&ws.description);
                self.db.update_workspace(ws.id, new_name, new_desc)?;
                if let Some(env_name) = &args.default_env {
                    let env = self
                        .db
                        .get_environment_by_name(ws.id, env_name)?
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "environment '{}' not found in workspace '{}'",
                                env_name,
                                ws.name
                            )
                        })?;
                    self.db.set_workspace_default_env(ws.id, Some(env.id))?;
                }
                println!("Updated workspace: {}", new_name);
                Ok(())
            }
            WorkCmds::Del(args) => {
                let ws = self
                    .db
                    .get_workspace_by_name(&args.name)?
                    .ok_or_else(|| anyhow::anyhow!("workspace '{}' not found", args.name))?;
                if !args.force {
                    let colls = self.db.list_collections(ws.id)?.len();
                    let envs = self.db.list_environments(ws.id)?.len();
                    let confirmed = inquire::Confirm::new(&format!(
                        "Delete workspace '{}' ({} collection(s), {} environment(s))?",
                        ws.name, colls, envs
                    ))
                    .with_default(false)
                    .prompt()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                }
                self.db.delete_workspace(ws.id)?;
                println!("Deleted workspace: {}", ws.name);
                Ok(())
            }
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
            ConfCmds::Init(args) => {
                let config_path = conf::config_path();
                let db_path = self
                    .config
                    .database
                    .as_ref()
                    .and_then(|d| d.path.as_ref())
                    .map(PathBuf::from)
                    .unwrap_or_else(conf::default_db_path);

                if config_path.exists() && !args.force {
                    anyhow::bail!(
                        "Config file already exists: {}\nUse --force to overwrite.",
                        config_path.display()
                    );
                }

                if args.force {
                    if config_path.exists() {
                        fs::remove_file(&config_path)?;
                    }
                    if db_path.exists() {
                        fs::remove_file(&db_path)?;
                    }
                }

                let default_db_path = conf::default_db_path();
                let config = AppConfig {
                    database: Some(conf::DatabaseConfig {
                        path: Some(
                            default_db_path
                                .to_str()
                                .expect("invalid db path")
                                .to_string(),
                        ),
                    }),
                    ..AppConfig::default()
                };
                conf::save(&config)?;
                println!("Created config file: {}", config_path.display());
                println!("Database file: {}", db_path.display());
                Ok(())
            }
            ConfCmds::Show => {
                // TODO: When --config CLI flag or YAPI_CONFIG env var is added,
                // update config_file source to reflect the override (e.g. "set via --config"
                // or "set via $YAPI_CONFIG") instead of always showing "using default".
                let config_file = conf::config_path();
                // TODO: When --db CLI flag or YAPI_DB env var is added,
                // add a source variant here for CLI/env overrides as well.
                let (db_file, db_source) = match self
                    .config
                    .database
                    .as_ref()
                    .and_then(|d| d.path.as_ref())
                {
                    Some(p) => (PathBuf::from(p), "set in config.toml"),
                    None => (conf::default_db_path(), "using default"),
                };
                println!(
                    "config_file = {:?} # using default",
                    config_file.display()
                );
                println!("db_file = {:?} # {db_source}", db_file.display());
                println!();
                let output = toml::to_string_pretty(&self.config)
                    .context("failed to serialize config")?;
                print!("{output}");
                Ok(())
            }
            ConfCmds::Get(args) => {
                if let Some(value) = conf::get_value(&self.config, &args.key)? {
                    println!("{value}");
                }
                Ok(())
            }
            ConfCmds::Set(args) => {
                let mut config = self.config;
                conf::set_value(&mut config, &args.key, &args.value)?;
                conf::save(&config)?;
                Ok(())
            }
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

    /// Helper: create an App with a temp dir and in-memory-like DB for testing.
    fn test_app() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
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
        (dir, app)
    }

    fn parse_cli(args: &[&str]) -> Cli {
        use clap::Parser;
        Cli::parse_from(std::iter::once("yapi").chain(args.iter().copied()))
    }

    #[test]
    fn test_work_create_and_list() {
        let (_dir, app) = test_app();

        // Create a workspace
        app.db.create_workspace("test-ws", "A test workspace").unwrap();

        let workspaces = app.db.list_workspaces().unwrap();
        // Should have the default workspace + the new one
        let names: Vec<&str> = workspaces.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"test-ws"));
    }

    #[test]
    fn test_work_create_empty_name_fails() {
        let (_dir, app) = test_app();
        let cli = parse_cli(&["work", "create", ""]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must not be empty"));
    }

    #[test]
    fn test_work_show_found() {
        let (_dir, app) = test_app();
        app.db.create_workspace("my-ws", "desc").unwrap();
        let cli = parse_cli(&["work", "show", "my-ws"]);
        // Should succeed without error
        app.run(cli).unwrap();
    }

    #[test]
    fn test_work_show_not_found() {
        let (_dir, app) = test_app();
        let cli = parse_cli(&["work", "show", "nonexistent"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_work_update_name_and_description() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        app.db.create_workspace("old-name", "old desc").unwrap();
        let cli = parse_cli(&[
            "work", "update", "old-name",
            "--new-name", "new-name",
            "--new-description", "new desc",
        ]);
        app.run(cli).unwrap();

        // Reconnect to verify
        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("new-name").unwrap();
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().description, "new desc");
        assert!(db.get_workspace_by_name("old-name").unwrap().is_none());
    }

    #[test]
    fn test_work_update_default_env() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws1", "").unwrap();
        app.db.create_environment(ws.id, "staging", "").unwrap();
        let cli = parse_cli(&["work", "update", "ws1", "--default-env", "staging"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws1").unwrap().unwrap();
        assert!(ws.default_env.is_some());
    }

    #[test]
    fn test_work_delete_force() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        app.db.create_workspace("to-delete", "").unwrap();
        let cli = parse_cli(&["work", "del", "to-delete", "--force"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        assert!(db.get_workspace_by_name("to-delete").unwrap().is_none());
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
