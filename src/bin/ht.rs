use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use clap::{Args, Parser, Subcommand};
use reqwest::{RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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
    ChunkUpload(ChunkUploadArgs),
}

#[derive(Debug, Args)]
struct LoginArgs {
    #[arg(long)]
    server: String,
    #[arg(long)]
    token: String,
    #[arg(long, default_value_t = false)]
    api_key_direct: bool,
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

#[derive(Debug, Args)]
struct ChunkUploadArgs {
    #[arg(long)]
    file: String,
    #[arg(long, default_value_t = 4 * 1024 * 1024)]
    chunk_size: usize,
    #[arg(long, default_value = "fixed-4m")]
    chunk_size_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliProfile {
    server: String,
    #[serde(alias = "token")]
    api_key: String,
    #[serde(default)]
    api_key_direct: bool,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    access_token_expires_at: Option<i64>,
    current_repo: Option<String>,
    #[serde(default = "default_branch")]
    current_branch: String,
}

fn default_branch() -> String {
    "main".to_string()
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
    error: Option<ApiError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiError {
    code: String,
    message: String,
    #[serde(default)]
    details: Option<serde_json::Value>,
    request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenPair {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: i64,
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

#[derive(Debug, Clone, Serialize)]
struct MissingChunksRequest<'a> {
    chunk_hashes: &'a [String],
}

#[derive(Debug, Clone, Deserialize)]
struct MissingChunksResponse {
    missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ManifestChunk {
    i: usize,
    chunk_hash: String,
    size: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CreateManifestRequest<'a> {
    version: u32,
    chunk_size_policy: &'a str,
    chunks: &'a [ManifestChunk],
    file_meta: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateManifestResponse {
    manifest_hash: String,
    merkle_root: String,
    chunk_count: usize,
    created: bool,
}

#[derive(Debug, Serialize)]
struct ExchangeKeyRequest<'a> {
    api_key: &'a str,
}

#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    refresh_token: &'a str,
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
        Command::ChunkUpload(args) => chunk_upload(args).await,
    }
}

async fn login(args: LoginArgs) -> Result<()> {
    let mut profile = CliProfile {
        server: args.server,
        api_key: args.token,
        api_key_direct: args.api_key_direct,
        access_token: None,
        refresh_token: None,
        access_token_expires_at: None,
        current_repo: args.repo,
        current_branch: args.branch,
    };

    if !profile.api_key_direct {
        let client = reqwest::Client::new();
        exchange_api_key_for_tokens(&client, &mut profile).await?;
    }

    save_profile(&profile)?;
    println!(
        "login saved: server={}, branch={}, mode={}",
        profile.server,
        profile.current_branch,
        if profile.api_key_direct {
            "api-key-direct"
        } else {
            "jwt"
        }
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
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let payload = CreateBranchRequest {
        repo_id: &args.repo,
        branch: &args.name,
        from_changeset_id: args.from.as_deref(),
    };
    let url = format!("{}/v2/branches", profile.server.trim_end_matches('/'));
    let response: ApiResponse<BranchRecord> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "create branch response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "create branch failed"
        )));
    }
    println!("branch created: {}", args.name);
    Ok(())
}

async fn branch_list(args: BranchListArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/branches/{}",
        profile.server.trim_end_matches('/'),
        args.repo
    );
    let response: ApiResponse<BranchListResponse> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "list branches response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "list branches failed"
        )));
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
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/branches/{}",
        profile.server.trim_end_matches('/'),
        args.repo
    );
    let response: ApiResponse<BranchListResponse> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "switch branch response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "list branches failed"
        )));
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
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();

    let mut stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
    if stage.branch != branch {
        stage = StageFile::default_for_branch(&branch);
    }
    if stage.assets.is_empty() {
        return Err(anyhow!("nothing staged; use `ht add --path --blob` first"));
    }

    let base = resolve_base_changeset(
        &client,
        &mut profile,
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

    let url = format!("{}/v2/changesets", profile.server.trim_end_matches('/'));
    let response: ApiResponse<ChangesetRecord> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "submit response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "submit failed")));
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
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/history/{}?branch={}&limit={}",
        profile.server.trim_end_matches('/'),
        repo,
        branch,
        args.limit
    );
    let response: ApiResponse<HistoryPage> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "log response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "log failed")));
    }
    let history = response.data.context("missing response data")?;
    for item in history.items {
        println!("{}  {}  {}", item.changeset_id, item.kind, item.message);
    }
    Ok(())
}

async fn rollback(args: RollbackArgs) -> Result<()> {
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
    let payload = RollbackRequest {
        repo_id: &repo,
        branch: &branch,
        target_changeset_id: &args.target_changeset_id,
        author: &author,
        message: args.message.as_deref(),
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
    println!(
        "rollback submitted on {}@{} to {}",
        repo, branch, args.target_changeset_id
    );
    Ok(())
}

async fn sync(args: SyncArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/v2/sync/{}?branch={}",
        profile.server.trim_end_matches('/'),
        repo,
        branch
    );
    if let Some(to) = &args.to_changeset_id {
        url.push_str("&to_changeset_id=");
        url.push_str(to);
    }

    let response: ApiResponse<SyncResponse> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "sync response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "sync failed")));
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
    profile: &mut CliProfile,
    repo: &str,
    branch: &str,
    current: Option<&str>,
) -> Result<String> {
    if let Some(existing) = current {
        return Ok(existing.to_string());
    }

    let url = format!(
        "{}/v2/branches/{}",
        profile.server.trim_end_matches('/'),
        repo
    );
    let response: ApiResponse<BranchListResponse> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.get(&url), profile),
        "resolve base response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(
            "{}",
            api_error_message(&response, "failed to resolve branch head")
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
    let url = format!("{}/v2/auth/verify", profile.server.trim_end_matches('/'));
    let response: HttpResponse<VerifyResponse> = execute_api(
        client.get(url).header("X-API-Key", &profile.api_key),
        "verify response decode failed",
    )
    .await?;
    if !response.payload.success {
        return Err(anyhow!(api_error_message(
            &response.payload,
            "verify failed"
        )));
    }
    let verify = response.payload.data.context("missing verify data")?;
    if !verify.valid {
        return Err(anyhow!("api key is invalid"));
    }
    verify.owner_id.context("missing owner_id from verify")
}

fn with_auth(request: RequestBuilder, profile: &CliProfile) -> RequestBuilder {
    if profile.api_key_direct {
        return request.header("X-API-Key", &profile.api_key);
    }
    if let Some(access_token) = profile.access_token.as_deref() {
        return request.bearer_auth(access_token);
    }
    request.header("X-API-Key", &profile.api_key)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn token_expired(profile: &CliProfile) -> bool {
    let Some(expires_at) = profile.access_token_expires_at else {
        return true;
    };
    now_unix() >= expires_at - 30
}

fn apply_token_pair(profile: &mut CliProfile, pair: TokenPair) {
    profile.access_token = Some(pair.access_token);
    profile.refresh_token = Some(pair.refresh_token);
    profile.access_token_expires_at = Some(now_unix() + pair.expires_in.max(0));
}

async fn exchange_api_key_for_tokens(
    client: &reqwest::Client,
    profile: &mut CliProfile,
) -> Result<()> {
    let url = format!(
        "{}/v2/auth/exchange-key",
        profile.server.trim_end_matches('/')
    );
    let payload = ExchangeKeyRequest {
        api_key: &profile.api_key,
    };
    let response: HttpResponse<TokenPair> = execute_api(
        client.post(url).json(&payload),
        "exchange-key response decode failed",
    )
    .await?;
    if !response.payload.success {
        return Err(anyhow!(
            "{}",
            api_error_message(&response.payload, "exchange-key failed")
        ));
    }
    let token_pair = response
        .payload
        .data
        .context("missing token pair in exchange-key response")?;
    apply_token_pair(profile, token_pair);
    Ok(())
}

async fn chunk_upload(args: ChunkUploadArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let file_path = PathBuf::from(&args.file);
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asset.bin")
        .to_string();
    let bytes = fs::read(&file_path)
        .with_context(|| format!("failed to read file {}", file_path.display()))?;

    if bytes.is_empty() {
        return Err(anyhow!("file is empty"));
    }
    let chunk_size = args.chunk_size.max(64 * 1024);
    let mut chunks = Vec::new();
    let mut cursor = 0usize;
    let mut index = 0usize;
    while cursor < bytes.len() {
        let end = (cursor + chunk_size).min(bytes.len());
        let mut hasher = Hasher::new();
        hasher.update(&bytes[cursor..end]);
        chunks.push(ManifestChunk {
            i: index,
            chunk_hash: hasher.finalize().to_hex().to_string(),
            size: end - cursor,
        });
        cursor = end;
        index += 1;
    }

    let hash_list = chunks
        .iter()
        .map(|chunk| chunk.chunk_hash.clone())
        .collect::<Vec<_>>();
    let missing_url = format!("{}/v2/blobs/missing", profile.server.trim_end_matches('/'));
    let missing_resp: ApiResponse<MissingChunksResponse> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| {
            with_auth(
                client.post(&missing_url).json(&MissingChunksRequest {
                    chunk_hashes: &hash_list,
                }),
                profile,
            )
        },
        "missing-chunk response decode failed",
    )
    .await?;
    if !missing_resp.success {
        return Err(anyhow!(api_error_message(
            &missing_resp,
            "query missing chunks failed"
        )));
    }

    let missing = missing_resp
        .data
        .context("missing missing-chunk response data")?
        .missing
        .into_iter()
        .collect::<HashSet<_>>();

    cursor = 0usize;
    let mut uploaded = 0usize;
    for chunk in &chunks {
        let end = (cursor + chunk.size).min(bytes.len());
        if missing.contains(&chunk.chunk_hash) {
            let upload_url = format!(
                "{}/v2/blobs/chunks/{}",
                profile.server.trim_end_matches('/'),
                chunk.chunk_hash
            );
            let payload = bytes[cursor..end].to_vec();
            let upload_resp: ApiResponse<serde_json::Value> = send_authed_api(
                &client,
                &mut profile,
                move |client, profile| {
                    with_auth(client.put(&upload_url).body(payload.clone()), profile)
                },
                "chunk upload response decode failed",
            )
            .await?;
            if !upload_resp.success {
                return Err(anyhow!(api_error_message(
                    &upload_resp,
                    "chunk upload failed"
                )));
            }
            uploaded += 1;
        }
        cursor = end;
    }

    let manifest_url = format!("{}/v2/manifests", profile.server.trim_end_matches('/'));
    let manifest_req = CreateManifestRequest {
        version: 1,
        chunk_size_policy: &args.chunk_size_policy,
        chunks: &chunks,
        file_meta: serde_json::json!({
            "path": args.file,
            "name": file_name,
            "size": bytes.len(),
        }),
    };
    let manifest_resp: ApiResponse<CreateManifestResponse> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&manifest_url).json(&manifest_req), profile),
        "manifest response decode failed",
    )
    .await?;
    if !manifest_resp.success {
        return Err(anyhow!(api_error_message(
            &manifest_resp,
            "manifest create failed"
        )));
    }
    let manifest = manifest_resp
        .data
        .context("missing manifest response data")?;

    println!(
        "chunk-upload done: chunks={} uploaded={} manifest={} merkle={} created={}",
        manifest.chunk_count,
        uploaded,
        manifest.manifest_hash,
        manifest.merkle_root,
        manifest.created
    );
    Ok(())
}

async fn refresh_access_token(client: &reqwest::Client, profile: &mut CliProfile) -> Result<()> {
    let refresh_token = profile
        .refresh_token
        .clone()
        .ok_or_else(|| anyhow!("refresh token missing; run `ht login` again"))?;
    let url = format!("{}/v2/auth/refresh", profile.server.trim_end_matches('/'));
    let payload = RefreshRequest {
        refresh_token: &refresh_token,
    };
    let response: HttpResponse<TokenPair> = execute_api(
        client.post(url).json(&payload),
        "refresh response decode failed",
    )
    .await?;
    if !response.payload.success {
        return Err(anyhow!(
            "{}",
            api_error_message(&response.payload, "refresh failed")
        ));
    }
    let token_pair = response
        .payload
        .data
        .context("missing token pair in refresh response")?;
    apply_token_pair(profile, token_pair);
    save_profile(profile)?;
    Ok(())
}

async fn ensure_access_token(client: &reqwest::Client, profile: &mut CliProfile) -> Result<()> {
    if profile.api_key_direct {
        return Ok(());
    }
    if !token_expired(profile) {
        return Ok(());
    }
    refresh_access_token(client, profile)
        .await
        .map_err(|error| anyhow!("{error}. please run `ht login --server ... --token ...`"))
}

fn api_error_message<T>(response: &ApiResponse<T>, fallback: &str) -> String {
    response
        .error
        .as_ref()
        .map(|error| {
            format!(
                "[{}] {} (request_id={})",
                error.code, error.message, error.request_id
            )
        })
        .unwrap_or_else(|| fallback.to_string())
}

async fn send_authed_api<T: DeserializeOwned, F>(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    build_request: F,
    context: &str,
) -> Result<ApiResponse<T>>
where
    F: Fn(&reqwest::Client, &CliProfile) -> RequestBuilder,
{
    ensure_access_token(client, profile).await?;

    let mut response: HttpResponse<T> =
        execute_api(build_request(client, profile), context).await?;
    if !profile.api_key_direct && response.status == StatusCode::UNAUTHORIZED {
        refresh_access_token(client, profile)
            .await
            .map_err(|error| anyhow!("{error}. please run `ht login --server ... --token ...`"))?;
        response = execute_api(build_request(client, profile), context).await?;
    }
    Ok(response.payload)
}

struct HttpResponse<T> {
    status: StatusCode,
    payload: ApiResponse<T>,
}

async fn execute_api<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
    context: &str,
) -> Result<HttpResponse<T>> {
    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await?;

    match serde_json::from_str::<ApiResponse<T>>(&body) {
        Ok(payload) => Ok(HttpResponse { status, payload }),
        Err(_) => Err(anyhow!(
            "{}: HTTP {} with non-JSON response body: {}",
            context,
            status,
            summarize_body(&body)
        )),
    }
}

fn summarize_body(body: &str) -> String {
    let compact = body.trim().replace('\n', " ");
    if compact.is_empty() {
        return "<empty>".to_string();
    }
    if compact.chars().count() > 200 {
        let preview: String = compact.chars().take(200).collect();
        return format!("{}...", preview);
    }
    compact
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
