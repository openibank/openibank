-- =============================================================================
-- OpeniBank Database Initialization
-- =============================================================================
-- This script runs on first database creation.
-- It sets up extensions and basic schema structure.
-- =============================================================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "citext";

-- Create schemas for organization
CREATE SCHEMA IF NOT EXISTS auth;
CREATE SCHEMA IF NOT EXISTS trading;
CREATE SCHEMA IF NOT EXISTS wallet;
CREATE SCHEMA IF NOT EXISTS audit;

-- Grant usage on schemas
GRANT USAGE ON SCHEMA auth TO openibank;
GRANT USAGE ON SCHEMA trading TO openibank;
GRANT USAGE ON SCHEMA wallet TO openibank;
GRANT USAGE ON SCHEMA audit TO openibank;

-- =============================================================================
-- Auth Schema - Users and Authentication
-- =============================================================================

CREATE TABLE IF NOT EXISTS auth.users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email CITEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    two_factor_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    two_factor_secret TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_users_email ON auth.users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON auth.users(role);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON auth.users(created_at);

CREATE TABLE IF NOT EXISTS auth.api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL,
    permissions JSONB NOT NULL DEFAULT '[]',
    ip_whitelist TEXT[] DEFAULT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user ON auth.api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON auth.api_keys(key_hash);

CREATE TABLE IF NOT EXISTS auth.sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    device_info JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON auth.sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON auth.sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON auth.sessions(expires_at);

-- =============================================================================
-- Wallet Schema - Balances and Transactions
-- =============================================================================

CREATE TABLE IF NOT EXISTS wallet.balances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    free DECIMAL(36, 18) NOT NULL DEFAULT 0,
    locked DECIMAL(36, 18) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, asset)
);

CREATE INDEX IF NOT EXISTS idx_balances_user ON wallet.balances(user_id);
CREATE INDEX IF NOT EXISTS idx_balances_asset ON wallet.balances(asset);

CREATE TABLE IF NOT EXISTS wallet.deposits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    network TEXT NOT NULL,
    address TEXT NOT NULL,
    tx_hash TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    confirmations INTEGER NOT NULL DEFAULT 0,
    required_confirmations INTEGER NOT NULL DEFAULT 6,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_deposits_user ON wallet.deposits(user_id);
CREATE INDEX IF NOT EXISTS idx_deposits_status ON wallet.deposits(status);
CREATE INDEX IF NOT EXISTS idx_deposits_tx_hash ON wallet.deposits(tx_hash);

CREATE TABLE IF NOT EXISTS wallet.withdrawals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    fee DECIMAL(36, 18) NOT NULL DEFAULT 0,
    network TEXT NOT NULL,
    address TEXT NOT NULL,
    tx_hash TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_withdrawals_user ON wallet.withdrawals(user_id);
CREATE INDEX IF NOT EXISTS idx_withdrawals_status ON wallet.withdrawals(status);

-- =============================================================================
-- Trading Schema - Orders and Trades
-- =============================================================================

CREATE TABLE IF NOT EXISTS trading.orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    client_order_id TEXT,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL,
    time_in_force TEXT NOT NULL DEFAULT 'GTC',
    quantity DECIMAL(36, 18) NOT NULL,
    price DECIMAL(36, 18),
    stop_price DECIMAL(36, 18),
    executed_qty DECIMAL(36, 18) NOT NULL DEFAULT 0,
    cumulative_quote_qty DECIMAL(36, 18) NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'NEW',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_orders_user ON trading.orders(user_id);
CREATE INDEX IF NOT EXISTS idx_orders_symbol ON trading.orders(symbol);
CREATE INDEX IF NOT EXISTS idx_orders_status ON trading.orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_created ON trading.orders(created_at);
CREATE INDEX IF NOT EXISTS idx_orders_client_id ON trading.orders(user_id, client_order_id);

CREATE TABLE IF NOT EXISTS trading.trades (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL REFERENCES trading.orders(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    price DECIMAL(36, 18) NOT NULL,
    quantity DECIMAL(36, 18) NOT NULL,
    quote_quantity DECIMAL(36, 18) NOT NULL,
    commission DECIMAL(36, 18) NOT NULL DEFAULT 0,
    commission_asset TEXT,
    is_maker BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_trades_user ON trading.trades(user_id);
CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trading.trades(symbol);
CREATE INDEX IF NOT EXISTS idx_trades_order ON trading.trades(order_id);
CREATE INDEX IF NOT EXISTS idx_trades_created ON trading.trades(created_at);

-- =============================================================================
-- Audit Schema - Security Logs
-- =============================================================================

CREATE TABLE IF NOT EXISTS audit.security_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES auth.users(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    ip_address INET,
    user_agent TEXT,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_security_events_user ON audit.security_events(user_id);
CREATE INDEX IF NOT EXISTS idx_security_events_type ON audit.security_events(event_type);
CREATE INDEX IF NOT EXISTS idx_security_events_created ON audit.security_events(created_at);

-- =============================================================================
-- Functions and Triggers
-- =============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger to users table
DROP TRIGGER IF EXISTS users_updated_at ON auth.users;
CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON auth.users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Apply trigger to balances table
DROP TRIGGER IF EXISTS balances_updated_at ON wallet.balances;
CREATE TRIGGER balances_updated_at
    BEFORE UPDATE ON wallet.balances
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Apply trigger to orders table
DROP TRIGGER IF EXISTS orders_updated_at ON trading.orders;
CREATE TRIGGER orders_updated_at
    BEFORE UPDATE ON trading.orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- =============================================================================
-- Initial Data (Development Only)
-- =============================================================================

-- Create a test admin user (password: "admin123456")
-- NOTE: In production, create users through the API
INSERT INTO auth.users (id, email, password_hash, role, is_verified, is_active)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'admin@openibank.io',
    '$argon2id$v=19$m=65536,t=3,p=4$YWRtaW5zYWx0$7kFhQXRbYWRtaW5oYXNo', -- placeholder hash
    'admin',
    TRUE,
    TRUE
) ON CONFLICT (email) DO NOTHING;

-- Grant table permissions
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA auth TO openibank;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA trading TO openibank;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA wallet TO openibank;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA audit TO openibank;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA auth TO openibank;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA trading TO openibank;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA wallet TO openibank;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA audit TO openibank;

-- Log initialization complete
DO $$
BEGIN
    RAISE NOTICE 'OpeniBank database initialization complete';
END $$;
