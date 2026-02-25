use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const ROOT_BASE_CHANGESET_ID: &str = "ROOT";

#[derive(Debug, Parser)]
#[command(name = "ht", version, about = "HyperTide CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Login(LoginArgs),
    Branch(BranchArgs),
    Add(AddArgs),
    Submit(SubmitArgs),
    Log(LogArgs),
    Rollback(RollbackArgs),
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
struct LoginArgs {
    #[arg(long)]
    server: String,
    #[arg(long)]
    token: String,
    #[arg(long)]
    repo: Option<String>,
    #[arg(long, default_value = "main")]
    branch: String,
}

#[derive(Debug, Args)]
struct BranchArgs {
    #[command(subcommand)]
    command: BranchCommand,
}

#[derive(Debug, Subcommand)]
enum BranchCommand {
    Create(BranchCreateArgs),
    List(BranchListArgs),
    Switch(BranchSwitchArgs),
}

#[derive(Debug, Args)]
struct BranchCreateArgs {
    #[arg(long)]
    repo: String,
    #[arg(long)]
    name: String,
    #[arg(long)]
    from: Option<String>,
}

#[derive(Debug, Args)]
struct BranchListArgs {
    #[arg(long)]
    repo: String,
}

#[derive(Debug, Args)]
struct BranchSwitchArgs {
    #[arg(long)]
    repo: String,
    #[arg(long)]
    name: String,
}

#[derive(Debug, Args)]
struct AddArgs {
    #[arg(long)]
    path: String,
    #[arg(long)]
    blob: String,
    #[arg(long)]
    branch: Option<String>,
}

#[derive(Debug, Args)]
struct SubmitArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long, default_value = "submit")]
    message: String,
}

#[derive(Debug, Args)]
struct LogArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Debug, Args)]
struct RollbackArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long = "to")]
    target_changeset_id: String,
    #[arg(long)]
    author: Option<String>,
    #[arg(long)]
    message: Option<String>,
}

#[derive(Debug, Args)]
struct SyncArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long = "to")]
    to_changeset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliProfile {
    server: String,
    token: String,
    current_repo: Option<String>,
    current_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StageFile {
    branch: String,
    base_changeset_id: Option<String>,
    assets: Vec<AssetDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetDelta {
    path: String,
    blob_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerifyResponse {
    valid: bool,
    owner_id: Option<String>,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BranchRecord {
    name: String,
    created_by: String,
    created_at: String,
    is_default: bool,
    head_changeset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BranchListResponse {
    repo_id: String,
    branches: Vec<BranchRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangesetRecord {
    changeset_id: String,
    repo_id: String,
    branch: String,
    parent_changeset_id: Option<String>,
    base_changeset_id: Option<String>,
    kind: String,
    rollback_of: Option<String>,
    author: String,
    message: String,
    created_at: String,
    assets: Vec<AssetDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryPage {
    items: Vec<ChangesetRecord>,
    next_cursor: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncResponse {
    repo_id: String,
    branch: String,
    changeset_id: Option<String>,
    assets: Vec<SyncAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncAsset {
    path: String,
    blob_hash: String,
}

#[derive(Debug, Serialize)]
struct CreateBranchRequest<'a> {
    repo_id: &'a str,
    branch: &'a str,
    from_changeset_id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct SubmitRequest<'a> {
    repo_id: &'a str,
    branch: &'a str,
    base_changeset_id: &'a str,
    kind: &'a str,
    rollback_of: Option<&'a str>,
    author: &'a str,
    message: &'a str,
    assets: &'a [AssetDelta],
}

#[derive(Debug, Serialize)]
struct RollbackRequest<'a> {
    repo_id: &'a str,
    branch: &'a str,
    target_changeset_id: &'a str,
    author: &'a str,
    message: Option<&'a str>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Login(args) => login(args).await,
        Command::Branch(args) => branch(args).await,
        Command::Add(args) => add(args).await,
        Command::Submit(args) => submit(args).await,
        Command::Log(args) => show_log(args).await,
        Command::Rollback(args) => rollback(args).await,
        Command::Sync(args) => sync(args).await,
    }
}

async fn login(args: LoginArgs) -> Result<()> {
    let profile = CliProfile {
        server: args.server,
        token: args.token,
        current_repo: args.repo,
        current_branch: args.branch,
    };
    save_profile(&profile)?;
    println!(
        "login saved: server={}, branch={}",
        profile.server, profile.current_branch
    );
    Ok(())
}

async fn branch(args: BranchArgs) -> Result<()> {
    match args.command {
        BranchCommand::Create(cmd) => branch_create(cmd).await,
        BranchCommand::List(cmd) => branch_list(cmd).await,
        BranchCommand::Switch(cmd) => branch_switch(cmd).await,
    }
}

async fn branch_create(args: BranchCreateArgs) -> Result<()> {
    let profile = load_profile()?;
    let client = http_client(&profile)?;
    let payload = CreateBranchRequest {
        repo_id: &args.repo,
        branch: &args.name,
        from_changeset_id: args.from.as_deref(),
    };
    let url = format!("{}/v1/branches", profile.server.trim_end_matches('/'));
    let response: ApiResponse<BranchRecord> =
        client.post(url).json(&payload).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "create branch failed".to_string())));
    }
    println!("branch created: {}", args.name);
    Ok(())
}

async fn branch_list(args: BranchListArgs) -> Result<()> {
    let profile = load_profile()?;
    let client = http_client(&profile)?;
    let url = format!(
        "{}/v1/branches/{}",
        profile.server.trim_end_matches('/'),
        args.repo
    );
    let response: ApiResponse<BranchListResponse> = client.get(url).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "list branches failed".to_string())));
    }
    let data = response.data.context("missing response data")?;
    for branch in data.branches {
        println!(
            "{}  head={}",
            branch.name,
            branch
                .head_changeset_id
                .unwrap_or_else(|| "None".to_string())
        );
    }
    Ok(())
}

async fn branch_switch(args: BranchSwitchArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = http_client(&profile)?;
    let url = format!(
        "{}/v1/branches/{}",
        profile.server.trim_end_matches('/'),
        args.repo
    );
    let response: ApiResponse<BranchListResponse> = client.get(url).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "list branches failed".to_string())));
    }
    let data = response.data.context("missing response data")?;
    if !data.branches.iter().any(|b| b.name == args.name) {
        return Err(anyhow!("branch not found: {}", args.name));
    }
    profile.current_repo = Some(args.repo);
    profile.current_branch = args.name.clone();
    save_profile(&profile)?;

    let mut stage =
        load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&profile.current_branch));
    stage.branch = profile.current_branch.clone();
    stage.base_changeset_id = None;
    stage.assets.clear();
    save_stage(&stage)?;
    println!("switched to branch {}", profile.current_branch);
    Ok(())
}

async fn add(args: AddArgs) -> Result<()> {
    let profile = load_profile()?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let mut stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
    if stage.branch != branch {
        stage = StageFile::default_for_branch(&branch);
    }

    if let Some(existing) = stage
        .assets
        .iter_mut()
        .find(|asset| asset.path == args.path)
    {
        existing.blob_hash = Some(args.blob.clone());
    } else {
        stage.assets.push(AssetDelta {
            path: args.path,
            blob_hash: Some(args.blob),
        });
    }

    save_stage(&stage)?;
    println!("staged {} asset(s) on {}", stage.assets.len(), stage.branch);
    Ok(())
}

async fn submit(args: SubmitArgs) -> Result<()> {
    let profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = http_client(&profile)?;

    let mut stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
    if stage.branch != branch {
        stage = StageFile::default_for_branch(&branch);
    }
    if stage.assets.is_empty() {
        return Err(anyhow!("nothing staged; use `ht add --path --blob` first"));
    }

    let base = resolve_base_changeset(
        &client,
        &profile,
        &repo,
        &branch,
        stage.base_changeset_id.as_deref(),
    )
    .await?;
    let author = fetch_owner_id(&client, &profile).await?;
    let payload = SubmitRequest {
        repo_id: &repo,
        branch: &branch,
        base_changeset_id: &base,
        kind: "normal",
        rollback_of: None,
        author: &author,
        message: &args.message,
        assets: &stage.assets,
    };

    let url = format!("{}/v1/changesets", profile.server.trim_end_matches('/'));
    let response: ApiResponse<ChangesetRecord> =
        client.post(url).json(&payload).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "submit failed".to_string())));
    }
    let changeset = response.data.context("missing response data")?;
    stage.base_changeset_id = Some(changeset.changeset_id.clone());
    stage.branch = branch.clone();
    stage.assets.clear();
    save_stage(&stage)?;
    println!(
        "submitted {} on {}@{}",
        changeset.changeset_id, repo, branch
    );
    Ok(())
}

async fn show_log(args: LogArgs) -> Result<()> {
    let profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = http_client(&profile)?;
    let url = format!(
        "{}/v1/history/{}?branch={}&limit={}",
        profile.server.trim_end_matches('/'),
        repo,
        branch,
        args.limit
    );
    let response: ApiResponse<HistoryPage> = client.get(url).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "log failed".to_string())));
    }
    let history = response.data.context("missing response data")?;
    for item in history.items {
        println!("{}  {}  {}", item.changeset_id, item.kind, item.message);
    }
    Ok(())
}

async fn rollback(args: RollbackArgs) -> Result<()> {
    let profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = http_client(&profile)?;
    let author = match args.author {
        Some(author) => author,
        None => fetch_owner_id(&client, &profile).await?,
    };
    let payload = RollbackRequest {
        repo_id: &repo,
        branch: &branch,
        target_changeset_id: &args.target_changeset_id,
        author: &author,
        message: args.message.as_deref(),
    };
    let url = format!("{}/v1/rollback", profile.server.trim_end_matches('/'));
    let response: ApiResponse<serde_json::Value> =
        client.post(url).json(&payload).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "rollback failed".to_string())));
    }
    println!(
        "rollback submitted on {}@{} to {}",
        repo, branch, args.target_changeset_id
    );
    Ok(())
}

async fn sync(args: SyncArgs) -> Result<()> {
    let profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = http_client(&profile)?;
    let mut url = format!(
        "{}/v1/sync/{}?branch={}",
        profile.server.trim_end_matches('/'),
        repo,
        branch
    );
    if let Some(to) = &args.to_changeset_id {
        url.push_str("&to_changeset_id=");
        url.push_str(to);
    }

    let response: ApiResponse<SyncResponse> = client.get(url).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "sync failed".to_string())));
    }
    let snapshot = response.data.context("missing response data")?;

    let mut stage = StageFile::default_for_branch(&branch);
    stage.base_changeset_id = snapshot.changeset_id;
    save_stage(&stage)?;
    println!(
        "synced {}@{} to {} ({} assets)",
        repo,
        branch,
        stage
            .base_changeset_id
            .clone()
            .unwrap_or_else(|| "ROOT".to_string()),
        snapshot.assets.len()
    );
    Ok(())
}

async fn resolve_base_changeset(
    client: &reqwest::Client,
    profile: &CliProfile,
    repo: &str,
    branch: &str,
    current: Option<&str>,
) -> Result<String> {
    if let Some(existing) = current {
        return Ok(existing.to_string());
    }

    let url = format!(
        "{}/v1/branches/{}",
        profile.server.trim_end_matches('/'),
        repo
    );
    let response: ApiResponse<BranchListResponse> = client.get(url).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "failed to resolve branch head".to_string())
        ));
    }
    let data = response.data.context("missing response data")?;
    let branch_item = data
        .branches
        .into_iter()
        .find(|item| item.name == branch)
        .ok_or_else(|| anyhow!("branch not found: {branch}"))?;
    Ok(branch_item
        .head_changeset_id
        .unwrap_or_else(|| ROOT_BASE_CHANGESET_ID.to_string()))
}

async fn fetch_owner_id(client: &reqwest::Client, profile: &CliProfile) -> Result<String> {
    let url = format!("{}/api/auth/verify", profile.server.trim_end_matches('/'));
    let response: ApiResponse<VerifyResponse> = client.get(url).send().await?.json().await?;
    if !response.success {
        return Err(anyhow!(response
            .error
            .unwrap_or_else(|| "verify failed".to_string())));
    }
    let verify = response.data.context("missing verify data")?;
    if !verify.valid {
        return Err(anyhow!("api key is invalid"));
    }
    verify.owner_id.context("missing owner_id from verify")
}

fn http_client(profile: &CliProfile) -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert("X-API-Key", HeaderValue::from_str(&profile.token)?);
    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .build()?)
}

fn resolve_repo(profile: &CliProfile, repo: Option<&str>) -> Result<String> {
    if let Some(repo) = repo {
        return Ok(repo.to_string());
    }
    profile
        .current_repo
        .clone()
        .ok_or_else(|| anyhow!("repo not set. pass --repo or run login with --repo"))
}

impl StageFile {
    fn default_for_branch(branch: &str) -> Self {
        Self {
            branch: branch.to_string(),
            base_changeset_id: None,
            assets: Vec::new(),
        }
    }
}

fn load_profile() -> Result<CliProfile> {
    let path = profile_path()?;
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let profile: CliProfile = serde_json::from_str(&content)?;
    Ok(profile)
}

fn save_profile(profile: &CliProfile) -> Result<()> {
    ensure_state_dir()?;
    let path = profile_path()?;
    fs::write(path, serde_json::to_vec_pretty(profile)?)?;
    Ok(())
}

fn load_stage() -> Result<StageFile> {
    let path = stage_path()?;
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let stage: StageFile = serde_json::from_str(&content)?;
    Ok(stage)
}

fn save_stage(stage: &StageFile) -> Result<()> {
    ensure_state_dir()?;
    let path = stage_path()?;
    fs::write(path, serde_json::to_vec_pretty(stage)?)?;
    Ok(())
}

fn ensure_state_dir() -> Result<()> {
    let dir = state_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

fn state_dir() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(cwd.join(".hypertide"))
}

fn profile_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("profile.json"))
}

fn stage_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("stage.json"))
}

#[allow(dead_code)]
fn _is_inside(path: &Path, maybe_parent: &Path) -> bool {
    path.starts_with(maybe_parent)
}
