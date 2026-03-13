//! Code for the app's wrapping CLI

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: RootCmds,
}

#[derive(Debug, Subcommand)]
pub enum RootCmds {
    /// Commands for interacting with requests in a collection
    Req(ReqArgs),

    /// Commands for interacting with collections
    Coll(CollArgs),

    /// Commands for interacting with environments
    Env(EnvArgs),

    /// Commands for interacting with workspaces
    Work(WorkArgs),

    /// Commands for interacting with specs
    Spec,

    /// Commands for viewing request history
    Hist(HistArgs),

    /// Commands for configuring yapi
    Conf(ConfArgs),
}

// ── Req ─────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ReqArgs {
    #[command(subcommand)]
    pub cmd: ReqCmds,
}

#[derive(Debug, Subcommand)]
pub enum ReqCmds {
    /// List requests in a collection
    List(ReqListArgs),
    /// Create a new request
    Create(ReqCreateArgs),
    /// Show details of a request
    Show(ReqShowArgs),
    /// Run a request
    Run(ReqRunArgs),
    /// Update a request
    Update(ReqUpdateArgs),
    /// Delete a request from a collection
    Del(ReqDelArgs),
}

#[derive(Debug, Args)]
pub struct ReqListArgs {
    /// Collection to list requests from
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Filter by workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ReqCreateArgs {
    /// Name for the new request
    pub name: String,
    /// URL for the request
    pub url: String,
    /// Collection to add the request to
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// HTTP method
    #[arg(short = 'X', long, default_value = "GET")]
    pub method: Option<String>,
    /// Request body
    #[arg(short = 'd', long = "data")]
    pub body: Option<String>,
    /// Headers (repeatable), format: "Key: Value"
    #[arg(short = 'H', long = "header")]
    pub headers: Vec<String>,
    /// Query parameters (repeatable), format: "key=value"
    #[arg(short, long = "query")]
    pub queries: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ReqShowArgs {
    /// Name of the request
    pub name: String,
    /// Collection containing the request
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ReqRunArgs {
    /// Name of the request to run
    pub name: String,
    /// Collection containing the request
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Environment to use for variable substitution
    #[arg(short, long)]
    pub env: Option<String>,
}

#[derive(Debug, Args)]
pub struct ReqUpdateArgs {
    /// Name of the request to update
    pub name: String,
    /// Collection containing the request
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// New name for the request
    #[arg(long)]
    pub new_name: Option<String>,
    /// New URL
    #[arg(long)]
    pub new_url: Option<String>,
    /// New HTTP method
    #[arg(long)]
    pub new_method: Option<String>,
    /// New request body
    #[arg(long)]
    pub new_body: Option<String>,
}

#[derive(Debug, Args)]
pub struct ReqDelArgs {
    /// Name of the request to delete
    pub name: String,
    /// Collection containing the request
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

// ── Coll ────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct CollArgs {
    #[command(subcommand)]
    pub cmd: CollCmds,
}

#[derive(Debug, Subcommand)]
pub enum CollCmds {
    /// List collections
    List(CollListArgs),
    /// Create a new collection
    Create(CollCreateArgs),
    /// Show details of a collection
    Show(CollShowArgs),
    /// Update a collection
    Update(CollUpdateArgs),
    /// Delete a collection
    Del(CollDelArgs),
}

#[derive(Debug, Args)]
pub struct CollListArgs {
    /// Filter by workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CollCreateArgs {
    /// Name for the new collection
    pub name: String,
    /// Workspace to create the collection in
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Description of the collection
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct CollShowArgs {
    /// Name of the collection
    pub name: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CollUpdateArgs {
    /// Name of the collection to update
    pub name: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// New name for the collection
    #[arg(long)]
    pub new_name: Option<String>,
    /// New description
    #[arg(long)]
    pub new_description: Option<String>,
    /// Set the default environment (by name)
    #[arg(long)]
    pub default_env: Option<String>,
}

#[derive(Debug, Args)]
pub struct CollDelArgs {
    /// Name of the collection to delete
    pub name: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

// ── Env ─────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub cmd: EnvCmds,
}

#[derive(Debug, Subcommand)]
pub enum EnvCmds {
    /// List environments
    List(EnvListArgs),
    /// Create a new environment
    Create(EnvCreateArgs),
    /// Show details of an environment
    Show(EnvShowArgs),
    /// Update an environment
    Update(EnvUpdateArgs),
    /// Delete an environment
    Del(EnvDelArgs),
    /// Manage environment variables
    Vars(EnvVarArgs),
}

#[derive(Debug, Args)]
pub struct EnvListArgs {
    /// Filter by workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EnvCreateArgs {
    /// Name for the new environment
    pub name: String,
    /// Workspace to create the environment in
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Description of the environment
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct EnvShowArgs {
    /// Name of the environment
    pub name: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EnvUpdateArgs {
    /// Name of the environment to update
    pub name: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// New name for the environment
    #[arg(long)]
    pub new_name: Option<String>,
    /// New description
    #[arg(long)]
    pub new_description: Option<String>,
}

#[derive(Debug, Args)]
pub struct EnvDelArgs {
    /// Name of the environment to delete
    pub name: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

// ── Env Vars ────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct EnvVarArgs {
    #[command(subcommand)]
    pub cmd: EnvVarCmds,
}

#[derive(Debug, Subcommand)]
pub enum EnvVarCmds {
    /// List variables in an environment
    List(EnvVarListArgs),
    /// Create a new environment variable
    Create(EnvVarCreateArgs),
    /// Show details of an environment variable
    Show(EnvVarShowArgs),
    /// Update an environment variable
    Update(EnvVarUpdateArgs),
    /// Delete an environment variable
    Del(EnvVarDelArgs),
}

#[derive(Debug, Args)]
pub struct EnvVarListArgs {
    /// Environment name
    #[arg(short, long)]
    pub env: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EnvVarCreateArgs {
    /// Name for the variable
    pub name: String,
    /// Value for the variable
    pub value: String,
    /// Environment to add the variable to
    #[arg(short, long)]
    pub env: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Description of the variable
    #[arg(short, long)]
    pub description: Option<String>,
    /// Mark as a secret variable
    #[arg(short, long)]
    pub secret: bool,
}

#[derive(Debug, Args)]
pub struct EnvVarShowArgs {
    /// Name of the variable
    pub name: String,
    /// Environment name
    #[arg(short, long)]
    pub env: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EnvVarUpdateArgs {
    /// Name of the variable to update
    pub name: String,
    /// Environment name
    #[arg(short, long)]
    pub env: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// New name for the variable
    #[arg(long)]
    pub new_name: Option<String>,
    /// New value
    #[arg(long)]
    pub new_value: Option<String>,
    /// New description
    #[arg(long)]
    pub new_description: Option<String>,
    /// Mark as secret (or not)
    #[arg(short, long)]
    pub secret: Option<bool>,
}

#[derive(Debug, Args)]
pub struct EnvVarDelArgs {
    /// Name of the variable to delete
    pub name: String,
    /// Environment name
    #[arg(short, long)]
    pub env: String,
    /// Workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
}

// ── Work ────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct WorkArgs {
    #[command(subcommand)]
    pub cmd: WorkCmds,
}

#[derive(Debug, Subcommand)]
pub enum WorkCmds {
    /// List workspaces
    List(WorkListArgs),
    /// Create a new workspace
    Create(WorkCreateArgs),
    /// Show details of a workspace
    Show(WorkShowArgs),
    /// Update a workspace
    Update(WorkUpdateArgs),
    /// Delete a workspace
    Del(WorkDelArgs),
}

#[derive(Debug, Args)]
pub struct WorkListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct WorkCreateArgs {
    /// Name for the new workspace
    pub name: String,
    /// Description of the workspace
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct WorkShowArgs {
    /// Name of the workspace
    pub name: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct WorkUpdateArgs {
    /// Name of the workspace to update
    pub name: String,
    /// New name for the workspace
    #[arg(long)]
    pub new_name: Option<String>,
    /// New description
    #[arg(long)]
    pub new_description: Option<String>,
}

#[derive(Debug, Args)]
pub struct WorkDelArgs {
    /// Name of the workspace to delete
    pub name: String,
    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

// ── Hist ────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct HistArgs {
    #[command(subcommand)]
    pub cmd: HistCmds,
}

#[derive(Debug, Subcommand)]
pub enum HistCmds {
    /// List history entries
    List(HistListArgs),
    /// Show details of a history entry
    Show(HistShowArgs),
    /// Delete a history entry
    Del(HistDelArgs),
}

#[derive(Debug, Args)]
pub struct HistListArgs {
    /// Filter by request name
    #[arg(short, long)]
    pub request: Option<String>,
    /// Filter by collection name
    #[arg(short, long)]
    pub collection: Option<String>,
    /// Filter by workspace name
    #[arg(short, long)]
    pub workspace: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct HistShowArgs {
    /// ID of the history entry
    pub id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct HistDelArgs {
    /// ID of the history entry to delete
    pub id: String,
}

// ── Conf ────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ConfArgs {
    #[command(subcommand)]
    pub cmd: ConfCmds,
}

#[derive(Debug, Subcommand)]
pub enum ConfCmds {
    /// Initialize a default configuration file
    Init(ConfInitArgs),
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set(ConfSetArgs),
    /// Get a configuration value
    Get(ConfGetArgs),
}

#[derive(Debug, Args)]
pub struct ConfInitArgs {
    /// Overwrite existing config and database files
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ConfSetArgs {
    /// Configuration key
    pub key: String,
    /// Value to set
    pub value: String,
}

#[derive(Debug, Args)]
pub struct ConfGetArgs {
    /// Configuration key
    pub key: String,
}
