use std::{fs, sync::Arc};

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub permissions: Vec<String>,
    #[serde(rename = "typ")]
    pub token_type: String,
    pub iss: String,
    pub iat: usize,
    pub exp: usize,
    pub jti: String,
    pub family_id: Option<String>,
    pub parent_jti: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

struct TokenServiceInner {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
}

#[derive(Clone)]
pub struct TokenService {
    inner: Arc<TokenServiceInner>,
}

impl TokenService {
    pub fn from_env() -> Result<Self, String> {
        let private_key_path = std::env::var("JWT_PRIVATE_KEY_PATH")
            .map_err(|_| "JWT_PRIVATE_KEY_PATH is required".to_string())?;
        let public_key_path = std::env::var("JWT_PUBLIC_KEY_PATH")
            .map_err(|_| "JWT_PUBLIC_KEY_PATH is required".to_string())?;
        let issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "hypertide".to_string());

        let private_pem = fs::read(private_key_path)
            .map_err(|e| format!("failed to read JWT private key: {e}"))?;
        let public_pem =
            fs::read(public_key_path).map_err(|e| format!("failed to read JWT public key: {e}"))?;

        let encoding_key = EncodingKey::from_rsa_pem(&private_pem)
            .map_err(|e| format!("invalid JWT private key: {e}"))?;
        let decoding_key = DecodingKey::from_rsa_pem(&public_pem)
            .map_err(|e| format!("invalid JWT public key: {e}"))?;

        Ok(Self {
            inner: Arc::new(TokenServiceInner {
                encoding_key,
                decoding_key,
                issuer,
            }),
        })
    }

    fn issue_token(
        &self,
        sub: &str,
        permissions: Vec<String>,
        token_type: &str,
        ttl_secs: i64,
        family_id: Option<String>,
        parent_jti: Option<String>,
    ) -> Result<String, String> {
        let now = Utc::now().timestamp().max(0) as usize;
        let exp = (Utc::now().timestamp() + ttl_secs).max(0) as usize;
        let claims = TokenClaims {
            sub: sub.to_string(),
            permissions,
            token_type: token_type.to_string(),
            iss: self.inner.issuer.clone(),
            iat: now,
            exp,
            jti: Uuid::new_v4().to_string(),
            family_id,
            parent_jti,
        };

        encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &self.inner.encoding_key,
        )
        .map_err(|e| format!("token encode failed: {e}"))
    }

    fn decode_token(&self, token: &str, expected_type: &str) -> Result<TokenClaims, String> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.inner.issuer.as_str()]);
        let decoded = decode::<TokenClaims>(token, &self.inner.decoding_key, &validation)
            .map_err(|e| format!("token decode failed: {e}"))?;
        if decoded.claims.token_type != expected_type {
            return Err(format!("invalid token type: expected {expected_type}"));
        }
        Ok(decoded.claims)
    }

    pub fn issue_access_token(
        &self,
        sub: &str,
        permissions: Vec<String>,
        ttl_secs: i64,
    ) -> Result<String, String> {
        self.issue_token(sub, permissions, "access", ttl_secs, None, None)
    }

    pub fn issue_refresh_token(
        &self,
        sub: &str,
        permissions: Vec<String>,
        ttl_secs: i64,
        family_id: String,
        parent_jti: Option<String>,
    ) -> Result<String, String> {
        self.issue_token(
            sub,
            permissions,
            "refresh",
            ttl_secs,
            Some(family_id),
            parent_jti,
        )
    }

    pub fn decode_access_token(&self, token: &str) -> Result<TokenClaims, String> {
        self.decode_token(token, "access")
    }

    pub fn decode_refresh_token(&self, token: &str) -> Result<TokenClaims, String> {
        self.decode_token(token, "refresh")
    }
}
