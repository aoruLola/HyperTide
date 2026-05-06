use anyhow::Result;
use clap::Args;

use crate::cmd::repo;

#[derive(Debug, Args)]
pub(crate) struct InitArgs {
    #[arg(long, help = "Repository id to create or select")]
    pub repo: String,
    #[arg(long, default_value = "main", help = "Branch to select")]
    pub branch: String,
    #[arg(long, help = "Clear staged changes while initializing defaults")]
    pub force: bool,
}

pub(crate) async fn execute(args: InitArgs) -> Result<()> {
    repo::init_repo(&args.repo, &args.branch, args.force).await
}
