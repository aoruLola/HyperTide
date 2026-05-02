use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RetentionPolicy {
    pub audit_retention_days: i64,
    pub event_retention_days: i64,
    pub checkpoint_retention_days: i64,
    pub dry_run_only: bool,
}

impl RetentionPolicy {
    pub fn from_env() -> Self {
        Self {
            audit_retention_days: read_env_i64("RETENTION_AUDIT_DAYS", 3650),
            event_retention_days: read_env_i64("RETENTION_EVENT_DAYS", 365),
            checkpoint_retention_days: read_env_i64("RETENTION_CHECKPOINT_DAYS", 365),
            dry_run_only: read_env_bool("RETENTION_DRY_RUN_ONLY", true),
        }
    }
}

fn read_env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(default)
}

fn read_env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}
