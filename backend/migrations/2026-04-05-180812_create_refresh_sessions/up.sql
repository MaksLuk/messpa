CREATE TABLE refresh_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    family_id UUID NOT NULL,
    token_hash TEXT NOT NULL,
    device_fingerprint TEXT NOT NULL,
    ip_address INET NOT NULL,
    user_agent TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN DEFAULT false NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(family_id)
);

CREATE INDEX idx_refresh_family ON refresh_sessions(family_id);
CREATE INDEX idx_refresh_user ON refresh_sessions(user_id);
CREATE INDEX idx_refresh_expires ON refresh_sessions(expires_at);
