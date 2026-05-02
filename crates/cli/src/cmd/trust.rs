use anyhow::Result;
use clap::{Args, Subcommand};

use crate::utils::*;

#[derive(Debug, Args)]
pub(crate) struct TrustArgs {
    #[command(subcommand)]
    pub command: TrustCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum TrustCommand {
    #[command(about = "Generate or inspect system state attestations")]
    Checkpoint(TrustCheckpointArgs),
    #[command(about = "Witness attestation operations")]
    Witness(TrustWitnessArgs),
    #[command(about = "Audit chain verification and export")]
    Audit(TrustAuditArgs),
    #[command(about = "Event replay verification")]
    Replay(TrustReplayArgs),
    #[command(about = "Retention policy inspection")]
    Retention(TrustRetentionArgs),
}

#[derive(Debug, Args)]
pub(crate) struct TrustCheckpointArgs {
    #[command(subcommand)]
    pub command: TrustCheckpointCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum TrustCheckpointCommand {
    Generate,
    Latest,
}

#[derive(Debug, Args)]
pub(crate) struct TrustWitnessArgs {
    #[command(subcommand)]
    pub command: TrustWitnessCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum TrustWitnessCommand {
    Attest(TrustWitnessAttestArgs),
    Summary(TrustWitnessSummaryArgs),
    Topology,
}

#[derive(Debug, Args)]
pub(crate) struct TrustWitnessAttestArgs {
    #[arg(long, help = "Trust checkpoint id")]
    pub checkpoint: String,
    #[arg(long, help = "Witness id")]
    pub witness: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct TrustWitnessSummaryArgs {
    #[arg(long, help = "Trust checkpoint id")]
    pub checkpoint: String,
}

#[derive(Debug, Args)]
pub(crate) struct TrustAuditArgs {
    #[command(subcommand)]
    pub command: TrustAuditCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum TrustAuditCommand {
    Verify,
    Export(TrustAuditExportArgs),
}

#[derive(Debug, Args)]
pub(crate) struct TrustAuditExportArgs {
    #[arg(long)]
    pub limit: Option<i64>,
    #[arg(long)]
    pub before_seq: Option<i64>,
    #[arg(long)]
    pub action: Option<String>,
    #[arg(long = "actor")]
    pub actor_id: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct TrustReplayArgs {
    #[command(subcommand)]
    pub command: TrustReplayCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum TrustReplayCommand {
    #[command(about = "Verify event replay consistency (optionally from a checkpoint)")]
    Verify(TrustReplayVerifyArgs),
    #[command(about = "Check replay readiness")]
    Readiness,
}

#[derive(Debug, Args)]
pub(crate) struct TrustReplayVerifyArgs {
    #[arg(long, help = "Start incremental replay from this checkpoint id")]
    pub from_checkpoint: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct TrustRetentionArgs {
    #[command(subcommand)]
    pub command: TrustRetentionCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum TrustRetentionCommand {
    Policy,
}

pub(crate) async fn execute(args: TrustArgs) -> Result<()> {
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
        TrustCheckpointCommand::Generate => trust_json_post("/v2/trust/checkpoint/generate", "generate checkpoint").await,
        TrustCheckpointCommand::Latest => trust_json_get("/v2/trust/checkpoint/latest", "latest checkpoint").await,
    }
}

async fn trust_witness(args: TrustWitnessArgs) -> Result<()> {
    match args.command {
        TrustWitnessCommand::Attest(cmd) => {
            let payload = AttestRequest {
                witness_id: cmd.witness.as_deref(),
            };
            let mut profile = load_profile()?;
            let client = reqwest::Client::new();
            let url = format!(
                "{}/v2/trust/witness/{}/attest",
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
            trust_json_get(
                &format!("/v2/trust/witness/{}/summary", cmd.checkpoint),
                "witness summary",
            )
            .await
        }
        TrustWitnessCommand::Topology => {
            trust_json_get("/v2/trust/witness/topology", "witness topology").await
        }
    }
}

async fn trust_audit(args: TrustAuditArgs) -> Result<()> {
    match args.command {
        TrustAuditCommand::Verify => trust_json_get("/v2/trust/audit/verify", "audit verify").await,
        TrustAuditCommand::Export(cmd) => {
            let mut path = "/v2/trust/audit/export".to_string();
            let mut sep = '?';
            push_query_param(&mut path, &mut sep, "limit", cmd.limit);
            push_query_param(&mut path, &mut sep, "before_seq", cmd.before_seq);
            push_query_param(&mut path, &mut sep, "action", cmd.action);
            push_query_param(&mut path, &mut sep, "actor", cmd.actor_id);
            trust_json_get(&path, "audit export").await
        }
    }
}

async fn trust_replay(args: TrustReplayArgs) -> Result<()> {
    match args.command {
        TrustReplayCommand::Verify(cmd) => {
            let mut path = "/v2/trust/replay/verify".to_string();
            if let Some(ref cp) = cmd.from_checkpoint {
                path.push_str("?from_checkpoint=");
                path.push_str(cp);
            }
            trust_json_get(&path, "replay verify").await
        }
        TrustReplayCommand::Readiness => {
            trust_json_get("/v2/trust/replay/readiness", "replay readiness").await
        }
    }
}

async fn trust_retention(args: TrustRetentionArgs) -> Result<()> {
    match args.command {
        TrustRetentionCommand::Policy => trust_json_get("/v2/trust/retention/policy", "retention policy").await,
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
    print_json_response(response, label)
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
    print_json_response(response, label)
}
