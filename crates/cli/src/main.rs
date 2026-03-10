use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use reqwest::{multipart, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const ROOT_BASE_CHANGESET_ID: &str = "ROOT";
const DIRECT_UPLOAD_THRESHOLD_BYTES: usize = 8 * 1024 * 1024;

mod client;
mod commands;
mod models;
mod workspace;

use clap::{CommandFactory, Parser};
use commands::*;
use models::*;

fn parse_cli_from<I, T>(iter: I) -> std::result::Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(iter)
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
        Command::Remove(args) => remove(args).await,
        Command::Submit(args) => submit(args).await,
        Command::Log(args) => show_log(args).await,
        Command::Rollback(args) => rollback(args).await,
        Command::Sync(args) => sync(args).await,
        Command::Checkout(args) => checkout(args).await,
        Command::Status(args) => show_status(args).await,
        Command::Diff(args) => show_diff(args).await,
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

    match (args.file, args.path, args.blob) {
        (Some(file), None, None) => {
            add_file(Path::new(&file), args.asset_path.as_deref(), &branch).await
        }
        (None, Some(path), Some(blob)) => {
            let mut stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
            if stage.branch != branch {
                stage = StageFile::default_for_branch(&branch);
            }
            upsert_stage_asset(&mut stage, &path, Some(blob));
            save_stage(&stage)?;
            println!("staged {} asset(s) on {}", stage.assets.len(), stage.branch);
            Ok(())
        }
        _ => Err(anyhow!(
            "use either `ht add --path <repo-path> --blob <hash>` or `ht add --file <local-file> [--asset-path <repo-path>]`"
        )),
    }
}

async fn remove(args: RemoveArgs) -> Result<()> {
    let profile = load_profile()?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let mut stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
    if stage.branch != branch {
        stage = StageFile::default_for_branch(&branch);
    }
    upsert_stage_asset(&mut stage, &args.asset_path, None);
    save_stage(&stage)?;
    println!(
        "staged delete for {} on {} ({} asset(s) staged)",
        args.asset_path,
        stage.branch,
        stage.assets.len()
    );
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
        return Err(anyhow!(submit_error_message(&response)));
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
    let snapshot = fetch_snapshot(
        &client,
        &mut profile,
        &repo,
        &branch,
        args.to_changeset_id.as_deref(),
    )
    .await?;

    let mut stage = StageFile::default_for_branch(&branch);
    stage.base_changeset_id = snapshot.changeset_id;
    save_stage(&stage)?;
    if let Ok(mut workspace) = load_workspace() {
        if workspace.repo_id == repo && workspace.branch == branch {
            workspace.base_changeset_id = stage.base_changeset_id.clone();
            workspace.last_synced_at = now_unix();
            save_workspace(&workspace)?;
        }
    }
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

async fn checkout(args: CheckoutArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();
    let snapshot = fetch_snapshot(
        &client,
        &mut profile,
        &repo,
        &branch,
        args.to_changeset_id.as_deref(),
    )
    .await?;
    let workspace_root = std::env::current_dir()?;
    let mut checked_out_assets = Vec::with_capacity(snapshot.assets.len());

    for asset in &snapshot.assets {
        let target = workspace_root.join(
            asset
                .path
                .replace('/', &std::path::MAIN_SEPARATOR.to_string()),
        );
        let bytes = fetch_blob_bytes(&client, &mut profile, &asset.blob_hash).await?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, &bytes)
            .with_context(|| format!("failed to write {}", target.display()))?;
        checked_out_assets.push(WorkspaceFile {
            path: asset.path.clone(),
            blob_hash: asset.blob_hash.clone(),
        });
    }

    let workspace = WorkspaceState {
        repo_id: repo.clone(),
        branch: branch.clone(),
        workspace_root: workspace_root.to_string_lossy().to_string(),
        base_changeset_id: snapshot.changeset_id.clone(),
        checked_out_assets,
        last_synced_at: now_unix(),
    };
    save_workspace(&workspace)?;

    let mut stage = StageFile::default_for_branch(&branch);
    stage.base_changeset_id = snapshot.changeset_id;
    save_stage(&stage)?;

    println!(
        "checked out {}@{} to {} ({} assets)",
        repo,
        branch,
        workspace.workspace_root,
        workspace.checked_out_assets.len()
    );
    Ok(())
}

async fn show_status(args: StatusArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let workspace = load_workspace()?;
    let repo =
        resolve_repo(&profile, args.repo.as_deref()).unwrap_or_else(|_| workspace.repo_id.clone());
    let branch = args.branch.unwrap_or_else(|| workspace.branch.clone());
    let client = reqwest::Client::new();
    let stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
    let locks = fetch_locks(&client, &mut profile).await.unwrap_or_default();
    let head = resolve_base_changeset(&client, &mut profile, &repo, &branch, None).await?;
    let stale_base = workspace
        .base_changeset_id
        .as_deref()
        .unwrap_or(ROOT_BASE_CHANGESET_ID)
        != head;

    let rows = collect_asset_rows(&workspace, &stage)?;
    for row in rows {
        let lock_conflict = locks
            .iter()
            .find(|lock| lock.file_path == row.path)
            .map(|lock| lock.owner_id.clone());
        let status = classify_asset_status(
            row.base_hash.as_deref(),
            row.local_hash.as_deref(),
            row.staged_hash.as_deref(),
            lock_conflict.as_deref(),
            stale_base,
        );
        println!("{}\t{}", status.as_str(), row.path);
    }
    Ok(())
}

async fn show_diff(args: DiffArgs) -> Result<()> {
    let profile = load_profile()?;
    let workspace = load_workspace()?;
    let branch = args.branch.unwrap_or_else(|| workspace.branch.clone());
    let _repo =
        resolve_repo(&profile, args.repo.as_deref()).unwrap_or_else(|_| workspace.repo_id.clone());
    let stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(&branch));
    let rows = collect_asset_rows(&workspace, &stage)?;

    for row in rows {
        println!(
            "{}\n  base={}\n  local={}\n  staged={}",
            row.path,
            row.base_hash.as_deref().unwrap_or("<none>"),
            row.local_hash.as_deref().unwrap_or("<none>"),
            row.staged_hash.as_deref().unwrap_or("<none>")
        );
    }
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

#[derive(Debug)]
struct AssetRow {
    path: String,
    base_hash: Option<String>,
    local_hash: Option<String>,
    staged_hash: Option<String>,
}

fn collect_asset_rows(workspace: &WorkspaceState, stage: &StageFile) -> Result<Vec<AssetRow>> {
    let workspace_root = PathBuf::from(&workspace.workspace_root);
    let mut paths = workspace
        .checked_out_assets
        .iter()
        .map(|asset| asset.path.clone())
        .collect::<HashSet<_>>();
    for asset in &stage.assets {
        paths.insert(asset.path.clone());
    }

    let mut rows = paths
        .into_iter()
        .map(|path| {
            let base_hash = workspace
                .checked_out_assets
                .iter()
                .find(|asset| asset.path == path)
                .map(|asset| asset.blob_hash.clone());
            let staged_hash = stage
                .assets
                .iter()
                .find(|asset| asset.path == path)
                .and_then(|asset| asset.blob_hash.clone());
            let local_hash = hash_local_asset(&workspace_root, &path)?;
            Ok(AssetRow {
                path,
                base_hash,
                local_hash,
                staged_hash,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(rows)
}

fn classify_asset_status(
    base_hash: Option<&str>,
    local_hash: Option<&str>,
    staged_hash: Option<&str>,
    lock_owner: Option<&str>,
    stale_base: bool,
) -> AssetStatusKind {
    if staged_hash.is_some()
        || (base_hash.is_some() && local_hash.is_none() && staged_hash.is_none())
    {
        if staged_hash.is_some() {
            return AssetStatusKind::Staged;
        }
    }
    if lock_owner.is_some() {
        return AssetStatusKind::LockedByOther;
    }
    if stale_base {
        return AssetStatusKind::StaleBase;
    }
    match (base_hash, local_hash, staged_hash) {
        (_, _, Some(_)) => AssetStatusKind::Staged,
        (Some(_), None, None) => AssetStatusKind::Deleted,
        (Some(base), Some(local), None) if base != local => AssetStatusKind::Modified,
        (None, Some(_), None) => AssetStatusKind::Added,
        _ => AssetStatusKind::Unmodified,
    }
}

fn hash_local_asset(workspace_root: &Path, asset_path: &str) -> Result<Option<String>> {
    let target =
        workspace_root.join(asset_path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
    if !target.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&target)
        .with_context(|| format!("failed to read workspace file {}", target.display()))?;
    Ok(Some(StorageHash::hash_bytes(&bytes)))
}

struct StorageHash;

impl StorageHash {
    fn hash_bytes(bytes: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(bytes);
        hasher.finalize().to_hex().to_string()
    }
}

fn upsert_stage_asset(stage: &mut StageFile, path: &str, blob_hash: Option<String>) {
    if let Some(existing) = stage.assets.iter_mut().find(|asset| asset.path == path) {
        existing.blob_hash = blob_hash;
    } else {
        stage.assets.push(AssetDelta {
            path: path.to_string(),
            blob_hash,
        });
    }
}

async fn add_file(file_path: &Path, asset_path: Option<&str>, branch: &str) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let repo_path = asset_path
        .map(|path| path.to_string())
        .unwrap_or_else(|| normalize_asset_path(file_path));
    let bytes = fs::read(file_path)
        .with_context(|| format!("failed to read file {}", file_path.display()))?;
    if bytes.is_empty() {
        return Err(anyhow!("file is empty"));
    }

    let blob_hash = upload_blob_from_bytes(
        &client,
        &mut profile,
        file_path,
        &bytes,
        4 * 1024 * 1024,
        "fixed-4m",
        false,
    )
    .await?
    .blob_hash;

    cache_blob(&blob_hash, &bytes)?;

    let mut stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(branch));
    if stage.branch != branch {
        stage = StageFile::default_for_branch(branch);
    }
    upsert_stage_asset(&mut stage, &repo_path, Some(blob_hash.clone()));
    save_stage(&stage)?;
    println!(
        "staged file {} as {} on {} (blob={})",
        file_path.display(),
        repo_path,
        branch,
        blob_hash
    );
    Ok(())
}

fn normalize_asset_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

async fn fetch_snapshot(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    repo: &str,
    branch: &str,
    to_changeset_id: Option<&str>,
) -> Result<SyncResponse> {
    let mut url = format!(
        "{}/v2/sync/{}?branch={}",
        profile.server.trim_end_matches('/'),
        repo,
        branch
    );
    if let Some(to) = to_changeset_id {
        url.push_str("&to_changeset_id=");
        url.push_str(to);
    }

    let response: ApiResponse<SyncResponse> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.get(&url), profile),
        "sync response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "sync failed")));
    }
    response.data.context("missing response data")
}

async fn fetch_blob_bytes(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    blob_hash: &str,
) -> Result<Vec<u8>> {
    let cache_path = cache_object_path(blob_hash)?;
    if cache_path.exists() {
        return fs::read(&cache_path)
            .with_context(|| format!("failed to read cached object {}", cache_path.display()));
    }

    ensure_access_token(client, profile).await?;
    let url = format!(
        "{}/v2/storage/download/{}",
        profile.server.trim_end_matches('/'),
        blob_hash
    );
    let response = with_auth(client.get(url), profile).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "download failed for {}: HTTP {}",
            blob_hash,
            response.status()
        ));
    }
    let bytes = response.bytes().await?.to_vec();
    cache_blob(blob_hash, &bytes)?;
    Ok(bytes)
}

async fn fetch_locks(
    client: &reqwest::Client,
    profile: &mut CliProfile,
) -> Result<Vec<FileLockInfo>> {
    let url = format!("{}/v2/locks", profile.server.trim_end_matches('/'));
    let response: ApiResponse<Vec<FileLockInfo>> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.get(&url), profile),
        "locks response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "list locks failed")));
    }
    Ok(response.data.unwrap_or_default())
}

async fn upload_blob_from_bytes(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    file_path: &Path,
    bytes: &[u8],
    chunk_size: usize,
    chunk_size_policy: &str,
    manifest_only: bool,
) -> Result<ComposeBlobResponse> {
    if bytes.len() <= DIRECT_UPLOAD_THRESHOLD_BYTES {
        let blob = upload_blob_direct(client, profile, file_path, bytes).await?;
        return Ok(blob);
    }
    upload_blob_via_chunks(
        client,
        profile,
        file_path,
        bytes,
        chunk_size,
        chunk_size_policy,
        manifest_only,
    )
    .await
}

async fn upload_blob_direct(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    file_path: &Path,
    bytes: &[u8],
) -> Result<ComposeBlobResponse> {
    ensure_access_token(client, profile).await?;
    let part = multipart::Part::bytes(bytes.to_vec())
        .file_name(
            file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("asset.bin")
                .to_string(),
        )
        .mime_str("application/octet-stream")?;
    let form = multipart::Form::new().part("file", part);
    let url = format!("{}/v2/storage/upload", profile.server.trim_end_matches('/'));
    let response: HttpResponse<UploadResponse> = execute_api(
        with_auth(client.post(url).multipart(form), profile),
        "upload response decode failed",
    )
    .await?;
    if !response.payload.success {
        return Err(anyhow!(api_error_message(
            &response.payload,
            "upload failed"
        )));
    }
    let uploaded = response
        .payload
        .data
        .context("missing upload response data")?;
    Ok(ComposeBlobResponse {
        blob_hash: uploaded.hash,
        size_bytes: uploaded.size_bytes,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadResponse {
    hash: String,
    size_bytes: u64,
    original_path: String,
}

async fn upload_blob_via_chunks(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    file_path: &Path,
    bytes: &[u8],
    chunk_size: usize,
    chunk_size_policy: &str,
    manifest_only: bool,
) -> Result<ComposeBlobResponse> {
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asset.bin")
        .to_string();
    let chunk_size = chunk_size.max(64 * 1024);
    let mut chunks = Vec::new();
    let mut cursor = 0usize;
    let mut index = 0usize;
    while cursor < bytes.len() {
        let end = (cursor + chunk_size).min(bytes.len());
        chunks.push(ManifestChunk {
            i: index,
            chunk_hash: StorageHash::hash_bytes(&bytes[cursor..end]),
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
        client,
        profile,
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
                client,
                profile,
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
        }
        cursor = end;
    }

    let manifest_url = format!("{}/v2/manifests", profile.server.trim_end_matches('/'));
    let manifest_req = CreateManifestRequest {
        version: 1,
        chunk_size_policy,
        chunks: &chunks,
        file_meta: serde_json::json!({
            "path": file_path.to_string_lossy(),
            "name": file_name,
            "size": bytes.len(),
        }),
    };
    let manifest_resp: ApiResponse<CreateManifestResponse> = send_authed_api(
        client,
        profile,
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

    if manifest_only {
        return Ok(ComposeBlobResponse {
            blob_hash: manifest.manifest_hash,
            size_bytes: bytes.len() as u64,
        });
    }

    let compose_url = format!("{}/v2/blobs/compose", profile.server.trim_end_matches('/'));
    let compose_resp: ApiResponse<ComposeBlobResponse> = send_authed_api(
        client,
        profile,
        |client, profile| {
            with_auth(
                client.post(&compose_url).json(&ComposeBlobRequest {
                    manifest_hash: &manifest.manifest_hash,
                }),
                profile,
            )
        },
        "compose response decode failed",
    )
    .await?;
    if !compose_resp.success {
        return Err(anyhow!(api_error_message(
            &compose_resp,
            "compose blob failed"
        )));
    }
    compose_resp.data.context("missing compose response data")
}

fn with_auth(request: RequestBuilder, profile: &CliProfile) -> RequestBuilder {
    client::with_auth(request, profile)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn token_expired(profile: &CliProfile) -> bool {
    client::token_expired(profile, now_unix())
}

fn apply_token_pair(profile: &mut CliProfile, pair: TokenPair) {
    client::apply_token_pair(profile, pair, now_unix())
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
    let bytes = fs::read(&file_path)
        .with_context(|| format!("failed to read file {}", file_path.display()))?;

    if bytes.is_empty() {
        return Err(anyhow!("file is empty"));
    }
    let blob = upload_blob_via_chunks(
        &client,
        &mut profile,
        &file_path,
        &bytes,
        args.chunk_size,
        &args.chunk_size_policy,
        args.manifest_only,
    )
    .await?;

    if args.manifest_only {
        println!(
            "chunk-upload manifest-only: manifest_hash={} size_bytes={}",
            blob.blob_hash, blob.size_bytes
        );
    } else {
        cache_blob(&blob.blob_hash, &bytes)?;
        println!(
            "chunk-upload done: blob_hash={} size_bytes={}",
            blob.blob_hash, blob.size_bytes
        );
    }
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

fn submit_error_message<T>(response: &ApiResponse<T>) -> String {
    let message = api_error_message(response, "submit failed");
    if message.contains("Lock conflict") {
        return format!("submit failed: lock conflict: {message}");
    }
    if message.contains("base_changeset_id mismatch") {
        return format!("submit failed: stale base snapshot: {message}");
    }
    if message.contains("Blob not found") {
        return format!("submit failed: missing blob: {message}");
    }
    if message.contains("UNAUTHORIZED") || message.contains("FORBIDDEN") {
        return format!("submit failed: authentication/authorization error: {message}");
    }
    message
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

fn state_paths() -> Result<workspace::StatePaths> {
    Ok(workspace::state_paths_from(&std::env::current_dir()?))
}

fn load_profile() -> Result<CliProfile> {
    let paths = state_paths()?;
    workspace::load_json(&paths.profile_path)
}

fn save_profile(profile: &CliProfile) -> Result<()> {
    let paths = state_paths()?;
    workspace::ensure_state_dirs(&paths)?;
    workspace::save_json(&paths.profile_path, profile)
}

fn load_stage() -> Result<StageFile> {
    let paths = state_paths()?;
    workspace::load_json(&paths.stage_path)
}

fn save_stage(stage: &StageFile) -> Result<()> {
    let paths = state_paths()?;
    workspace::ensure_state_dirs(&paths)?;
    workspace::save_json(&paths.stage_path, stage)
}

fn load_workspace() -> Result<WorkspaceState> {
    let paths = state_paths()?;
    workspace::load_json(&paths.workspace_path)
}

fn save_workspace(workspace_state: &WorkspaceState) -> Result<()> {
    let paths = state_paths()?;
    workspace::ensure_state_dirs(&paths)?;
    workspace::save_json(&paths.workspace_path, workspace_state)
}

fn cache_object_path(hash: &str) -> Result<PathBuf> {
    let paths = state_paths()?;
    Ok(workspace::cache_object_path(&paths, hash))
}

fn cache_blob(hash: &str, bytes: &[u8]) -> Result<()> {
    let paths = state_paths()?;
    workspace::ensure_state_dirs(&paths)?;
    let path = workspace::cache_object_path(&paths, hash);
    fs::write(&path, bytes)
        .with_context(|| format!("failed to write cached blob {}", path.display()))?;
    Ok(())
}

#[allow(dead_code)]
fn _is_inside(path: &Path, maybe_parent: &Path) -> bool {
    path.starts_with(maybe_parent)
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn parse_commands_baseline() {
        let cases = vec![
            vec!["ht", "login", "--server", "http://x", "--token", "t"],
            vec!["ht", "branch", "create", "--repo", "r", "--name", "feat"],
            vec!["ht", "branch", "list", "--repo", "r"],
            vec!["ht", "branch", "switch", "--repo", "r", "--name", "main"],
            vec!["ht", "add", "--path", "a", "--blob", "b"],
            vec!["ht", "remove", "--asset-path", "a"],
            vec!["ht", "submit"],
            vec!["ht", "log"],
            vec!["ht", "rollback", "--to", "c1"],
            vec!["ht", "sync"],
            vec!["ht", "checkout"],
            vec!["ht", "status"],
            vec!["ht", "diff"],
            vec!["ht", "chunk-upload", "--file", "f.bin"],
        ];
        for case in cases {
            assert!(parse_cli_from(case).is_ok());
        }
    }

    fn command_help(mut cmd: clap::Command) -> String {
        let mut out = Vec::new();
        cmd.write_long_help(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn help_snapshot_contains_key_fragments() {
        let root = command_help(Cli::command());
        for fragment in [
            "login",
            "branch",
            "add",
            "remove",
            "submit",
            "log",
            "rollback",
            "sync",
            "checkout",
            "status",
            "diff",
            "chunk-upload",
        ] {
            assert!(root.contains(fragment), "missing fragment: {fragment}");
        }
        let login = command_help(Cli::command().find_subcommand("login").unwrap().clone());
        assert!(login.contains("--branch"));
        assert!(login.contains("[default: main]"));
        let log_help = command_help(Cli::command().find_subcommand("log").unwrap().clone());
        assert!(log_help.contains("--limit"));
        assert!(log_help.contains("[default: 20]"));
        let chunk_help = command_help(
            Cli::command()
                .find_subcommand("chunk-upload")
                .unwrap()
                .clone(),
        );
        assert!(chunk_help.contains("--chunk-size-policy"));
        assert!(chunk_help.contains("[default: fixed-4m]"));
    }

    #[test]
    fn hypertide_paths_and_rw_defaults() {
        let dir = std::env::temp_dir().join(format!("hypertide-cli-test-{}", now_unix()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = workspace::state_paths_from(&dir);
        workspace::ensure_state_dirs(&paths).unwrap();
        assert_eq!(paths.profile_path, dir.join(".hypertide/profile.json"));
        assert_eq!(paths.stage_path, dir.join(".hypertide/stage.json"));
        assert_eq!(paths.workspace_path, dir.join(".hypertide/workspace.json"));
        assert_eq!(
            workspace::cache_object_path(&paths, "abc"),
            dir.join(".hypertide/cache/objects/abc")
        );

        let profile = CliProfile {
            server: "http://x".into(),
            api_key: "k".into(),
            api_key_direct: true,
            access_token: None,
            refresh_token: None,
            access_token_expires_at: None,
            current_repo: Some("r".into()),
            current_branch: "main".into(),
        };
        workspace::save_json(&paths.profile_path, &profile).unwrap();
        let loaded_profile: CliProfile = workspace::load_json(&paths.profile_path).unwrap();
        assert_eq!(loaded_profile.current_branch, "main");

        let stage = StageFile::default_for_branch("dev");
        workspace::save_json(&paths.stage_path, &stage).unwrap();
        let loaded_stage: StageFile = workspace::load_json(&paths.stage_path).unwrap();
        assert!(loaded_stage.assets.is_empty());
        assert_eq!(loaded_stage.base_changeset_id, None);
    }

    #[test]
    fn classify_status_prefers_staged_and_stale_base_signals() {
        assert_eq!(
            classify_asset_status(Some("base"), Some("local"), Some("next"), None, false),
            AssetStatusKind::Staged
        );
        assert_eq!(
            classify_asset_status(Some("base"), Some("base"), None, None, true),
            AssetStatusKind::StaleBase
        );
    }
}
