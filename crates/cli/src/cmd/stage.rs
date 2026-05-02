use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::utils::*;

#[derive(Debug, Args)]
pub(crate) struct StageArgs {
    #[command(subcommand)]
    pub command: StageCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum StageCommand {
    #[command(about = "List staged assets")]
    List,
    #[command(about = "Clear all staged assets")]
    Clear(StageClearArgs),
}

#[derive(Debug, Args)]
pub(crate) struct StageClearArgs {
    #[arg(long, help = "Skip confirmation prompt")]
    pub yes: bool,
}

pub(crate) async fn execute(args: StageArgs) -> Result<()> {
    match args.command {
        StageCommand::List => stage_list(),
        StageCommand::Clear(cmd) => stage_clear(cmd),
    }
}

fn stage_list() -> Result<()> {
    let stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch("main"));
    if json_output_enabled() {
        #[derive(Serialize)]
        struct JsonStage {
            branch: String,
            base_changeset_id: Option<String>,
            assets: Vec<JsonStageAsset>,
        }
        #[derive(Serialize)]
        struct JsonStageAsset {
            path: String,
            blob_hash: Option<String>,
        }
        let json = JsonStage {
            branch: stage.branch.clone(),
            base_changeset_id: stage.base_changeset_id.clone(),
            assets: stage
                .assets
                .iter()
                .map(|a| JsonStageAsset {
                    path: a.path.clone(),
                    blob_hash: a.blob_hash.clone(),
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        if stage.assets.is_empty() {
            println!("staging area is empty");
            return Ok(());
        }
        println!(
            "staged assets on {} (base={}):",
            stage.branch,
            stage
                .base_changeset_id
                .as_deref()
                .unwrap_or("<none>")
        );
        for asset in &stage.assets {
            let hash = asset.blob_hash.as_deref().unwrap_or("<deleted>");
            println!("  {}  {}", hash, asset.path);
        }
        println!("({} asset(s) staged)", stage.assets.len());
    }
    Ok(())
}

fn stage_clear(args: StageClearArgs) -> Result<()> {
    let stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch("main"));
    if stage.assets.is_empty() {
        println!("staging area is already empty");
        return Ok(());
    }
    confirm_dangerous(
        &format!("clear {} staged asset(s)", stage.assets.len()),
        args.yes,
    )?;
    let mut updated = stage;
    updated.assets.clear();
    save_stage(&updated)?;
    println!("staging area cleared");
    Ok(())
}
