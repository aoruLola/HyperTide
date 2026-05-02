use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ht", version, about = "HyperTide CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Login(LoginArgs),
    Branch(BranchArgs),
    Add(AddArgs),
    Remove(RemoveArgs),
    Submit(SubmitArgs),
    Log(LogArgs),
    Rollback(RollbackArgs),
    Sync(SyncArgs),
    Checkout(CheckoutArgs),
    Status(StatusArgs),
    Diff(DiffArgs),
    ChunkUpload(ChunkUploadArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(long)]
    pub server: String,
    #[arg(long)]
    pub token: String,
    #[arg(long, default_value_t = false)]
    pub api_key_direct: bool,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value = "main")]
    pub branch: String,
}
#[derive(Debug, Args)]
pub struct BranchArgs {
    #[command(subcommand)]
    pub command: BranchCommand,
}
#[derive(Debug, Subcommand)]
pub enum BranchCommand {
    Create(BranchCreateArgs),
    List(BranchListArgs),
    Switch(BranchSwitchArgs),
}
#[derive(Debug, Args)]
pub struct BranchCreateArgs {
    #[arg(long)]
    pub repo: String,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub from: Option<String>,
}
#[derive(Debug, Args)]
pub struct BranchListArgs {
    #[arg(long)]
    pub repo: String,
}
#[derive(Debug, Args)]
pub struct BranchSwitchArgs {
    #[arg(long)]
    pub repo: String,
    #[arg(long)]
    pub name: String,
}
#[derive(Debug, Args)]
pub struct AddArgs {
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub blob: Option<String>,
    #[arg(long)]
    pub file: Option<String>,
    #[arg(long)]
    pub asset_path: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
}
#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(long)]
    pub asset_path: String,
    #[arg(long)]
    pub branch: Option<String>,
}
#[derive(Debug, Args)]
pub struct SubmitArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long, default_value = "submit")]
    pub message: String,
}
#[derive(Debug, Args)]
pub struct LogArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}
#[derive(Debug, Args)]
pub struct RollbackArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long = "to")]
    pub target_changeset_id: String,
    #[arg(long)]
    pub author: Option<String>,
    #[arg(long)]
    pub message: Option<String>,
}
#[derive(Debug, Args)]
pub struct SyncArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long = "to")]
    pub to_changeset_id: Option<String>,
}
#[derive(Debug, Args)]
pub struct CheckoutArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long = "to")]
    pub to_changeset_id: Option<String>,
}
#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
}
#[derive(Debug, Args)]
pub struct DiffArgs {
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
}
#[derive(Debug, Args)]
pub struct ChunkUploadArgs {
    #[arg(long)]
    pub file: String,
    #[arg(long, default_value_t = 4 * 1024 * 1024)]
    pub chunk_size: usize,
    #[arg(long, default_value = "fixed-4m")]
    pub chunk_size_policy: String,
    #[arg(long, default_value_t = false)]
    pub manifest_only: bool,
}
