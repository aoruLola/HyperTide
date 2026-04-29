use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use clap::{ArgGroup, Args, Parser, Subcommand};
use reqwest::{multipart, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const ROOT_BASE_CHANGESET_ID: &str = "ROOT";
const DIRECT_UPLOAD_THRESHOLD_BYTES: usize = 8 * 1024 * 1024;
static NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);

mod workspace;

fn parse_cli_from<I, T>(iter: I) -> std::result::Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(iter)
}

#[derive(Debug, Parser)]
#[command(
    name = "ht",
    version,
    about = "HyperTide CLI",
    long_about = "HyperTide CLI for logging in, syncing assets, staging local changes, and submitting asset versions."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Save server credentials and defaults")]
    Login(LoginArgs),
    #[command(about = "Create, list, or switch branches")]
    Branch(BranchArgs),
    #[command(about = "Stage a local file or existing blob")]
    Add(AddArgs),
    #[command(about = "Stage an asset removal")]
    Remove(RemoveArgs),
    #[command(about = "Save current workspace progress")]
    Save(SaveArgs),
    #[command(about = "Create, restore, or branch from checkpoints")]
    Checkpoint(CheckpointArgs),
    #[command(about = "Approve, promote, or inspect changesets")]
    Changeset(ChangesetArgs),
    #[command(about = "Acquire, release, renew, or inspect locks")]
    Lock(LockArgs),
    #[command(about = "Run trust, witness, audit, replay, and retention operations")]
    Trust(TrustArgs),
    #[command(about = "Submit staged asset changes")]
    Submit(SubmitArgs),
    #[command(about = "Show changeset history")]
    Log(LogArgs),
    #[command(about = "Submit a rollback changeset")]
    Rollback(RollbackArgs),
    #[command(about = "Sync local metadata to a branch snapshot")]
    Sync(SyncArgs),
    #[command(about = "Materialize branch assets into the workspace")]
    Checkout(CheckoutArgs),
    #[command(about = "Show asset status for the workspace")]
    Status(StatusArgs),
    #[command(about = "Show asset-level hash differences")]
    Diff(DiffArgs),
    #[command(about = "Upload a large file through chunk storage")]
    ChunkUpload(ChunkUploadArgs),
}

#[derive(Debug, Args)]
struct LoginArgs {
    #[arg(long, help = "HyperTide server URL")]
    server: String,
    #[arg(long, help = "API key or development token")]
    token: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Send the token directly as an API key instead of exchanging it for JWT tokens"
    )]
    api_key_direct: bool,
    #[arg(long, help = "Default repository for later commands")]
    repo: Option<String>,
    #[arg(
        long,
        default_value = "main",
        help = "Default branch for later commands"
    )]
    branch: String,
}

#[derive(Debug, Args)]
struct BranchArgs {
    #[command(subcommand)]
    command: BranchCommand,
}

#[derive(Debug, Subcommand)]
enum BranchCommand {
    #[command(about = "Create a branch")]
    Create(BranchCreateArgs),
    #[command(about = "List branches")]
    List(BranchListArgs),
    #[command(about = "Switch the default branch")]
    Switch(BranchSwitchArgs),
}

#[derive(Debug, Args)]
struct BranchCreateArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Branch name to create")]
    name: String,
    #[arg(long, help = "Source changeset id for the new branch")]
    from: Option<String>,
}

#[derive(Debug, Args)]
struct BranchListArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
}

#[derive(Debug, Args)]
struct BranchSwitchArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Branch name to make current")]
    name: String,
}

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("input").required(true).args(["file", "blob"])))]
struct AddArgs {
    #[arg(long, help = "Repository asset path", alias = "asset-path")]
    path: Option<String>,
    #[arg(
        long,
        help = "Existing blob hash to stage",
        requires = "path",
        conflicts_with = "file"
    )]
    blob: Option<String>,
    #[arg(long, help = "Local file to upload and stage", conflicts_with = "blob")]
    file: Option<String>,
    #[arg(long, help = "Target branch; defaults to the login profile branch")]
    branch: Option<String>,
}

#[derive(Debug, Args)]
struct RemoveArgs {
    #[arg(long, help = "Repository asset path to remove", alias = "asset-path")]
    path: String,
    #[arg(long, help = "Target branch; defaults to the login profile branch")]
    branch: Option<String>,
}

#[derive(Debug, Args)]
struct SubmitArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Target branch; defaults to the login profile branch")]
    branch: Option<String>,
    #[arg(long, default_value = "submit", help = "Submit message")]
    message: String,
    #[arg(long)]
    visibility: Option<String>,
    #[arg(long)]
    from_checkpoint: Option<String>,
}

#[derive(Debug, Args)]
struct SaveArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    session: Option<String>,
    #[arg(long)]
    message: Option<String>,
}

#[derive(Debug, Args)]
struct CheckpointArgs {
    #[command(subcommand)]
    command: CheckpointCommand,
}

#[derive(Debug, Subcommand)]
enum CheckpointCommand {
    Create(CheckpointCreateArgs),
    Restore(CheckpointRestoreArgs),
    Branch(CheckpointBranchArgs),
    List(CheckpointListArgs),
}

#[derive(Debug, Args)]
struct CheckpointCreateArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    session: Option<String>,
    #[arg(long)]
    message: Option<String>,
}

#[derive(Debug, Args)]
struct CheckpointRestoreArgs {
    #[arg(long)]
    id: String,
}

#[derive(Debug, Args)]
struct CheckpointBranchArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    name: String,
}

#[derive(Debug, Args)]
struct CheckpointListArgs {
    #[arg(long)]
    session: Option<String>,
}

#[derive(Debug, Args)]
struct ChangesetArgs {
    #[command(subcommand)]
    command: ChangesetCommand,
}

#[derive(Debug, Subcommand)]
enum ChangesetCommand {
    Approve(ChangesetActionArgs),
    Promote(ChangesetPromoteArgs),
    Gate(ChangesetActionArgs),
}

#[derive(Debug, Args)]
struct ChangesetActionArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Changeset id")]
    id: String,
}

#[derive(Debug, Args)]
struct ChangesetPromoteArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Changeset id")]
    id: String,
    #[arg(
        long,
        help = "High-risk signing secret; falls back to HT_HIGH_RISK_SIGNING_SECRET"
    )]
    high_risk_secret: Option<String>,
}

#[derive(Debug, Args)]
struct LockArgs {
    #[command(subcommand)]
    command: LockCommand,
}

#[derive(Debug, Subcommand)]
enum LockCommand {
    Acquire(LockPathArgs),
    Release(LockPathArgs),
    Renew(LockPathArgs),
    List,
    ForceRelease(LockForceReleaseArgs),
}

#[derive(Debug, Args)]
struct LockPathArgs {
    #[arg(long, help = "Repository asset path to lock")]
    path: String,
}

#[derive(Debug, Args)]
struct LockForceReleaseArgs {
    #[arg(long, help = "Repository asset path to force release")]
    path: String,
    #[arg(
        long,
        help = "High-risk signing secret; falls back to HT_HIGH_RISK_SIGNING_SECRET"
    )]
    high_risk_secret: Option<String>,
}

#[derive(Debug, Args)]
struct TrustArgs {
    #[command(subcommand)]
    command: TrustCommand,
}

#[derive(Debug, Subcommand)]
enum TrustCommand {
    Checkpoint(TrustCheckpointArgs),
    Witness(TrustWitnessArgs),
    Audit(TrustAuditArgs),
    Replay(TrustReplayArgs),
    Retention(TrustRetentionArgs),
}

#[derive(Debug, Args)]
struct TrustCheckpointArgs {
    #[command(subcommand)]
    command: TrustCheckpointCommand,
}

#[derive(Debug, Subcommand)]
enum TrustCheckpointCommand {
    Generate,
    Latest,
}

#[derive(Debug, Args)]
struct TrustWitnessArgs {
    #[command(subcommand)]
    command: TrustWitnessCommand,
}

#[derive(Debug, Subcommand)]
enum TrustWitnessCommand {
    Attest(TrustWitnessAttestArgs),
    Summary(TrustWitnessSummaryArgs),
    Topology,
}

#[derive(Debug, Args)]
struct TrustWitnessAttestArgs {
    #[arg(long, help = "Trust checkpoint id")]
    checkpoint: String,
    #[arg(long, help = "Witness id")]
    witness: Option<String>,
}

#[derive(Debug, Args)]
struct TrustWitnessSummaryArgs {
    #[arg(long, help = "Trust checkpoint id")]
    checkpoint: String,
}

#[derive(Debug, Args)]
struct TrustAuditArgs {
    #[command(subcommand)]
    command: TrustAuditCommand,
}

#[derive(Debug, Subcommand)]
enum TrustAuditCommand {
    Verify,
    Export(TrustAuditExportArgs),
}

#[derive(Debug, Args)]
struct TrustAuditExportArgs {
    #[arg(long)]
    limit: Option<i64>,
    #[arg(long)]
    before_seq: Option<i64>,
    #[arg(long)]
    action: Option<String>,
    #[arg(long = "actor")]
    actor_id: Option<String>,
}

#[derive(Debug, Args)]
struct TrustReplayArgs {
    #[command(subcommand)]
    command: TrustReplayCommand,
}

#[derive(Debug, Subcommand)]
enum TrustReplayCommand {
    Verify,
    Readiness,
}

#[derive(Debug, Args)]
struct TrustRetentionArgs {
    #[command(subcommand)]
    command: TrustRetentionCommand,
}

#[derive(Debug, Subcommand)]
enum TrustRetentionCommand {
    Policy,
}

#[derive(Debug, Args)]
struct LogArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Branch to inspect; defaults to the login profile branch")]
    branch: Option<String>,
    #[arg(
        long,
        default_value_t = 20,
        help = "Maximum number of changesets to show"
    )]
    limit: usize,
}

#[derive(Debug, Args)]
struct RollbackArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Target branch; defaults to the login profile branch")]
    branch: Option<String>,
    #[arg(long = "to", help = "Changeset id to roll back to")]
    target_changeset_id: String,
    #[arg(
        long,
        help = "Override rollback author; defaults to the authenticated owner"
    )]
    author: Option<String>,
    #[arg(long, help = "Rollback message")]
    message: Option<String>,
}

#[derive(Debug, Args)]
struct SyncArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(long, help = "Branch to sync; defaults to the login profile branch")]
    branch: Option<String>,
    #[arg(long = "to", help = "Optional changeset id to sync to")]
    to_changeset_id: Option<String>,
}

#[derive(Debug, Args)]
struct CheckoutArgs {
    #[arg(long, help = "Repository id; defaults to the login profile repository")]
    repo: Option<String>,
    #[arg(
        long,
        help = "Branch to checkout; defaults to the login profile branch"
    )]
    branch: Option<String>,
    #[arg(long = "to", help = "Optional changeset id to checkout")]
    to_changeset_id: Option<String>,
}

#[derive(Debug, Args)]
struct StatusArgs {
    #[arg(
        long,
        help = "Repository id; defaults to the workspace or login profile repository"
    )]
    repo: Option<String>,
    #[arg(long, help = "Branch to inspect; defaults to the workspace branch")]
    branch: Option<String>,
}

#[derive(Debug, Args)]
struct DiffArgs {
    #[arg(
        long,
        help = "Repository id; defaults to the workspace or login profile repository"
    )]
    repo: Option<String>,
    #[arg(long, help = "Branch to inspect; defaults to the workspace branch")]
    branch: Option<String>,
}

#[derive(Debug, Args)]
struct ChunkUploadArgs {
    #[arg(long, help = "Local file to upload through chunk storage")]
    file: String,
    #[arg(long, default_value_t = 4 * 1024 * 1024, help = "Chunk size in bytes")]
    chunk_size: usize,
    #[arg(
        long,
        default_value = "fixed-4m",
        help = "Chunk size policy recorded in the manifest"
    )]
    chunk_size_policy: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Create only the manifest without composing a final blob"
    )]
    manifest_only: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenPair {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StageFile {
    branch: String,
    base_changeset_id: Option<String>,
    assets: Vec<AssetDelta>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetDelta {
    path: String,
    blob_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WorkspaceFile {
    path: String,
    blob_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceState {
    repo_id: String,
    branch: String,
    workspace_root: String,
    base_changeset_id: Option<String>,
    checked_out_assets: Vec<WorkspaceFile>,
    last_synced_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SessionState {
    current_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileLockInfo {
    file_path: String,
    owner_id: String,
    locked_at: String,
    lease_expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct LockRequest<'a> {
    file_path: &'a str,
}

#[derive(Debug, Serialize)]
struct AttestRequest<'a> {
    witness_id: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComposeBlobRequest<'a> {
    manifest_hash: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComposeBlobResponse {
    blob_hash: String,
    size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AssetStatusKind {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Staged,
    LockedByOther,
    StaleBase,
}

impl AssetStatusKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Unmodified => "unmodified",
            Self::Modified => "modified",
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Staged => "staged",
            Self::LockedByOther => "locked_by_other",
            Self::StaleBase => "stale_base",
        }
    }
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
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    staging_ref: Option<String>,
    #[serde(default)]
    visible_ref: Option<String>,
    assets: Vec<AssetDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangesetGate {
    repo_id: String,
    changeset_id: String,
    branch: String,
    status: String,
    required_state: String,
    can_promote: bool,
    blocking_reason: Option<String>,
    base_changeset_id: Option<String>,
    branch_head_changeset_id: Option<String>,
    staging_ref: Option<String>,
    visible_ref: Option<String>,
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
    #[serde(default)]
    asset_id: Option<String>,
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
    visibility: Option<&'a str>,
    rollback_of: Option<&'a str>,
    author: &'a str,
    message: &'a str,
    intent_id: Option<&'a str>,
    task_id: Option<&'a str>,
    agent_run_id: Option<&'a str>,
    session_id: Option<&'a str>,
    parent_checkpoint_id: Option<&'a str>,
    risk_level: Option<&'a str>,
    semantic_summary: Option<&'a str>,
    assets: &'a [AssetDelta],
}

#[derive(Debug, Serialize)]
struct CreateSessionRequest<'a> {
    repo_id: &'a str,
    branch: &'a str,
    base_changeset_id: Option<&'a str>,
    workspace_root: &'a str,
    trigger_reason: &'a str,
    semantic_summary: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct AgentSessionRecord {
    session_id: String,
}

#[derive(Debug, Serialize)]
struct CreateCheckpointRequest<'a> {
    trigger_reason: &'a str,
    semantic_summary: Option<&'a str>,
    assets: &'a [CheckpointAsset],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckpointAsset {
    asset_id: String,
    path: String,
    blob_hash: String,
}

#[derive(Debug, Deserialize)]
struct SessionCheckpointRecord {
    checkpoint_id: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    repo_id: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    base_changeset_id: Option<String>,
    #[serde(default)]
    trigger_reason: Option<String>,
    #[serde(default)]
    semantic_summary: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    assets: Vec<CheckpointAsset>,
}

#[derive(Debug, Deserialize)]
struct CheckpointPage {
    items: Vec<SessionCheckpointRecord>,
}

#[derive(Debug, Deserialize)]
struct CheckpointSnapshot {
    checkpoint_id: String,
    session_id: String,
    repo_id: String,
    branch: String,
    base_changeset_id: Option<String>,
    workspace_root: String,
    assets: Vec<CheckpointAsset>,
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
        Command::Save(args) => save_progress(args).await,
        Command::Checkpoint(args) => checkpoint(args).await,
        Command::Changeset(args) => changeset(args).await,
        Command::Lock(args) => lock(args).await,
        Command::Trust(args) => trust(args).await,
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
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let payload = CreateBranchRequest {
        repo_id: &repo,
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
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/branches/{}",
        profile.server.trim_end_matches('/'),
        repo
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
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/branches/{}",
        profile.server.trim_end_matches('/'),
        repo
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
    profile.current_repo = Some(repo);
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
            add_file(Path::new(&file), None, &branch).await
        }
        (Some(file), Some(path), None) => {
            add_file(Path::new(&file), Some(&path), &branch).await
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
            "use either `ht add --file <local-file> [--path <repo-path>]` or `ht add --path <repo-path> --blob <hash>`"
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
    upsert_stage_asset(&mut stage, &args.path, None);
    save_stage(&stage)?;
    println!(
        "staged delete for {} on {} ({} asset(s) staged)",
        args.path,
        stage.branch,
        stage.assets.len()
    );
    Ok(())
}

async fn save_progress(args: SaveArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();
    let session_id = ensure_session(
        &client,
        &mut profile,
        &repo,
        &branch,
        args.session.as_deref(),
        args.message.as_deref(),
    )
    .await?;
    let assets = collect_checkpoint_assets(&client, &mut profile, &branch).await?;
    let checkpoint = create_remote_checkpoint(
        &client,
        &mut profile,
        &session_id,
        "agent_save",
        args.message.as_deref(),
        &assets,
        true,
    )
    .await?;
    println!(
        "saved checkpoint {} in session {} ({} assets)",
        checkpoint.checkpoint_id,
        session_id,
        assets.len()
    );
    Ok(())
}

async fn checkpoint(args: CheckpointArgs) -> Result<()> {
    match args.command {
        CheckpointCommand::Create(cmd) => checkpoint_create(cmd).await,
        CheckpointCommand::Restore(cmd) => checkpoint_restore(cmd).await,
        CheckpointCommand::Branch(cmd) => checkpoint_branch(cmd).await,
        CheckpointCommand::List(cmd) => checkpoint_list(cmd).await,
    }
}

async fn checkpoint_create(args: CheckpointCreateArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let branch = args
        .branch
        .unwrap_or_else(|| profile.current_branch.clone());
    let client = reqwest::Client::new();
    let session_id = ensure_session(
        &client,
        &mut profile,
        &repo,
        &branch,
        args.session.as_deref(),
        args.message.as_deref(),
    )
    .await?;
    let assets = collect_checkpoint_assets(&client, &mut profile, &branch).await?;
    let checkpoint = create_remote_checkpoint(
        &client,
        &mut profile,
        &session_id,
        "manual_checkpoint",
        args.message.as_deref(),
        &assets,
        false,
    )
    .await?;
    println!(
        "checkpoint created: {} ({} assets)",
        checkpoint.checkpoint_id,
        assets.len()
    );
    Ok(())
}

async fn checkpoint_restore(args: CheckpointRestoreArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let snapshot = fetch_checkpoint_snapshot(&client, &mut profile, &args.id).await?;
    materialize_checkpoint_snapshot(&client, &mut profile, &snapshot).await?;
    println!(
        "restored checkpoint {} to {} ({} assets)",
        snapshot.checkpoint_id,
        snapshot.workspace_root,
        snapshot.assets.len()
    );
    Ok(())
}

async fn checkpoint_branch(args: CheckpointBranchArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let snapshot = fetch_checkpoint_snapshot(&client, &mut profile, &args.id).await?;
    let payload = CreateBranchRequest {
        repo_id: &snapshot.repo_id,
        branch: &args.name,
        from_changeset_id: snapshot.base_changeset_id.as_deref(),
    };
    let url = format!("{}/v2/branches", profile.server.trim_end_matches('/'));
    let response: ApiResponse<BranchRecord> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "checkpoint branch response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "checkpoint branch failed"
        )));
    }
    let author = fetch_owner_id(&client, &profile).await?;
    let submit_assets = checkpoint_assets_to_deltas(&snapshot.assets);
    let base = snapshot
        .base_changeset_id
        .clone()
        .unwrap_or_else(|| ROOT_BASE_CHANGESET_ID.to_string());
    let message = format!("draft from checkpoint {}", snapshot.checkpoint_id);
    let payload = SubmitRequest {
        repo_id: &snapshot.repo_id,
        branch: &args.name,
        base_changeset_id: &base,
        kind: "normal",
        visibility: Some("draft"),
        rollback_of: None,
        author: &author,
        message: &message,
        intent_id: None,
        task_id: None,
        agent_run_id: None,
        session_id: Some(&snapshot.session_id),
        parent_checkpoint_id: None,
        risk_level: None,
        semantic_summary: Some(&message),
        assets: &submit_assets,
    };
    let submit_url = format!("{}/v2/changesets", profile.server.trim_end_matches('/'));
    let submit_response: ApiResponse<ChangesetRecord> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&submit_url).json(&payload), profile),
        "checkpoint branch draft response decode failed",
    )
    .await?;
    if !submit_response.success {
        return Err(anyhow!(submit_error_message(&submit_response)));
    }
    println!(
        "branch created from checkpoint {}: {}",
        snapshot.checkpoint_id, args.name
    );
    Ok(())
}

async fn checkpoint_list(args: CheckpointListArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let session_id = match args.session {
        Some(session_id) => session_id,
        None => load_session_state()?.current_session_id.ok_or_else(|| {
            anyhow!("session not set. pass --session or create a checkpoint first")
        })?,
    };
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/sessions/{}/checkpoints",
        profile.server.trim_end_matches('/'),
        session_id
    );
    let response: ApiResponse<CheckpointPage> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "checkpoint list response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "checkpoint list failed"
        )));
    }
    let page = response.data.context("missing checkpoint page data")?;
    for checkpoint in page.items {
        println!(
            "{}  session={} repo={} branch={} base={} assets={} reason={} summary={} expires_at={}",
            checkpoint.checkpoint_id,
            checkpoint.session_id.as_deref().unwrap_or(&session_id),
            checkpoint.repo_id.as_deref().unwrap_or("<unknown>"),
            checkpoint.branch.as_deref().unwrap_or("<unknown>"),
            checkpoint
                .base_changeset_id
                .as_deref()
                .unwrap_or(ROOT_BASE_CHANGESET_ID),
            checkpoint.assets.len(),
            checkpoint.trigger_reason.as_deref().unwrap_or("<none>"),
            checkpoint.semantic_summary.as_deref().unwrap_or("<none>"),
            checkpoint.expires_at.as_deref().unwrap_or("<none>")
        );
    }
    Ok(())
}

async fn changeset(args: ChangesetArgs) -> Result<()> {
    match args.command {
        ChangesetCommand::Approve(cmd) => changeset_approve(cmd).await,
        ChangesetCommand::Promote(cmd) => changeset_promote(cmd).await,
        ChangesetCommand::Gate(cmd) => changeset_gate(cmd).await,
    }
}

async fn changeset_approve(args: ChangesetActionArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/changesets/{}/approve?repo_id={}",
        profile.server.trim_end_matches('/'),
        args.id,
        repo
    );
    let response: ApiResponse<ChangesetRecord> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url), profile),
        "changeset approve response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "changeset approve failed"
        )));
    }
    let changeset = response.data.context("missing changeset response data")?;
    print_changeset_action("approved", &changeset);
    Ok(())
}

async fn changeset_promote(args: ChangesetPromoteArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let actor_id = fetch_owner_id(&client, &profile).await?;
    let payload = serde_json::json!({
        "repo_id": repo,
        "changeset_id": args.id,
    });
    let high_risk = build_high_risk_headers(
        args.high_risk_secret.as_deref(),
        "CHANGESET_PROMOTE",
        &actor_id,
        &payload,
    );
    let url = format!(
        "{}/v2/changesets/{}/promote?repo_id={}",
        profile.server.trim_end_matches('/'),
        args.id,
        repo
    );
    let response: ApiResponse<ChangesetRecord> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| {
            let request = with_auth(client.post(&url), profile);
            apply_high_risk_headers(request, high_risk.as_ref())
        },
        "changeset promote response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "changeset promote failed"
        )));
    }
    let changeset = response.data.context("missing changeset response data")?;
    print_changeset_action("promoted", &changeset);
    Ok(())
}

async fn changeset_gate(args: ChangesetActionArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let repo = resolve_repo(&profile, args.repo.as_deref())?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v2/changesets/{}/gate?repo_id={}",
        profile.server.trim_end_matches('/'),
        args.id,
        repo
    );
    let response: ApiResponse<ChangesetGate> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        "changeset gate response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "changeset gate failed"
        )));
    }
    let gate = response.data.context("missing gate response data")?;
    println!(
        "gate {} status={} required={} can_promote={} branch={} base={} head={} blocking={}",
        gate.changeset_id,
        gate.status,
        gate.required_state,
        gate.can_promote,
        gate.branch,
        gate.base_changeset_id
            .as_deref()
            .unwrap_or(ROOT_BASE_CHANGESET_ID),
        gate.branch_head_changeset_id
            .as_deref()
            .unwrap_or(ROOT_BASE_CHANGESET_ID),
        gate.blocking_reason.as_deref().unwrap_or("<none>")
    );
    Ok(())
}

async fn lock(args: LockArgs) -> Result<()> {
    match args.command {
        LockCommand::Acquire(cmd) => lock_acquire(cmd).await,
        LockCommand::Release(cmd) => lock_release(cmd).await,
        LockCommand::Renew(cmd) => lock_renew(cmd).await,
        LockCommand::List => lock_list().await,
        LockCommand::ForceRelease(cmd) => lock_force_release(cmd).await,
    }
}

async fn lock_acquire(args: LockPathArgs) -> Result<()> {
    let lock = send_lock_path_request("acquire", "/v2/locks/acquire", &args.path).await?;
    print_lock("lock acquired", &lock);
    Ok(())
}

async fn lock_release(args: LockPathArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let payload = LockRequest {
        file_path: &args.path,
    };
    let url = format!("{}/v2/locks/release", profile.server.trim_end_matches('/'));
    let response: ApiResponse<()> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "lock release response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(&response, "lock release failed")));
    }
    println!("lock released: {}", args.path);
    Ok(())
}

async fn lock_renew(args: LockPathArgs) -> Result<()> {
    let lock = send_lock_path_request("renew", "/v2/locks/renew", &args.path).await?;
    print_lock("lock renewed", &lock);
    Ok(())
}

async fn lock_list() -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let locks = fetch_locks(&client, &mut profile).await?;
    for lock in locks {
        print_lock("lock", &lock);
    }
    Ok(())
}

async fn lock_force_release(args: LockForceReleaseArgs) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let payload = LockRequest {
        file_path: &args.path,
    };
    let high_risk_payload = serde_json::json!({
        "file_path": args.path,
    });
    let high_risk = build_high_risk_headers(
        args.high_risk_secret.as_deref(),
        "LOCK_FORCE_RELEASE",
        "system-admin",
        &high_risk_payload,
    );
    let url = format!(
        "{}/v2/locks/force-release",
        profile.server.trim_end_matches('/')
    );
    let response: ApiResponse<bool> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| {
            let request = with_auth(client.post(&url).json(&payload), profile);
            apply_high_risk_headers(request, high_risk.as_ref())
        },
        "lock force-release response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "lock force-release failed"
        )));
    }
    println!("lock force-released: {}", args.path);
    Ok(())
}

async fn send_lock_path_request(action: &str, endpoint: &str, path: &str) -> Result<FileLockInfo> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let payload = LockRequest { file_path: path };
    let url = format!("{}{}", profile.server.trim_end_matches('/'), endpoint);
    let response: ApiResponse<FileLockInfo> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "lock response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            &format!("lock {action} failed")
        )));
    }
    response.data.context("missing lock response data")
}

async fn trust(args: TrustArgs) -> Result<()> {
    match args.command {
        TrustCommand::Checkpoint(cmd) => trust_checkpoint(cmd).await,
        TrustCommand::Witness(cmd) => trust_witness(cmd).await,
        TrustCommand::Audit(cmd) => trust_audit(cmd).await,
        TrustCommand::Replay(cmd) => trust_replay(cmd).await,
        TrustCommand::Retention(cmd) => trust_retention(cmd).await,
    }
}

async fn trust_checkpoint(args: TrustCheckpointArgs) -> Result<()> {
    match args.command {
        TrustCheckpointCommand::Generate => {
            trust_json_post(
                "/v2/trust/checkpoints/generate",
                "trust checkpoint generate",
            )
            .await
        }
        TrustCheckpointCommand::Latest => {
            trust_json_get("/v2/trust/checkpoints/latest", "trust checkpoint latest").await
        }
    }
}

async fn trust_witness(args: TrustWitnessArgs) -> Result<()> {
    match args.command {
        TrustWitnessCommand::Attest(cmd) => {
            let mut profile = load_profile()?;
            let client = reqwest::Client::new();
            let payload = AttestRequest {
                witness_id: cmd.witness.as_deref(),
            };
            let url = format!(
                "{}/v2/trust/checkpoints/{}/witness/attest",
                profile.server.trim_end_matches('/'),
                cmd.checkpoint
            );
            let response: ApiResponse<serde_json::Value> = send_authed_api(
                &client,
                &mut profile,
                |client, profile| with_auth(client.post(&url).json(&payload), profile),
                "witness attest response decode failed",
            )
            .await?;
            print_json_response(response, "witness attest failed")
        }
        TrustWitnessCommand::Summary(cmd) => {
            let path = format!("/v2/trust/witness/summary?checkpoint_id={}", cmd.checkpoint);
            trust_json_get(&path, "witness summary").await
        }
        TrustWitnessCommand::Topology => {
            trust_json_get("/v2/trust/witness/topology", "witness topology").await
        }
    }
}

async fn trust_audit(args: TrustAuditArgs) -> Result<()> {
    match args.command {
        TrustAuditCommand::Verify => {
            trust_json_post("/v2/trust/audit/verify", "audit verify").await
        }
        TrustAuditCommand::Export(cmd) => {
            let mut path = "/v2/trust/audit/export".to_string();
            let mut sep = '?';
            push_query_param(&mut path, &mut sep, "limit", cmd.limit);
            push_query_param(&mut path, &mut sep, "before_seq", cmd.before_seq);
            push_query_param(&mut path, &mut sep, "action", cmd.action.as_deref());
            push_query_param(&mut path, &mut sep, "actor_id", cmd.actor_id.as_deref());
            trust_json_get(&path, "audit export").await
        }
    }
}

async fn trust_replay(args: TrustReplayArgs) -> Result<()> {
    match args.command {
        TrustReplayCommand::Verify => {
            trust_json_post("/v2/trust/replay/verify", "replay verify").await
        }
        TrustReplayCommand::Readiness => {
            trust_json_get("/v2/trust/replay/readiness", "replay readiness").await
        }
    }
}

async fn trust_retention(args: TrustRetentionArgs) -> Result<()> {
    match args.command {
        TrustRetentionCommand::Policy => {
            trust_json_get("/v2/trust/retention/policy", "retention policy").await
        }
    }
}

async fn trust_json_get(path: &str, label: &str) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let url = format!("{}{}", profile.server.trim_end_matches('/'), path);
    let response: ApiResponse<serde_json::Value> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.get(&url), profile),
        &format!("{label} response decode failed"),
    )
    .await?;
    print_json_response(response, &format!("{label} failed"))
}

async fn trust_json_post(path: &str, label: &str) -> Result<()> {
    let mut profile = load_profile()?;
    let client = reqwest::Client::new();
    let url = format!("{}{}", profile.server.trim_end_matches('/'), path);
    let response: ApiResponse<serde_json::Value> = send_authed_api(
        &client,
        &mut profile,
        |client, profile| with_auth(client.post(&url), profile),
        &format!("{label} response decode failed"),
    )
    .await?;
    print_json_response(response, &format!("{label} failed"))
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
    let checkpoint_snapshot = if let Some(checkpoint_id) = args.from_checkpoint.as_deref() {
        Some(fetch_checkpoint_snapshot(&client, &mut profile, checkpoint_id).await?)
    } else {
        None
    };
    let checkpoint_assets = checkpoint_snapshot
        .as_ref()
        .map(|snapshot| checkpoint_assets_to_deltas(&snapshot.assets))
        .unwrap_or_default();
    let submit_assets = if checkpoint_assets.is_empty() {
        stage.assets.clone()
    } else {
        checkpoint_assets
    };
    if submit_assets.is_empty() {
        return Err(anyhow!(
            "nothing staged; use `ht add --file <local-file> [--path <repo-path>]` first"
        ));
    }

    let base_hint = checkpoint_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.base_changeset_id.as_deref())
        .or(stage.base_changeset_id.as_deref());
    let base = resolve_base_changeset(&client, &mut profile, &repo, &branch, base_hint).await?;
    let author = fetch_owner_id(&client, &profile).await?;
    let visibility = args
        .visibility
        .as_deref()
        .or_else(|| args.from_checkpoint.as_ref().map(|_| "draft"));
    let session_id = checkpoint_snapshot
        .as_ref()
        .map(|snapshot| snapshot.session_id.as_str());
    let checkpoint_id = checkpoint_snapshot
        .as_ref()
        .map(|snapshot| snapshot.checkpoint_id.as_str());
    let payload = SubmitRequest {
        repo_id: &repo,
        branch: &branch,
        base_changeset_id: &base,
        kind: "normal",
        visibility,
        rollback_of: None,
        author: &author,
        message: &args.message,
        intent_id: None,
        task_id: None,
        agent_run_id: None,
        session_id,
        parent_checkpoint_id: checkpoint_id,
        risk_level: None,
        semantic_summary: Some(&args.message),
        assets: &submit_assets,
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
    if checkpoint_snapshot.is_none() {
        stage.base_changeset_id = Some(changeset.changeset_id.clone());
        stage.branch = branch.clone();
        stage.assets.clear();
        save_stage(&stage)?;
    }
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

#[derive(Debug, Clone)]
struct HighRiskHeaders {
    nonce: String,
    timestamp: i64,
    signature: String,
}

fn build_high_risk_headers(
    explicit_secret: Option<&str>,
    action: &str,
    actor_id: &str,
    payload: &serde_json::Value,
) -> Option<HighRiskHeaders> {
    let secret = explicit_secret
        .map(str::to_string)
        .or_else(|| env::var("HT_HIGH_RISK_SIGNING_SECRET").ok())?;
    let nonce = next_nonce();
    let timestamp = now_unix();
    let signature = high_risk_signature(&secret, action, actor_id, &nonce, timestamp, payload);
    Some(HighRiskHeaders {
        nonce,
        timestamp,
        signature,
    })
}

fn apply_high_risk_headers(
    request: RequestBuilder,
    headers: Option<&HighRiskHeaders>,
) -> RequestBuilder {
    let Some(headers) = headers else {
        return request;
    };
    request
        .header("X-HT-Nonce", &headers.nonce)
        .header("X-HT-Timestamp", headers.timestamp.to_string())
        .header("X-HT-Signature", &headers.signature)
}

fn high_risk_signature(
    secret: &str,
    action: &str,
    actor_id: &str,
    nonce: &str,
    timestamp: i64,
    payload: &serde_json::Value,
) -> String {
    let payload_hash = blake3::hash(
        serde_json::to_string(payload)
            .unwrap_or_default()
            .as_bytes(),
    )
    .to_hex()
    .to_string();
    let material = format!("{secret}|{action}|{actor_id}|{nonce}|{timestamp}|{payload_hash}");
    blake3::hash(material.as_bytes()).to_hex().to_string()
}

fn next_nonce() -> String {
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{counter}", now_unix(), std::process::id())
}

fn print_changeset_action(action: &str, changeset: &ChangesetRecord) {
    println!(
        "changeset {}: {} status={} branch={} staging_ref={} visible_ref={}",
        action,
        changeset.changeset_id,
        changeset.status.as_deref().unwrap_or("<unknown>"),
        changeset.branch,
        changeset.staging_ref.as_deref().unwrap_or("<none>"),
        changeset.visible_ref.as_deref().unwrap_or("<none>")
    );
}

fn print_lock(label: &str, lock: &FileLockInfo) {
    println!(
        "{}: {} owner={} locked_at={} lease_expires_at={}",
        label,
        lock.file_path,
        lock.owner_id,
        lock.locked_at,
        lock.lease_expires_at.as_deref().unwrap_or("<none>")
    );
}

fn print_json_response(response: ApiResponse<serde_json::Value>, fallback: &str) -> Result<()> {
    if !response.success {
        return Err(anyhow!(api_error_message(&response, fallback)));
    }
    let data = response.data.context("missing response data")?;
    println!("{}", serde_json::to_string_pretty(&data)?);
    Ok(())
}

fn push_query_param<T: ToString>(path: &mut String, sep: &mut char, key: &str, value: Option<T>) {
    let Some(value) = value else {
        return;
    };
    path.push(*sep);
    *sep = '&';
    path.push_str(key);
    path.push('=');
    path.push_str(&value.to_string());
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

async fn ensure_session(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    repo: &str,
    branch: &str,
    requested_session: Option<&str>,
    message: Option<&str>,
) -> Result<String> {
    if let Some(session_id) = requested_session {
        return Ok(session_id.to_string());
    }
    if let Ok(session) = load_session_state() {
        if let Some(session_id) = session.current_session_id {
            return Ok(session_id);
        }
    }

    let workspace_root = std::env::current_dir()?.to_string_lossy().to_string();
    let base_changeset_id = load_workspace()
        .ok()
        .and_then(|workspace| workspace.base_changeset_id)
        .or_else(|| load_stage().ok().and_then(|stage| stage.base_changeset_id));
    let payload = CreateSessionRequest {
        repo_id: repo,
        branch,
        base_changeset_id: base_changeset_id.as_deref(),
        workspace_root: &workspace_root,
        trigger_reason: "agent_session",
        semantic_summary: message,
    };
    let url = format!("{}/v2/sessions", profile.server.trim_end_matches('/'));
    let response: ApiResponse<AgentSessionRecord> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "create session response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "create session failed"
        )));
    }
    let session = response.data.context("missing session response data")?;
    save_session_state(&SessionState {
        current_session_id: Some(session.session_id.clone()),
    })?;
    Ok(session.session_id)
}

async fn create_remote_checkpoint(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    session_id: &str,
    trigger_reason: &str,
    message: Option<&str>,
    assets: &[CheckpointAsset],
    save_endpoint: bool,
) -> Result<SessionCheckpointRecord> {
    let payload = CreateCheckpointRequest {
        trigger_reason,
        semantic_summary: message,
        assets,
    };
    let suffix = if save_endpoint {
        "save".to_string()
    } else {
        "checkpoints".to_string()
    };
    let url = format!(
        "{}/v2/sessions/{}/{}",
        profile.server.trim_end_matches('/'),
        session_id,
        suffix
    );
    let response: ApiResponse<SessionCheckpointRecord> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.post(&url).json(&payload), profile),
        "checkpoint response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "checkpoint create failed"
        )));
    }
    response.data.context("missing checkpoint response data")
}

async fn fetch_checkpoint_snapshot(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    checkpoint_id: &str,
) -> Result<CheckpointSnapshot> {
    let url = format!(
        "{}/v2/checkpoints/{}/snapshot",
        profile.server.trim_end_matches('/'),
        checkpoint_id
    );
    let response: ApiResponse<CheckpointSnapshot> = send_authed_api(
        client,
        profile,
        |client, profile| with_auth(client.get(&url), profile),
        "checkpoint snapshot response decode failed",
    )
    .await?;
    if !response.success {
        return Err(anyhow!(api_error_message(
            &response,
            "checkpoint snapshot failed"
        )));
    }
    response.data.context("missing checkpoint snapshot data")
}

async fn collect_checkpoint_assets(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    branch: &str,
) -> Result<Vec<CheckpointAsset>> {
    let stage = load_stage().unwrap_or_else(|_| StageFile::default_for_branch(branch));
    if !stage.assets.is_empty() {
        return Ok(stage
            .assets
            .into_iter()
            .filter_map(|asset| {
                asset.blob_hash.map(|blob_hash| CheckpointAsset {
                    asset_id: asset.path.clone(),
                    path: asset.path,
                    blob_hash,
                })
            })
            .collect());
    }
    let workspace = load_workspace()?;
    let mut assets = collect_workspace_checkpoint_assets(&workspace)?;
    let workspace_root = PathBuf::from(&workspace.workspace_root);
    for asset in &mut assets {
        let target = resolve_workspace_target(&workspace_root, &asset.path)?;
        let bytes =
            fs::read(&target).with_context(|| format!("failed to read {}", target.display()))?;
        let uploaded = upload_blob_from_bytes(
            client,
            profile,
            &target,
            &bytes,
            4 * 1024 * 1024,
            "fixed-4m",
            false,
        )
        .await?;
        asset.blob_hash = uploaded.blob_hash;
    }
    Ok(assets)
}

fn collect_workspace_checkpoint_assets(workspace: &WorkspaceState) -> Result<Vec<CheckpointAsset>> {
    let workspace_root = PathBuf::from(&workspace.workspace_root);
    workspace
        .checked_out_assets
        .iter()
        .map(|asset| {
            let target = resolve_workspace_target(&workspace_root, &asset.path)?;
            let bytes = fs::read(&target)
                .with_context(|| format!("failed to read {}", target.display()))?;
            Ok(CheckpointAsset {
                asset_id: asset.path.clone(),
                path: asset.path.clone(),
                blob_hash: hash_bytes(&bytes),
            })
        })
        .collect()
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

fn checkpoint_assets_to_deltas(assets: &[CheckpointAsset]) -> Vec<AssetDelta> {
    assets
        .iter()
        .map(|asset| AssetDelta {
            path: asset.path.clone(),
            blob_hash: Some(asset.blob_hash.clone()),
        })
        .collect()
}

async fn materialize_checkpoint_snapshot(
    client: &reqwest::Client,
    profile: &mut CliProfile,
    snapshot: &CheckpointSnapshot,
) -> Result<()> {
    let workspace_root = std::env::current_dir()?;
    let mut checked_out_assets = Vec::with_capacity(snapshot.assets.len());
    for asset in &snapshot.assets {
        let target = resolve_workspace_target(&workspace_root, &asset.path)?;
        let bytes = fetch_blob_bytes(client, profile, &asset.blob_hash).await?;
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
    save_workspace(&WorkspaceState {
        repo_id: snapshot.repo_id.clone(),
        branch: snapshot.branch.clone(),
        workspace_root: workspace_root.to_string_lossy().to_string(),
        base_changeset_id: snapshot.base_changeset_id.clone(),
        checked_out_assets,
        last_synced_at: now_unix(),
    })?;
    let mut stage = StageFile::default_for_branch(&snapshot.branch);
    stage.base_changeset_id = snapshot.base_changeset_id.clone();
    save_stage(&stage)?;
    Ok(())
}

fn resolve_workspace_target(workspace_root: &Path, asset_path: &str) -> Result<PathBuf> {
    let normalized = asset_path.replace('/', &std::path::MAIN_SEPARATOR.to_string());
    let candidate = PathBuf::from(&normalized);
    if candidate.is_absolute() {
        return Err(anyhow!(
            "checkpoint asset path must be relative: {asset_path}"
        ));
    }
    let has_parent = candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir));
    if has_parent {
        return Err(anyhow!(
            "checkpoint asset path escapes workspace: {asset_path}"
        ));
    }
    let target = workspace_root.join(candidate);
    if !_is_inside(&target, workspace_root) {
        return Err(anyhow!(
            "checkpoint asset path escapes workspace: {asset_path}"
        ));
    }
    Ok(target)
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
    now_unix() >= expires_at.saturating_sub(30)
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

fn save_workspace(workspace: &WorkspaceState) -> Result<()> {
    ensure_state_dir()?;
    let path = workspace_path()?;
    fs::write(path, serde_json::to_vec_pretty(workspace)?)?;
    Ok(())
}

fn load_session_state() -> Result<SessionState> {
    let path = session_path()?;
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let session: SessionState = serde_json::from_str(&content)?;
    Ok(session)
}

fn save_session_state(session: &SessionState) -> Result<()> {
    ensure_state_dir()?;
    let path = session_path()?;
    fs::write(path, serde_json::to_vec_pretty(session)?)?;
    Ok(())
}

fn ensure_state_dir() -> Result<()> {
    let dir = state_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let cache_dir = cache_dir()?;
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
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

fn workspace_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("workspace.json"))
}

fn session_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("session.json"))
}

fn cache_dir() -> Result<PathBuf> {
    Ok(state_dir()?.join("cache").join("objects"))
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
    use clap::CommandFactory;

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

    #[test]
    fn classify_status_covers_modified_added_and_deleted() {
        assert_eq!(
            classify_asset_status(Some("base"), Some("other"), None, None, false),
            AssetStatusKind::Modified
        );
        assert_eq!(
            classify_asset_status(None, Some("new"), None, None, false),
            AssetStatusKind::Added
        );
        assert_eq!(
            classify_asset_status(Some("base"), None, None, None, false),
            AssetStatusKind::Deleted
        );
    }

    #[test]
    fn top_level_help_includes_session_checkpoint_commands() {
        let help = Cli::command().render_long_help().to_string();
        assert!(help.contains("Save current workspace progress"));
        assert!(help.contains("Create, restore, or branch from checkpoints"));
        assert!(help.contains("Approve, promote, or inspect changesets"));
        assert!(help.contains("Acquire, release, renew, or inspect locks"));
        assert!(help.contains("Run trust, witness, audit, replay, and retention operations"));
    }

    #[test]
    fn submit_accepts_checkpoint_and_visibility_flags() {
        let mut command = Cli::command();
        let submit = command
            .find_subcommand_mut("submit")
            .expect("submit command")
            .render_long_help()
            .to_string();
        assert!(submit.contains("--from-checkpoint"));
        assert!(submit.contains("--visibility"));
    }

    #[test]
    fn changeset_commands_accept_required_flags() {
        assert!(Cli::try_parse_from([
            "ht",
            "changeset",
            "approve",
            "--repo",
            "repo-a",
            "--id",
            "cs-1",
        ])
        .is_ok());
        assert!(Cli::try_parse_from([
            "ht",
            "changeset",
            "promote",
            "--repo",
            "repo-a",
            "--id",
            "cs-1",
            "--high-risk-secret",
            "secret",
        ])
        .is_ok());
        assert!(Cli::try_parse_from([
            "ht",
            "changeset",
            "gate",
            "--repo",
            "repo-a",
            "--id",
            "cs-1",
        ])
        .is_ok());
    }

    #[test]
    fn checkpoint_list_accepts_optional_session() {
        assert!(Cli::try_parse_from(["ht", "checkpoint", "list"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "checkpoint", "list", "--session", "sess-1"]).is_ok());
    }

    #[test]
    fn lock_commands_accept_paths_and_high_risk_secret() {
        assert!(
            Cli::try_parse_from(["ht", "lock", "acquire", "--path", "Content/a.uasset"]).is_ok()
        );
        assert!(
            Cli::try_parse_from(["ht", "lock", "release", "--path", "Content/a.uasset"]).is_ok()
        );
        assert!(Cli::try_parse_from(["ht", "lock", "renew", "--path", "Content/a.uasset"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "lock", "list"]).is_ok());
        assert!(Cli::try_parse_from([
            "ht",
            "lock",
            "force-release",
            "--path",
            "Content/a.uasset",
            "--high-risk-secret",
            "secret",
        ])
        .is_ok());
    }

    #[test]
    fn trust_commands_accept_governance_arguments() {
        assert!(Cli::try_parse_from(["ht", "trust", "checkpoint", "generate"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "trust", "checkpoint", "latest"]).is_ok());
        assert!(Cli::try_parse_from([
            "ht",
            "trust",
            "witness",
            "attest",
            "--checkpoint",
            "tc-1",
            "--witness",
            "witness-a",
        ])
        .is_ok());
        assert!(Cli::try_parse_from(
            ["ht", "trust", "witness", "summary", "--checkpoint", "tc-1",]
        )
        .is_ok());
        assert!(Cli::try_parse_from(["ht", "trust", "witness", "topology"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "trust", "audit", "verify"]).is_ok());
        assert!(Cli::try_parse_from([
            "ht",
            "trust",
            "audit",
            "export",
            "--limit",
            "10",
            "--before-seq",
            "99",
            "--action",
            "CHANGESET_PROMOTED",
            "--actor",
            "alice",
        ])
        .is_ok());
        assert!(Cli::try_parse_from(["ht", "trust", "replay", "verify"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "trust", "replay", "readiness"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "trust", "retention", "policy"]).is_ok());
    }

    #[test]
    fn high_risk_signature_is_stable_for_fixed_inputs() {
        let payload = serde_json::json!({
            "repo_id": "repo-a",
            "changeset_id": "cs-1",
        });

        let signature = high_risk_signature(
            "secret",
            "CHANGESET_PROMOTE",
            "alice",
            "nonce-1",
            1_700_000_000,
            &payload,
        );

        assert_eq!(
            signature,
            "a52cc938a612b8115516cdb7afc42e40eb7edae89ca98778fdeea92ee232c301"
        );
    }

    #[test]
    fn workspace_checkpoint_assets_hash_current_disk_files() {
        let unique = now_unix();
        let root = std::env::temp_dir().join(format!("hypertide-cli-save-{unique}"));
        fs::create_dir_all(root.join("Assets")).expect("mkdir");
        let asset_path = "Assets/a.txt";
        fs::write(root.join(asset_path), b"changed").expect("write file");
        let workspace = WorkspaceState {
            repo_id: "repo-a".to_string(),
            branch: "main".to_string(),
            workspace_root: root.to_string_lossy().to_string(),
            base_changeset_id: Some("ROOT".to_string()),
            checked_out_assets: vec![WorkspaceFile {
                path: asset_path.to_string(),
                blob_hash: "old-hash".to_string(),
            }],
            last_synced_at: 0,
        };

        let assets = collect_workspace_checkpoint_assets(&workspace).expect("assets");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].blob_hash, hash_bytes(b"changed"));
    }

    #[test]
    fn restore_target_rejects_paths_outside_workspace() {
        let root = PathBuf::from("E:/workspace/game");

        assert!(resolve_workspace_target(&root, "../outside.txt").is_err());
        assert!(resolve_workspace_target(&root, "C:/outside.txt").is_err());
        assert!(resolve_workspace_target(&root, "Assets/a.txt").is_ok());
    }

    #[test]
    fn add_without_input_fails_at_argument_validation() {
        let error = Cli::try_parse_from(["ht", "add"]).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("--file <FILE>"));
        assert!(message.contains("--blob <BLOB>"));
    }

    #[test]
    fn branch_commands_accept_repo_from_profile_default() {
        assert!(Cli::try_parse_from(["ht", "branch", "create", "--name", "feature/test"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "branch", "list"]).is_ok());
        assert!(Cli::try_parse_from(["ht", "branch", "switch", "--name", "main"]).is_ok());
    }

    #[test]
    fn path_aliases_preserve_asset_path_compatibility() {
        assert!(Cli::try_parse_from([
            "ht",
            "add",
            "--file",
            "tree.uasset",
            "--path",
            "Content/tree.uasset",
        ])
        .is_ok());
        assert!(
            Cli::try_parse_from(["ht", "remove", "--asset-path", "Content/tree.uasset",]).is_ok()
        );
    }

    #[test]
    fn help_text_explains_core_commands_and_path_arguments() {
        let top_help = Cli::command().render_long_help().to_string();
        assert!(top_help.contains("Save server credentials and defaults"));
        assert!(top_help.contains("Stage a local file or existing blob"));

        let mut command = Cli::command();
        let add = command
            .find_subcommand_mut("add")
            .expect("add subcommand exists");
        let add_help = add.render_long_help().to_string();
        assert!(add_help.contains("Local file to upload and stage"));
        assert!(add_help.contains("Repository asset path"));
    }
}
