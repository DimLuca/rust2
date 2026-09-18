CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE user_role AS ENUM ('ADMIN', 'TECHNICIAN');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    display_name TEXT NOT NULL,
    role user_role NOT NULL DEFAULT 'TECHNICIAN',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    token_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    CONSTRAINT users_username_format CHECK (username ~ '^[a-z0-9._-]{3,64}$')
);

CREATE UNIQUE INDEX users_username_lower_unique ON users (LOWER(username));

CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_version INTEGER NOT NULL,
    source_ip INET NOT NULL,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    revoke_reason TEXT
);

CREATE INDEX auth_sessions_user_active_idx
    ON auth_sessions (user_id, expires_at)
    WHERE revoked_at IS NULL;

CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    support_session_id UUID,
    auth_session_id UUID,
    technician_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    technician_username TEXT,
    technician_display_name TEXT,
    technician_role TEXT,
    technician_device_id TEXT,
    technician_hostname TEXT,
    technician_local_ip INET,
    technician_source_ip INET,
    client_rustdesk_id TEXT,
    client_device_id TEXT,
    client_hostname TEXT,
    client_os_username TEXT,
    client_local_ip INET,
    result TEXT,
    duration_seconds BIGINT CHECK (duration_seconds IS NULL OR duration_seconds >= 0),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX audit_events_occurred_at_idx ON audit_events (occurred_at DESC);
CREATE INDEX audit_events_support_session_idx ON audit_events (support_session_id, occurred_at);
CREATE INDEX audit_events_technician_idx ON audit_events (technician_user_id, occurred_at DESC);

CREATE FUNCTION prevent_audit_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'audit events are append-only';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_events_no_update
    BEFORE UPDATE OR DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_mutation();

