//! Code for the app's wrapping CLI

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about)]
pub struct Cli {
    #[command(subcommand)]
    cmd: RootCmds,
}

#[derive(Debug, Subcommand)]
pub enum RootCmds {
    /// Commands for interacting with requests in a collection
    Req(ReqArgs),

    /// Commands for interacting with collections
    Coll,

    /// Commands for interacting with environments
    Env,

    /// Commands for interacting with workspaces
    Work,

    /// Commands for interacting with specs
    Spec,

    /// Commands for viewing request history
    Hist,

    /// Commands for configuring yapi
    Conf,
}

#[derive(Debug, Args)]
pub struct ReqArgs {
    #[command(subcommand)]
    cmd: ReqCmds,
}

#[derive(Debug, Subcommand)]
pub enum ReqCmds {
    /// List requests in a collection
    List(ReqListArgs),

    /// Create a new request
    Create,

    /// Run a request
    Run,

    /// Run a request and test the response
    Test,

    /// Update part of a request
    Update,

    /// Delete a request from a collection
    Del,
}

#[derive(Debug, Args)]
pub struct ReqListArgs {}
