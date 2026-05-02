use anyhow::Result;
use clap::Args;

use crate::utils::*;

#[derive(Debug, Args)]
pub(crate) struct DoctorArgs {}

pub(crate) async fn execute(_args: DoctorArgs) -> Result<()> {
    let mut ok_count = 0u32;
    let mut warn_count = 0u32;
    let mut err_count = 0u32;

    // 1. Login state
    match load_profile() {
        Ok(profile) => {
            let mode = if profile.api_key_direct {
                "api-key-direct"
            } else {
                "jwt"
            };
            println!("[ok]   login: server={}, mode={}", profile.server, mode);
            ok_count += 1;

            // 2. Server connectivity
            let client = reqwest::Client::new();
            let health_url = format!(
                "{}/health/ready",
                profile.server.trim_end_matches('/')
            );
            match client
                .get(&health_url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("[ok]   server: {} responding", profile.server);
                    ok_count += 1;
                }
                Ok(resp) => {
                    println!(
                        "[warn] server: {} returned HTTP {}",
                        profile.server,
                        resp.status()
                    );
                    warn_count += 1;
                }
                Err(e) => {
                    println!("[err]  server: {} unreachable ({})", profile.server, e);
                    err_count += 1;
                }
            }

            // 3. Default repo
            match &profile.current_repo {
                Some(repo) => {
                    println!("[ok]   default repo: {}", repo);
                    ok_count += 1;
                }
                None => {
                    println!("[warn] default repo: not set (use --repo or re-login)");
                    warn_count += 1;
                }
            }

            // 4. Default branch
            println!("[ok]   default branch: {}", profile.current_branch);
            ok_count += 1;

            // 5. Token expiry
            if !profile.api_key_direct {
                if token_expired(&profile) {
                    println!("[warn] token: expired — run 'ht login' to refresh");
                    warn_count += 1;
                } else if let Some(expires_at) = profile.access_token_expires_at {
                    let remaining = expires_at - now_unix();
                    if remaining < 300 {
                        println!(
                            "[warn] token: expires in {}s — consider 'ht login' to refresh",
                            remaining
                        );
                        warn_count += 1;
                    } else {
                        println!("[ok]   token: valid ({}s remaining)", remaining);
                        ok_count += 1;
                    }
                }
            }
        }
        Err(_) => {
            println!("[err]  login: not configured — run 'ht login --server <url> --token <key>'");
            err_count += 1;
        }
    }

    // 6. Workspace state
    match load_workspace() {
        Ok(workspace) => {
            println!(
                "[ok]   workspace: {} assets checked out (branch={})",
                workspace.checked_out_assets.len(),
                workspace.branch
            );
            ok_count += 1;
        }
        Err(_) => {
            println!("[warn] workspace: not initialized — run 'ht checkout'");
            warn_count += 1;
        }
    }

    // 7. Stage state
    match load_stage() {
        Ok(stage) => {
            if stage.assets.is_empty() {
                println!("[ok]   stage: empty");
            } else {
                println!(
                    "[warn] stage: {} asset(s) pending — run 'ht submit' or 'ht stage clear'",
                    stage.assets.len()
                );
                warn_count += 1;
            }
            ok_count += 1;
        }
        Err(_) => {
            println!("[ok]   stage: not initialized (will be created on first 'ht add')");
            ok_count += 1;
        }
    }

    // Summary
    println!();
    println!(
        "doctor: {} ok, {} warning(s), {} error(s)",
        ok_count, warn_count, err_count
    );
    if err_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}
