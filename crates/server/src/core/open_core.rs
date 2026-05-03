#![allow(dead_code)]

use async_trait::async_trait;

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn describe(&self) -> &'static str;
}

#[async_trait]
pub trait PolicyEngine: Send + Sync {
    async fn evaluate(&self, _operation: &str) -> PolicyDecision;
}

#[async_trait]
pub trait AttestationProvider: Send + Sync {
    async fn attest(&self, _subject: &str) -> Result<Option<String>, String>;
}

#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn publish(&self, _event_type: &str, _payload: &serde_json::Value) -> Result<(), String>;
}

#[async_trait]
pub trait WitnessProvider: Send + Sync {
    async fn describe_topology(&self) -> serde_json::Value;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CommunityAuthProvider;

#[derive(Debug, Default, Clone, Copy)]
pub struct CommunityPolicyEngine;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAttestationProvider;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAuditSink;

#[derive(Debug, Default, Clone, Copy)]
pub struct CommunityWitnessProvider;

#[async_trait]
impl AuthProvider for CommunityAuthProvider {
    async fn describe(&self) -> &'static str {
        "community-api-key-jwt"
    }
}

#[async_trait]
impl PolicyEngine for CommunityPolicyEngine {
    async fn evaluate(&self, _operation: &str) -> PolicyDecision {
        PolicyDecision::Allow
    }
}

#[async_trait]
impl AttestationProvider for NoopAttestationProvider {
    async fn attest(&self, _subject: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
}

#[async_trait]
impl AuditSink for NoopAuditSink {
    async fn publish(&self, _event_type: &str, _payload: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}

#[async_trait]
impl WitnessProvider for CommunityWitnessProvider {
    async fn describe_topology(&self) -> serde_json::Value {
        serde_json::json!({"provider": "community-witness"})
    }
}
