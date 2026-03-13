//! The core app code

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::*;
use crate::conf::{self, AppConfig, Env, RealEnv};
use crate::db::DBClient;
use crate::dtypes::{Collection, CreateHistoryEntry, Workspace};
use crate::vars;

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
            ReqCmds::List(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self.resolve_collection(ws.id, args.collection.as_deref())?;
                let requests = self.db.list_requests(coll.id)?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&requests)?);
                } else {
                    for r in &requests {
                        println!("{}\t{}\t{}", r.name, r.method.as_str(), r.url);
                    }
                }
                Ok(())
            }
            ReqCmds::Create(args) => {
                if args.name.is_empty() {
                    anyhow::bail!("request name must not be empty");
                }
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self.resolve_collection(ws.id, args.collection.as_deref())?;
                let method = args.method.as_deref().unwrap_or("GET");
                let req = self.db.create_request(
                    coll.id,
                    &args.name,
                    method,
                    &args.url,
                    args.body.as_deref(),
                )?;
                for h in &args.headers {
                    let (key, value) = h.split_once(": ").ok_or_else(|| {
                        anyhow::anyhow!("invalid header format '{}', expected 'Key: Value'", h)
                    })?;
                    self.db.create_request_header(req.id, key, value)?;
                }
                for q in &args.queries {
                    let (key, value) = q.split_once('=').ok_or_else(|| {
                        anyhow::anyhow!(
                            "invalid query param format '{}', expected 'key=value'",
                            q
                        )
                    })?;
                    self.db.create_request_query_param(req.id, key, value)?;
                }
                println!("Created request: {}", req.name);
                Ok(())
            }
            ReqCmds::Show(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self.resolve_collection(ws.id, args.collection.as_deref())?;
                let req = self
                    .db
                    .get_request_by_name(coll.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "request '{}' not found in collection '{}'",
                            args.name,
                            coll.name
                        )
                    })?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&req)?);
                } else {
                    let headers = self.db.list_request_headers(req.id)?;
                    let params = self.db.list_request_query_params(req.id)?;
                    println!("Name:    {}", req.name);
                    println!("Method:  {}", req.method.as_str());
                    println!("URL:     {}", req.url);
                    println!(
                        "Body:    {}",
                        req.body.as_deref().unwrap_or("(none)")
                    );
                    println!("Headers:");
                    for h in &headers {
                        println!("  {}: {}", h.hkey, h.hval);
                    }
                    println!("Query Params:");
                    for p in &params {
                        println!("  {}={}", p.qkey, p.qval);
                    }
                    println!("Created: {}", req.created_at);
                    println!("Updated: {}", req.updated_at);
                }
                Ok(())
            }
            ReqCmds::Update(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self.resolve_collection(ws.id, args.collection.as_deref())?;
                let req = self
                    .db
                    .get_request_by_name(coll.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "request '{}' not found in collection '{}'",
                            args.name,
                            coll.name
                        )
                    })?;
                let new_name = args.new_name.as_deref().unwrap_or(&req.name);
                let new_method = args
                    .new_method
                    .as_deref()
                    .unwrap_or(req.method.as_str());
                let new_url = args.new_url.as_deref().unwrap_or(&req.url);
                let new_body = args.new_body.as_deref().or(req.body.as_deref());
                self.db
                    .update_request(req.id, new_name, new_method, new_url, new_body)?;
                println!("Updated request: {}", new_name);
                Ok(())
            }
            ReqCmds::Del(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self.resolve_collection(ws.id, args.collection.as_deref())?;
                let req = self
                    .db
                    .get_request_by_name(coll.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "request '{}' not found in collection '{}'",
                            args.name,
                            coll.name
                        )
                    })?;
                if !args.force {
                    let confirmed = inquire::Confirm::new(&format!(
                        "Permanently delete request '{}'?",
                        req.name
                    ))
                    .with_default(false)
                    .prompt()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                }
                self.db.delete_request(req.id)?;
                println!("Deleted request: {}", req.name);
                Ok(())
            }
            ReqCmds::Run(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self.resolve_collection(ws.id, args.collection.as_deref())?;
                let req = self
                    .db
                    .get_request_by_name(coll.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "request '{}' not found in collection '{}'",
                            args.name,
                            coll.name
                        )
                    })?;

                let headers = self.db.list_request_headers(req.id)?;
                let qps = self.db.list_request_query_params(req.id)?;

                // Resolve environment variables
                let env_vars = if let Some(env_name) = &args.env {
                    let env = self
                        .db
                        .get_environment_by_name(ws.id, env_name)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("environment '{}' not found", env_name)
                        })?;
                    self.db.list_environment_vars(env.id)?
                } else if let Some(env_id) = coll.default_env {
                    match self.db.get_environment_by_id(env_id)? {
                        Some(env) => self.db.list_environment_vars(env.id)?,
                        None => vec![],
                    }
                } else {
                    vec![]
                };

                let coll_vars = self.db.list_collection_vars(coll.id)?;
                let mut var_map = vars::build_var_map(&coll_vars, &env_vars);

                // Apply ad-hoc variables before resolving
                for v in &args.vars {
                    if let Some((k, val)) = v.split_once('=') {
                        var_map.insert(k.to_string(), val.to_string());
                    } else {
                        anyhow::bail!("invalid --var format '{}', expected key=value", v);
                    }
                }

                let mut resolved = vars::resolve_request(&req, &headers, &qps, &var_map)?;

                // Apply overrides after resolving
                if let Some(url) = &args.url {
                    resolved.url = vars::fill(url, &var_map)?;
                }
                if let Some(m) = &args.method {
                    resolved.method = m.parse().context("invalid HTTP method")?;
                }
                if let Some(body) = &args.body {
                    resolved.body = Some(vars::fill(body, &var_map)?);
                }
                for h in &args.headers {
                    if let Some((k, v)) = h.split_once(':') {
                        let v = vars::fill(v.trim(), &var_map)?;
                        if let Some(entry) = resolved.headers.iter_mut().find(|(key, _)| key == k.trim()) {
                            entry.1 = v;
                        } else {
                            resolved.headers.push((k.trim().to_string(), v));
                        }
                    } else {
                        anyhow::bail!("invalid --header format '{}', expected 'Key: Value'", h);
                    }
                }
                for q in &args.queries {
                    if let Some((k, v)) = q.split_once('=') {
                        let v = vars::fill(v, &var_map)?;
                        resolved.query_params.push((k.to_string(), v));
                    } else {
                        anyhow::bail!("invalid --query format '{}', expected key=value", q);
                    }
                }

                if args.dry_run {
                    println!("{}", resolved.to_curl());
                    return Ok(());
                }

                // Send the request
                let client = reqwest::Client::new();
                let rt = tokio::runtime::Runtime::new()?;
                let result = rt.block_on(vars::send_request(&resolved, &client));

                match result {
                    Ok(response) => {
                        // Print curl-style output
                        // Build full URL with query params for display
                        let mut display_url = reqwest::Url::parse(&resolved.url)?;
                        if !resolved.query_params.is_empty() {
                            let mut pairs = display_url.query_pairs_mut();
                            for (k, v) in &resolved.query_params {
                                pairs.append_pair(k, v);
                            }
                            drop(pairs);
                        }
                        let host = display_url.host_str().unwrap_or("unknown").to_string();
                        let path_and_query = match display_url.query() {
                            Some(q) => format!("{}?{}", display_url.path(), q),
                            None => display_url.path().to_string(),
                        };

                        if !args.body_only {
                            println!("* Host {}", host);
                            println!(
                                "> {} {} HTTP/1.1",
                                resolved.method.as_str(),
                                path_and_query,
                            );
                            for (k, v) in &resolved.headers {
                                println!("> {}: {}", k, v);
                            }
                            println!(">");
                            println!(
                                "< {} {}",
                                response.http_version,
                                response.status,
                            );
                            for h in &response.headers {
                                println!("< {}: {}", h.key, h.value);
                            }
                            println!("<");
                        }
                        if !args.hide_body {
                            if let Some(body) = &response.body {
                                println!("{}", body);
                            }
                        }

                        // Save history
                        self.db.create_history(&CreateHistoryEntry {
                            req_id: Some(req.id),
                            method: req.method.as_str().to_string(),
                            resolved_url: resolved.url.clone(),
                            resolved_req_headers: resolved.to_header_json(),
                            resolved_req_body: resolved.body.clone(),
                            success: true,
                            res_status: Some(response.status),
                            res_body: response.body.clone(),
                            res_headers: serde_json::to_string(&response.headers)?,
                            res_duration: Some(response.duration_secs),
                        })?;

                        Ok(())
                    }
                    Err(err) => {
                        // Save failed history
                        self.db.create_history(&CreateHistoryEntry {
                            req_id: Some(req.id),
                            method: req.method.as_str().to_string(),
                            resolved_url: resolved.url.clone(),
                            resolved_req_headers: resolved.to_header_json(),
                            resolved_req_body: resolved.body.clone(),
                            success: false,
                            res_status: None,
                            res_body: None,
                            res_headers: "[]".to_string(),
                            res_duration: None,
                        })?;

                        Err(err)
                    }
                }
            }
        }
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn resolve_workspace(&self, name: Option<&str>) -> Result<Workspace> {
        let ws_name = name
            .map(String::from)
            .or_else(|| self.config.defaults.as_ref()?.workspace.clone())
            .unwrap_or_else(|| "default".into());
        self.db
            .get_workspace_by_name(&ws_name)?
            .ok_or_else(|| anyhow::anyhow!("workspace '{}' not found", ws_name))
    }

    fn resolve_collection(&self, ws_id: i64, name: Option<&str>) -> Result<Collection> {
        let coll_name = name
            .map(String::from)
            .or_else(|| self.config.defaults.as_ref()?.collection.clone());
        match coll_name {
            Some(n) => self
                .db
                .get_collection_by_name(ws_id, &n)?
                .ok_or_else(|| anyhow::anyhow!("collection '{}' not found", n)),
            None => {
                let colls = self.db.list_collections(ws_id)?;
                if colls.len() == 1 {
                    Ok(colls.into_iter().next().unwrap())
                } else {
                    anyhow::bail!(
                        "no collection specified and no default configured. Use -c <name> or set defaults.collection in config"
                    )
                }
            }
        }
    }

    // ── Collection handlers ─────────────────────────────────────

    fn run_coll(self, args: CollArgs) -> Result<()> {
        match args.cmd {
            CollCmds::List(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let collections = self.db.list_collections(ws.id)?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&collections)?);
                } else {
                    for c in &collections {
                        println!("{}\t{}", c.name, c.description);
                    }
                }
                Ok(())
            }
            CollCmds::Create(args) => {
                if args.name.is_empty() {
                    anyhow::bail!("collection name must not be empty");
                }
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let desc = args.description.as_deref().unwrap_or("");
                let coll = self.db.create_collection(ws.id, &args.name, desc)?;
                println!("Created collection: {}", coll.name);
                Ok(())
            }
            CollCmds::Show(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self
                    .db
                    .get_collection_by_name(ws.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "collection '{}' not found in workspace '{}'",
                            args.name,
                            ws.name
                        )
                    })?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&coll)?);
                } else {
                    println!("Name:        {}", coll.name);
                    println!("Description: {}", coll.description);
                    println!("Workspace:   {}", ws.name);
                    println!("Default Env: {}", coll.default_env.map_or("(none)".into(), |id: i64| id.to_string()));
                    println!("Created:     {}", coll.created_at);
                    println!("Updated:     {}", coll.updated_at);
                }
                Ok(())
            }
            CollCmds::Update(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self
                    .db
                    .get_collection_by_name(ws.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "collection '{}' not found in workspace '{}'",
                            args.name,
                            ws.name
                        )
                    })?;
                let new_name = args.new_name.as_deref().unwrap_or(&coll.name);
                let new_desc = args.new_description.as_deref().unwrap_or(&coll.description);
                self.db.update_collection(coll.id, new_name, new_desc)?;
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
                    self.db.set_collection_default_env(coll.id, Some(env.id))?;
                }
                println!("Updated collection: {}", new_name);
                Ok(())
            }
            CollCmds::Del(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let coll = self
                    .db
                    .get_collection_by_name(ws.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "collection '{}' not found in workspace '{}'",
                            args.name,
                            ws.name
                        )
                    })?;
                if !args.force {
                    let reqs = self.db.list_requests(coll.id)?.len();
                    let confirmed = inquire::Confirm::new(&format!(
                        "Permanently delete collection '{}' and all its contents ({} request(s))?",
                        coll.name, reqs
                    ))
                    .with_default(false)
                    .prompt()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                }
                self.db.delete_collection(coll.id)?;
                println!("Deleted collection: {}", coll.name);
                Ok(())
            }
        }
    }

    // ── Environment handlers ────────────────────────────────────

    fn run_env(self, args: EnvArgs) -> Result<()> {
        match args.cmd {
            EnvCmds::List(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let envs = self.db.list_environments(ws.id)?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&envs)?);
                } else {
                    for e in &envs {
                        println!("{}\t{}", e.name, e.description);
                    }
                }
                Ok(())
            }
            EnvCmds::Create(args) => {
                if args.name.is_empty() {
                    anyhow::bail!("environment name must not be empty");
                }
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let desc = args.description.as_deref().unwrap_or("");
                let env = self.db.create_environment(ws.id, &args.name, desc)?;
                println!("Created environment: {}", env.name);
                Ok(())
            }
            EnvCmds::Show(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.name,
                            ws.name
                        )
                    })?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&env)?);
                } else {
                    println!("Name:        {}", env.name);
                    println!("Description: {}", env.description);
                    println!("Workspace:   {}", ws.name);
                    println!("Created:     {}", env.created_at);
                    println!("Updated:     {}", env.updated_at);
                }
                Ok(())
            }
            EnvCmds::Update(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.name,
                            ws.name
                        )
                    })?;
                let new_name = args.new_name.as_deref().unwrap_or(&env.name);
                let new_desc = args.new_description.as_deref().unwrap_or(&env.description);
                self.db.update_environment(env.id, new_name, new_desc)?;
                println!("Updated environment: {}", new_name);
                Ok(())
            }
            EnvCmds::Del(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.name,
                            ws.name
                        )
                    })?;
                if !args.force {
                    let vars = self.db.list_environment_vars(env.id)?.len();
                    let confirmed = inquire::Confirm::new(&format!(
                        "Permanently delete environment '{}' and all its contents ({} variable(s))?",
                        env.name, vars
                    ))
                    .with_default(false)
                    .prompt()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                }
                self.db.delete_environment(env.id)?;
                println!("Deleted environment: {}", env.name);
                Ok(())
            }
            EnvCmds::Vars(vars) => self.run_env_vars(vars),
        }
    }

    fn run_env_vars(self, args: EnvVarArgs) -> Result<()> {
        match args.cmd {
            EnvVarCmds::List(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.env)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.env,
                            ws.name
                        )
                    })?;
                let vars = self.db.list_environment_vars(env.id)?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&vars)?);
                } else {
                    for v in &vars {
                        let display_val = if v.is_secret { "********" } else { &v.value };
                        println!("{}\t{}", v.name, display_val);
                    }
                }
                Ok(())
            }
            EnvVarCmds::Create(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.env)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.env,
                            ws.name
                        )
                    })?;
                if args.name.is_empty() {
                    anyhow::bail!("variable name must not be empty");
                }
                let desc = args.description.as_deref().unwrap_or("");
                let var = self.db.create_environment_var(
                    env.id,
                    &args.name,
                    &args.value,
                    args.secret,
                    desc,
                )?;
                println!("Created variable: {}", var.name);
                Ok(())
            }
            EnvVarCmds::Show(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.env)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.env,
                            ws.name
                        )
                    })?;
                let var = self
                    .db
                    .get_environment_var_by_name(env.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "variable '{}' not found in environment '{}'",
                            args.name,
                            env.name
                        )
                    })?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&var)?);
                } else {
                    let display_val = if var.is_secret { "********" } else { &var.value };
                    println!("Name:        {}", var.name);
                    println!("Value:       {}", display_val);
                    println!("Description: {}", var.description);
                    println!("Secret:      {}", var.is_secret);
                    println!("Created:     {}", var.created_at);
                    println!("Updated:     {}", var.updated_at);
                }
                Ok(())
            }
            EnvVarCmds::Update(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.env)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.env,
                            ws.name
                        )
                    })?;
                let var = self
                    .db
                    .get_environment_var_by_name(env.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "variable '{}' not found in environment '{}'",
                            args.name,
                            env.name
                        )
                    })?;
                let new_name = args.new_name.as_deref().unwrap_or(&var.name);
                let new_value = args.new_value.as_deref().unwrap_or(&var.value);
                let new_secret = args.secret.unwrap_or(var.is_secret);
                let new_desc = args.new_description.as_deref().unwrap_or(&var.description);
                self.db.update_environment_var(var.id, new_name, new_value, new_secret, new_desc)?;
                println!("Updated variable: {}", new_name);
                Ok(())
            }
            EnvVarCmds::Del(args) => {
                let ws = self.resolve_workspace(args.workspace.as_deref())?;
                let env = self
                    .db
                    .get_environment_by_name(ws.id, &args.env)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "environment '{}' not found in workspace '{}'",
                            args.env,
                            ws.name
                        )
                    })?;
                let var = self
                    .db
                    .get_environment_var_by_name(env.id, &args.name)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "variable '{}' not found in environment '{}'",
                            args.name,
                            env.name
                        )
                    })?;
                self.db.delete_environment_var(var.id)?;
                println!("Deleted variable: {}", var.name);
                Ok(())
            }
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
                        "Permanently delete workspace '{}' and all its contents ({} collection(s), {} environment(s))?",
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
                    defaults: Some(conf::DefaultsConfig {
                        workspace: Some("default".into()),
                        collection: Some("default".into()),
                        environment: None,
                    }),
                    history: Some(conf::HistoryConfig {
                        retention_days: None,
                    }),
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
    fn test_coll_update_default_env() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "staging", "").unwrap();
        app.db.create_collection(ws.id, "my-coll", "").unwrap();
        let cli = parse_cli(&["coll", "update", "my-coll", "-w", "ws", "--default-env", "staging"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let coll = db.get_collection_by_name(ws.id, "my-coll").unwrap().unwrap();
        assert!(coll.default_env.is_some());
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

    // ── Collection tests ──────────────────────────────────────

    #[test]
    fn test_coll_create_and_list() {
        let (_dir, app) = test_app();
        app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(
            app.db.get_workspace_by_name("ws").unwrap().unwrap().id,
            "my-coll",
            "a collection",
        ).unwrap();
        let cli = parse_cli(&["coll", "list", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_coll_create_via_cli() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        // Don't pass -w; should fall back to "default" workspace
        let cli = parse_cli(&["coll", "create", "new-coll", "-d", "desc"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("default").unwrap().unwrap();
        let coll = db.get_collection_by_name(ws.id, "new-coll").unwrap();
        assert!(coll.is_some());
        assert_eq!(coll.unwrap().description, "desc");
    }

    #[test]
    fn test_coll_create_empty_name_fails() {
        let (_dir, app) = test_app();
        app.db.create_workspace("ws", "").unwrap();
        let cli = parse_cli(&["coll", "create", "", "-w", "ws"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must not be empty"));
    }

    #[test]
    fn test_coll_show_found() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(ws.id, "my-coll", "desc").unwrap();
        let cli = parse_cli(&["coll", "show", "my-coll", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_coll_show_not_found() {
        let (_dir, app) = test_app();
        app.db.create_workspace("ws", "").unwrap();
        let cli = parse_cli(&["coll", "show", "nonexistent", "-w", "ws"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_coll_update_name_and_description() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(ws.id, "old-coll", "old desc").unwrap();
        let cli = parse_cli(&[
            "coll", "update", "old-coll", "-w", "ws",
            "--new-name", "new-coll",
            "--new-description", "new desc",
        ]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let coll = db.get_collection_by_name(ws.id, "new-coll").unwrap();
        assert!(coll.is_some());
        assert_eq!(coll.unwrap().description, "new desc");
        assert!(db.get_collection_by_name(ws.id, "old-coll").unwrap().is_none());
    }

    #[test]
    fn test_coll_delete_force() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(ws.id, "to-delete", "").unwrap();
        let cli = parse_cli(&["coll", "del", "to-delete", "-w", "ws", "--force"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        assert!(db.get_collection_by_name(ws.id, "to-delete").unwrap().is_none());
    }

    #[test]
    fn test_cascade_delete_workspace_removes_children() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let env = app.db.create_environment(ws.id, "dev", "").unwrap();
        app.db.create_environment_var(env.id, "KEY", "val", false, "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        app.db.create_collection_var(coll.id, "CV", "v", false).unwrap();
        let req = app.db.create_request(coll.id, "r1", "GET", "http://x", None).unwrap();
        app.db.create_request_header(req.id, "h", "v").unwrap();
        app.db.create_request_query_param(req.id, "q", "v").unwrap();

        // Delete workspace — everything should cascade
        app.db.delete_workspace(ws.id).unwrap();

        assert!(app.db.list_environments(ws.id).unwrap().is_empty());
        assert!(app.db.list_collections(ws.id).unwrap().is_empty());
        assert!(app.db.list_requests(coll.id).unwrap().is_empty());
    }

    #[test]
    fn test_cascade_delete_preserves_history_with_null_req() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        let req = app.db.create_request(coll.id, "r1", "GET", "http://x", None).unwrap();

        let hist = app.db.create_history(&crate::dtypes::CreateHistoryEntry {
            req_id: Some(req.id),
            method: "GET".into(),
            resolved_url: "http://x".into(),
            resolved_req_headers: "[]".into(),
            resolved_req_body: None,
            success: true,
            res_status: Some(200),
            res_body: None,
            res_headers: "[]".into(),
            res_duration: Some(0.1),
        }).unwrap();

        // Delete workspace — cascades to collection → request, history.req_id becomes NULL
        app.db.delete_workspace(ws.id).unwrap();

        let preserved = app.db.get_history_by_id(hist.id).unwrap();
        assert!(preserved.is_some(), "history row should still exist");
        assert_eq!(preserved.unwrap().req_id, None, "req_id should be NULL after cascade");
    }

    // ── Environment tests ──────────────────────────────────────

    #[test]
    fn test_env_create_and_list() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "staging", "Staging env").unwrap();
        let cli = parse_cli(&["env", "list", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_env_create_via_cli() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        // Don't pass -w; should fall back to "default" workspace
        let cli = parse_cli(&["env", "create", "staging", "-d", "Staging env"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("default").unwrap().unwrap();
        let env = db.get_environment_by_name(ws.id, "staging").unwrap();
        assert!(env.is_some());
        assert_eq!(env.unwrap().description, "Staging env");
    }

    #[test]
    fn test_env_create_empty_name_fails() {
        let (_dir, app) = test_app();
        let cli = parse_cli(&["env", "create", ""]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must not be empty"));
    }

    #[test]
    fn test_env_show_found() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "staging", "desc").unwrap();
        let cli = parse_cli(&["env", "show", "staging", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_env_show_not_found() {
        let (_dir, app) = test_app();
        app.db.create_workspace("ws", "").unwrap();
        let cli = parse_cli(&["env", "show", "nonexistent", "-w", "ws"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_env_update_name_and_description() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "old-env", "old desc").unwrap();
        let cli = parse_cli(&[
            "env", "update", "old-env", "-w", "ws",
            "--new-name", "new-env",
            "--new-description", "new desc",
        ]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let env = db.get_environment_by_name(ws.id, "new-env").unwrap();
        assert!(env.is_some());
        assert_eq!(env.unwrap().description, "new desc");
        assert!(db.get_environment_by_name(ws.id, "old-env").unwrap().is_none());
    }

    #[test]
    fn test_env_delete_force() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "to-delete", "").unwrap();
        let cli = parse_cli(&["env", "del", "to-delete", "-w", "ws", "--force"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        assert!(db.get_environment_by_name(ws.id, "to-delete").unwrap().is_none());
    }

    // ── Environment variable tests ──────────────────────────────

    #[test]
    fn test_env_var_create_and_list() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let env = app.db.create_environment(ws.id, "dev", "").unwrap();
        app.db.create_environment_var(env.id, "API_KEY", "secret", true, "An API key").unwrap();
        let cli = parse_cli(&["env", "vars", "list", "-e", "dev", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_env_var_create_via_cli() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "dev", "").unwrap();
        let cli = parse_cli(&[
            "env", "vars", "create", "MY_VAR", "my-value",
            "-e", "dev", "-w", "ws", "--secret", "-d", "A desc",
        ]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let env = db.get_environment_by_name(ws.id, "dev").unwrap().unwrap();
        let var = db.get_environment_var_by_name(env.id, "MY_VAR").unwrap();
        assert!(var.is_some());
        let var = var.unwrap();
        assert_eq!(var.value, "my-value");
        assert!(var.is_secret);
        assert_eq!(var.description, "A desc");
    }

    #[test]
    fn test_env_var_show_found() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let env = app.db.create_environment(ws.id, "dev", "").unwrap();
        app.db.create_environment_var(env.id, "KEY", "val", false, "").unwrap();
        let cli = parse_cli(&["env", "vars", "show", "KEY", "-e", "dev", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_env_var_show_not_found() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_environment(ws.id, "dev", "").unwrap();
        let cli = parse_cli(&["env", "vars", "show", "NOPE", "-e", "dev", "-w", "ws"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_env_var_update_value() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        let env = app.db.create_environment(ws.id, "dev", "").unwrap();
        app.db.create_environment_var(env.id, "KEY", "old-val", false, "old desc").unwrap();
        let cli = parse_cli(&[
            "env", "vars", "update", "KEY", "-e", "dev", "-w", "ws",
            "--new-value", "new-val", "--new-description", "new desc",
        ]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let env = db.get_environment_by_name(ws.id, "dev").unwrap().unwrap();
        let var = db.get_environment_var_by_name(env.id, "KEY").unwrap().unwrap();
        assert_eq!(var.value, "new-val");
        assert_eq!(var.description, "new desc");
    }

    #[test]
    fn test_env_var_delete() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        let env = app.db.create_environment(ws.id, "dev", "").unwrap();
        app.db.create_environment_var(env.id, "KEY", "val", false, "").unwrap();
        let cli = parse_cli(&["env", "vars", "del", "KEY", "-e", "dev", "-w", "ws"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let env = db.get_environment_by_name(ws.id, "dev").unwrap().unwrap();
        assert!(db.get_environment_var_by_name(env.id, "KEY").unwrap().is_none());
    }

    // ── Request tests ──────────────────────────────────────────

    #[test]
    fn test_req_create_and_list() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        app.db.create_request(coll.id, "my-req", "GET", "http://example.com", None).unwrap();
        let cli = parse_cli(&["req", "list", "-c", "coll", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_req_create_via_cli() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(ws.id, "coll", "").unwrap();
        let cli = parse_cli(&[
            "req", "create", "my-req", "http://example.com",
            "-c", "coll", "-w", "ws",
            "-X", "POST",
            "-d", r#"{"key":"val"}"#,
            "-H", "Content-Type: application/json",
            "-q", "page=1",
        ]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let coll = db.get_collection_by_name(ws.id, "coll").unwrap().unwrap();
        let req = db.get_request_by_name(coll.id, "my-req").unwrap();
        assert!(req.is_some());
        let req = req.unwrap();
        assert_eq!(req.method, crate::dtypes::Method::POST);
        assert_eq!(req.url, "http://example.com");
        assert_eq!(req.body.as_deref(), Some(r#"{"key":"val"}"#));

        let headers = db.list_request_headers(req.id).unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].hkey, "Content-Type");
        assert_eq!(headers[0].hval, "application/json");

        let params = db.list_request_query_params(req.id).unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].qkey, "page");
        assert_eq!(params[0].qval, "1");
    }

    #[test]
    fn test_req_create_empty_name_fails() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(ws.id, "coll", "").unwrap();
        let cli = parse_cli(&["req", "create", "", "http://x", "-c", "coll", "-w", "ws"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must not be empty"));
    }

    #[test]
    fn test_req_show_found() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        let req = app.db.create_request(coll.id, "my-req", "POST", "http://x", Some("body")).unwrap();
        app.db.create_request_header(req.id, "Accept", "text/html").unwrap();
        app.db.create_request_query_param(req.id, "q", "test").unwrap();
        let cli = parse_cli(&["req", "show", "my-req", "-c", "coll", "-w", "ws"]);
        app.run(cli).unwrap();
    }

    #[test]
    fn test_req_show_not_found() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        app.db.create_collection(ws.id, "coll", "").unwrap();
        let cli = parse_cli(&["req", "show", "nonexistent", "-c", "coll", "-w", "ws"]);
        let result = app.run(cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_req_update_name_and_url() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        app.db.create_request(coll.id, "old-req", "GET", "http://old.com", None).unwrap();
        let cli = parse_cli(&[
            "req", "update", "old-req", "-c", "coll", "-w", "ws",
            "--new-name", "new-req",
            "--new-url", "http://new.com",
        ]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let coll = db.get_collection_by_name(ws.id, "coll").unwrap().unwrap();
        let req = db.get_request_by_name(coll.id, "new-req").unwrap();
        assert!(req.is_some());
        let req = req.unwrap();
        assert_eq!(req.url, "http://new.com");
        assert!(db.get_request_by_name(coll.id, "old-req").unwrap().is_none());
    }

    #[test]
    fn test_req_delete_force() {
        let (dir, app) = test_app();
        let db_path = dir.path().join("test.db");
        let ws = app.db.create_workspace("ws", "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        app.db.create_request(coll.id, "to-delete", "GET", "http://x", None).unwrap();
        let cli = parse_cli(&["req", "del", "to-delete", "-c", "coll", "-w", "ws", "--force"]);
        app.run(cli).unwrap();

        let db = DBClient::new(Some(db_path.to_str().unwrap())).unwrap();
        let ws = db.get_workspace_by_name("ws").unwrap().unwrap();
        let coll = db.get_collection_by_name(ws.id, "coll").unwrap().unwrap();
        assert!(db.get_request_by_name(coll.id, "to-delete").unwrap().is_none());
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

    #[test]
    fn test_req_run_missing_request_errors() {
        let (_dir, app) = test_app();
        app.db.create_workspace("ws", "").unwrap();
        app.db
            .create_collection(
                app.db.get_workspace_by_name("ws").unwrap().unwrap().id,
                "coll",
                "",
            )
            .unwrap();
        let cli = parse_cli(&["req", "run", "nonexistent", "-c", "coll", "-w", "ws"]);
        let err = app.run(cli).unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "expected 'not found' error, got: {}",
            err
        );
    }

    #[test]
    fn test_req_run_undefined_var_errors() {
        let (_dir, app) = test_app();
        let ws = app.db.create_workspace("ws", "").unwrap();
        let coll = app.db.create_collection(ws.id, "coll", "").unwrap();
        app.db
            .create_request(coll.id, "var-req", "GET", "https://{{ missing }}/api", None)
            .unwrap();
        let cli = parse_cli(&["req", "run", "var-req", "-c", "coll", "-w", "ws"]);
        let err = app.run(cli).unwrap_err();
        assert!(
            err.to_string().contains("undefined variables"),
            "expected 'undefined variables' error, got: {}",
            err
        );
    }
}
