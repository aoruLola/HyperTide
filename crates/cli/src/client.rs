use reqwest::RequestBuilder;

use crate::models::{CliProfile, TokenPair};

pub fn with_auth(request: RequestBuilder, profile: &CliProfile) -> RequestBuilder {
    if profile.api_key_direct {
        return request.header("X-API-Key", &profile.api_key);
    }
    if let Some(access_token) = profile.access_token.as_deref() {
        return request.bearer_auth(access_token);
    }
    request.header("X-API-Key", &profile.api_key)
}

pub fn token_expired(profile: &CliProfile, now_unix: i64) -> bool {
    let Some(expires_at) = profile.access_token_expires_at else {
        return true;
    };
    now_unix >= expires_at.saturating_sub(30)
}

pub fn apply_token_pair(profile: &mut CliProfile, pair: TokenPair, now_unix: i64) {
    profile.access_token = Some(pair.access_token);
    profile.refresh_token = Some(pair.refresh_token);
    profile.access_token_expires_at = Some(now_unix + pair.expires_in.max(0));
}
