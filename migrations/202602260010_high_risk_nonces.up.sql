CREATE TABLE IF NOT EXISTS high_risk_nonces (
    nonce TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_high_risk_nonces_expires_at ON high_risk_nonces(expires_at);
