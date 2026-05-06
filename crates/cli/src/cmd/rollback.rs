use anyhow::{anyhow, Result};
use clap::Args;

use crate::utils::*;

#[derive(Debug, Args)]
pub(crate) struct RollbackArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    pub repo: Option<String>,
    #[arg(long, help = "Target branch; defaults to the login profile branch")]
    pub branch: Option<String>,
    #[arg(long = "to", help = "Changeset id to roll back to")]
    pub target_changeset_id: String,
    #[arg(
        long,
        help = "Override rollback author; defaults to the authenticated owner"
    )]
    pub author: Option<String>,
    #[arg(long, help = "Rollback message")]
    pub message: Option<String>,
    #[arg(long, help = "Skip confirmation prompt")]
    pub yes: bool,
}

pub(crate) async fn execute(args: RollbackArgs) -> Result<()> {
    confirm_dangerous(
        &format!("rollback to changeset {}", args.target_changeset_id),
        args.yes,
    )?;
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();
    let author = match args.author {
        Some(author) => author,
        None => fetch_owner_id(&client, &profile).await?,
    };
    let message = args
        .message
        .unwrap_or_else(|| format!("rollback: {}", args.target_changeset_id));
    let payload = RollbackRequest {
        repo_id: &repo,
        branch: &branch,
        target_changeset_id: &args.target_changeset_id,
        author: &author,
        message: Some(&message),
    };
    let url = format!("{}/v2/rollback", profile.server.trim_end_matches('/'));
    let response: ApiResponse<serde_json::Value> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "rollback response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "rollback failed")));
    }
    print_json_response(response, "rollback failed")
}
