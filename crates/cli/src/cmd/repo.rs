use anyhow::{anyhow, Context, Result};
use clap::{Args, Subcommand};

use crate::utils::*;

#[derive(Debug, Args)]
pub(crate) struct RepoArgs {
    #[command(subcommand)]
    pub command: RepoCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum RepoCommand {
    #[command(about = "Create a repository")]
    Create(RepoCreateArgs),
    #[command(about = "List repositories")]
    List,
    #[command(about = "Show repository details")]
    Info(RepoInfoArgs),
    #[command(about = "Use a repository as the local default")]
    Use(RepoUseArgs),
}

#[derive(Debug, Args)]
pub(crate) struct RepoCreateArgs {
    #[arg(help = "Repository id to create")]
    pub repo: String,
    #[arg(long, default_value = "main", help = "Default branch for the new repo")]
    pub default_branch: String,
    #[arg(long = "use", help = "Set the new repo as the local default")]
    pub use_repo: bool,
    #[arg(long, help = "Allow switching defaults by clearing staged changes")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub(crate) struct RepoInfoArgs {
    #[arg(help = "Repository id; defaults to the login profile repository")]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct RepoUseArgs {
    #[arg(help = "Repository id to make current")]
    pub repo: String,
    #[arg(long, default_value = "main", help = "Branch to make current")]
    pub branch: String,
    #[arg(long, help = "Clear staged changes while switching defaults")]
    pub force: bool,
}

pub(crate) async fn execute(args: RepoArgs) -> Result<()> {
    match args.command {
        RepoCommand::Create(cmd) => repo_create(cmd).await,
        RepoCommand::List => repo_list().await,
        RepoCommand::Info(cmd) => repo_info(cmd).await,
        RepoCommand::Use(cmd) => repo_use(cmd).await,
    }
}

async fn repo_create(args: RepoCreateArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let repo = create_repo(&client, &mut profile, &args.repo, &args.default_branch).await?;

    if args.use_repo {
        switch_repo(&mut profile, &repo, &args.default_branch, args.force)?;
    }

    if json_output_enabled() {
        println!("{}", serde_json::to_string_pretty(&repo)?);
    } else if args.use_repo {
        println!(
            "repo created and selected: {}/{}",
            args.repo, args.default_branch
        );
    } else {
        println!("repo created: {}", args.repo);
    }
    Ok(())
}

async fn repo_list() -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let url = format!("{}/v2/repos", profile.server.trim_end_matches('/'));
    let response: ApiResponse<RepoListResponse> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "list repos response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "list repos failed")));
    }
    let data = response.data.context("missing response data")?;

    if json_output_enabled() {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else if data.repos.is_empty() {
        println!("no repos found");
    } else {
        for repo in data.repos {
            println!(
                "{}  default={}  branches={}  head={}",
                repo.repo_id,
                repo.default_branch,
                repo.branch_count,
                repo.default_head_changeset_id
                    .unwrap_or_else(|| "None".to_string())
            );
        }
    }
    Ok(())
}

async fn repo_info(args: RepoInfoArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let info = fetch_repo_info(&client, &mut profile, &repo).await?;

    if json_output_enabled() {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!(
            "{}  default={}  branches={}  head={}",
            info.repo_id,
            info.default_branch,
            info.branch_count,
            info.default_head_changeset_id.as_deref().unwrap_or("None")
        );
        for branch in info.branches {
            println!(
                "  {}{}  head={}",
                branch.name,
                if branch.is_default { " (default)" } else { "" },
                branch
                    .head_changeset_id
                    .unwrap_or_else(|| "None".to_string())
            );
        }
    }
    Ok(())
}

async fn repo_use(args: RepoUseArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let info = fetch_repo_info(&client, &mut profile, &args.repo).await?;
    switch_repo(&mut profile, &info, &args.branch, args.force)?;

    if json_output_enabled() {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("using repo {}/{}", args.repo, args.branch);
    }
    Ok(())
}

pub(crate) async fn init_repo(repo: &str, branch: &str, force: bool) -> Result<()> {
    let mut profile = load_profile()
        .context("not logged in; run `ht login --server <url> --token <key>` first")?;
    let client = reqwest::Client::new();
    let info = create_or_fetch_repo(&client, &mut profile, repo, branch).await?;
    switch_repo(&mut profile, &info, branch, force)?;

    if json_output_enabled() {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("initialized workspace for {}/{}", repo, branch);
    }
    Ok(())
}

async fn create_or_fetch_repo(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    repo: &str,
    branch: &str,
) -> Result<RepoInfo> {
    match create_repo(client, profile, repo, branch).await {
        Ok(info) => Ok(info),
        Err(error) if error.to_string().contains("Repo already exists") => {
            let info = fetch_repo_info(client, profile, repo).await?;
            if !info.branches.iter().any(|item| item.name == branch) {
                return Err(anyhow!("branch not found: {}", branch));
            }
            Ok(info)
        }
        Err(error) => Err(error),
    }
}

async fn create_repo(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    repo: &str,
    default_branch: &str,
) -> Result<RepoInfo> {
    let payload = CreateRepoRequest {
        repo_id: repo,
        default_branch,
    };
    let url = format!("{}/v2/repos", profile.server.trim_end_matches('/'));
    let response: ApiResponse<RepoInfo> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "create repo response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "create repo failed")));
    }
    response.data.context("missing response data")
}

async fn fetch_repo_info(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    repo: &str,
) -> Result<RepoInfo> {
    let url = format!("{}/v2/repos/{}", profile.server.trim_end_matches('/'), repo);
    let response: ApiResponse<RepoInfo> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.get(&url), profile),
        "repo info response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "repo info failed")));
    }
    response.data.context("missing response data")
}

fn switch_repo(profile: &mut CliProfile, info: &RepoInfo, branch: &str, force: bool) -> Result<()> {
    if !info.branches.iter().any(|item| item.name == branch) {
        return Err(anyhow!("branch not found: {}", branch));
    }

    if !force {
        if let Ok(stage) = load_stage() {
            if !stage.assets.is_empty() {
                return Err(anyhow!(
                    "current workspace has {} staged modification(s). run `ht submit` first or use `--force` to clear the staging area",
                    stage.assets.len()
                ));
            }
        }
    }

    profile.current_repo = Some(info.repo_id.clone());
    profile.current_branch = branch.to_string();
    save_profile(profile)?;

    let mut stage = StageFile::default_for_branch(branch);
    stage.base_changeset_id = None;
    save_stage(&stage)?;
    Ok(())
}
