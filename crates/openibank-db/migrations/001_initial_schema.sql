-- OpeniBank Initial Schema
-- PostgreSQL 16+ with TimescaleDB extension

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- USERS & AUTHENTICATION
-- ============================================================================

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    password_hash TEXT NOT NULL,
    username TEXT UNIQUE,
    phone TEXT,
    phone_verified BOOLEAN DEFAULT FALSE,
    kyc_tier SMALLINT DEFAULT 0 CHECK (kyc_tier >= 0 AND kyc_tier <= 3),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'banned', 'pending')),
    referral_code TEXT UNIQUE,
    referred_by UUID REFERENCES users(id),
    anti_phishing_code TEXT,
    locale TEXT DEFAULT 'en',
    timezone TEXT DEFAULT 'UTC',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_username ON users(username) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_referral_code ON users(referral_code) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_referred_by ON users(referred_by);

-- API Keys for programmatic access
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    label TEXT,
    permissions JSONB DEFAULT '{"read": true, "trade": false, "withdraw": false}',
    ip_whitelist TEXT[],
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_user ON api_keys(user_id) WHERE revoked_at IS NULL;
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash) WHERE revoked_at IS NULL;

-- User sessions for web/mobile
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    ip_address INET,
    user_agent TEXT,
    device_name TEXT,
    device_type TEXT CHECK (device_type IN ('web', 'ios', 'android', 'api')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_sessions_user ON user_sessions(user_id);
CREATE INDEX idx_sessions_token ON user_sessions(token_hash);
CREATE INDEX idx_sessions_expires ON user_sessions(expires_at);

-- Two-factor authentication
CREATE TABLE user_2fa (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    method TEXT NOT NULL CHECK (method IN ('totp', 'sms', 'email', 'yubikey')),
    secret_encrypted BYTEA,
    backup_codes_hash TEXT[],
    enabled_at TIMESTAMPTZ DEFAULT NOW()
);

-- Login attempts for rate limiting
CREATE TABLE login_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL,
    ip_address INET NOT NULL,
    success BOOLEAN NOT NULL,
    failure_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_login_attempts_email ON login_attempts(email, created_at DESC);
CREATE INDEX idx_login_attempts_ip ON login_attempts(ip_address, created_at DESC);

-- ============================================================================
-- WALLETS & BALANCES
-- ============================================================================

CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    wallet_type TEXT DEFAULT 'spot' CHECK (wallet_type IN ('spot', 'futures', 'earn', 'funding', 'agent')),
    agent_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, wallet_type, agent_id)
);

CREATE INDEX idx_wallets_user ON wallets(user_id);
CREATE INDEX idx_wallets_agent ON wallets(agent_id) WHERE agent_id IS NOT NULL;

CREATE TABLE balances (
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE RESTRICT,
    currency TEXT NOT NULL,
    available NUMERIC(36,18) DEFAULT 0 CHECK (available >= 0),
    locked NUMERIC(36,18) DEFAULT 0 CHECK (locked >= 0),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (wallet_id, currency)
);

CREATE INDEX idx_balances_currency ON balances(currency);

-- Balance change ledger (immutable audit trail)
CREATE TABLE balance_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES wallets(id),
    currency TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK (change_type IN (
        'deposit', 'withdraw', 'trade', 'fee', 'transfer', 
        'stake', 'unstake', 'reward', 'liquidation', 'adjustment'
    )),
    amount NUMERIC(36,18) NOT NULL,
    balance_before NUMERIC(36,18) NOT NULL,
    balance_after NUMERIC(36,18) NOT NULL,
    reference_type TEXT,
    reference_id UUID,
    receipt_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_balance_changes_wallet ON balance_changes(wallet_id, created_at DESC);
CREATE INDEX idx_balance_changes_reference ON balance_changes(reference_type, reference_id);

-- ============================================================================
-- DEPOSITS & WITHDRAWALS
-- ============================================================================

CREATE TABLE deposit_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    currency TEXT NOT NULL,
    network TEXT NOT NULL,
    address TEXT NOT NULL,
    memo TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(currency, network, address)
);

CREATE INDEX idx_deposit_addresses_user ON deposit_addresses(user_id);
CREATE INDEX idx_deposit_addresses_address ON deposit_addresses(address);

CREATE TABLE deposits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    wallet_id UUID NOT NULL REFERENCES wallets(id),
    currency TEXT NOT NULL,
    network TEXT NOT NULL,
    amount NUMERIC(36,18) NOT NULL CHECK (amount > 0),
    tx_hash TEXT,
    from_address TEXT,
    confirmations INTEGER DEFAULT 0,
    required_confirmations INTEGER NOT NULL,
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'confirming', 'completed', 'failed')),
    credited_at TIMESTAMPTZ,
    receipt_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_deposits_user ON deposits(user_id, created_at DESC);
CREATE INDEX idx_deposits_status ON deposits(status) WHERE status != 'completed';
CREATE INDEX idx_deposits_tx ON deposits(tx_hash) WHERE tx_hash IS NOT NULL;

CREATE TABLE withdrawals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    wallet_id UUID NOT NULL REFERENCES wallets(id),
    currency TEXT NOT NULL,
    network TEXT NOT NULL,
    amount NUMERIC(36,18) NOT NULL CHECK (amount > 0),
    fee NUMERIC(36,18) NOT NULL CHECK (fee >= 0),
    to_address TEXT NOT NULL,
    memo TEXT,
    tx_hash TEXT,
    status TEXT DEFAULT 'pending' CHECK (status IN (
        'pending', 'awaiting_approval', 'processing', 'completed', 'failed', 'cancelled'
    )),
    approved_by UUID REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    failure_reason TEXT,
    receipt_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_withdrawals_user ON withdrawals(user_id, created_at DESC);
CREATE INDEX idx_withdrawals_status ON withdrawals(status) WHERE status NOT IN ('completed', 'cancelled', 'failed');

-- Withdrawal address whitelist
CREATE TABLE withdrawal_whitelist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    currency TEXT NOT NULL,
    network TEXT NOT NULL,
    address TEXT NOT NULL,
    label TEXT,
    activated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    UNIQUE(user_id, currency, network, address)
);

CREATE INDEX idx_whitelist_user ON withdrawal_whitelist(user_id) WHERE deleted_at IS NULL;

-- ============================================================================
-- TRADING (ResonanceX)
-- ============================================================================

CREATE TABLE rx_markets (
    id TEXT PRIMARY KEY,
    base_currency TEXT NOT NULL,
    quote_currency TEXT NOT NULL,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'halt', 'delisted', 'pre_trading')),
    price_precision SMALLINT NOT NULL CHECK (price_precision >= 0),
    amount_precision SMALLINT NOT NULL CHECK (amount_precision >= 0),
    min_amount NUMERIC(36,18) NOT NULL CHECK (min_amount > 0),
    max_amount NUMERIC(36,18),
    min_notional NUMERIC(36,18) DEFAULT 10,
    tick_size NUMERIC(36,18) NOT NULL CHECK (tick_size > 0),
    lot_size NUMERIC(36,18) NOT NULL CHECK (lot_size > 0),
    maker_fee NUMERIC(10,8) NOT NULL CHECK (maker_fee >= -0.01 AND maker_fee <= 0.1),
    taker_fee NUMERIC(10,8) NOT NULL CHECK (taker_fee >= 0 AND taker_fee <= 0.1),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE rx_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    market_id TEXT NOT NULL REFERENCES rx_markets(id),
    client_order_id TEXT,
    side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type TEXT NOT NULL CHECK (order_type IN (
        'limit', 'market', 'stop_limit', 'stop_market', 
        'take_profit', 'take_profit_limit', 'trailing_stop'
    )),
    price NUMERIC(36,18),
    stop_price NUMERIC(36,18),
    trailing_delta NUMERIC(36,18),
    amount NUMERIC(36,18) NOT NULL CHECK (amount > 0),
    filled NUMERIC(36,18) DEFAULT 0 CHECK (filled >= 0),
    remaining NUMERIC(36,18) NOT NULL CHECK (remaining >= 0),
    quote_filled NUMERIC(36,18) DEFAULT 0 CHECK (quote_filled >= 0),
    fee_total NUMERIC(36,18) DEFAULT 0,
    fee_currency TEXT,
    time_in_force TEXT DEFAULT 'GTC' CHECK (time_in_force IN ('GTC', 'IOC', 'FOK', 'GTT')),
    expire_at TIMESTAMPTZ,
    post_only BOOLEAN DEFAULT FALSE,
    reduce_only BOOLEAN DEFAULT FALSE,
    iceberg_qty NUMERIC(36,18),
    status TEXT DEFAULT 'new' CHECK (status IN (
        'new', 'partially_filled', 'filled', 'cancelled', 
        'pending_cancel', 'rejected', 'expired'
    )),
    reject_reason TEXT,
    permit_id UUID,
    commitment_id UUID,
    receipt_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_rx_orders_user ON rx_orders(user_id, created_at DESC);
CREATE INDEX idx_rx_orders_user_status ON rx_orders(user_id, status) WHERE status IN ('new', 'partially_filled');
CREATE INDEX idx_rx_orders_market ON rx_orders(market_id, status) WHERE status IN ('new', 'partially_filled');
CREATE INDEX idx_rx_orders_market_price ON rx_orders(market_id, side, price) WHERE status IN ('new', 'partially_filled');
CREATE INDEX idx_rx_orders_client ON rx_orders(user_id, client_order_id);

CREATE TABLE rx_trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    market_id TEXT NOT NULL REFERENCES rx_markets(id),
    price NUMERIC(36,18) NOT NULL,
    amount NUMERIC(36,18) NOT NULL,
    quote_amount NUMERIC(36,18) NOT NULL,
    maker_order_id UUID NOT NULL REFERENCES rx_orders(id),
    taker_order_id UUID NOT NULL REFERENCES rx_orders(id),
    maker_user_id UUID NOT NULL REFERENCES users(id),
    taker_user_id UUID NOT NULL REFERENCES users(id),
    maker_fee NUMERIC(36,18) NOT NULL,
    taker_fee NUMERIC(36,18) NOT NULL,
    maker_fee_currency TEXT NOT NULL,
    taker_fee_currency TEXT NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL,
    maker_receipt_id UUID,
    taker_receipt_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_rx_trades_market ON rx_trades(market_id, created_at DESC);
CREATE INDEX idx_rx_trades_maker ON rx_trades(maker_user_id, created_at DESC);
CREATE INDEX idx_rx_trades_taker ON rx_trades(taker_user_id, created_at DESC);
CREATE INDEX idx_rx_trades_time ON rx_trades(created_at DESC);

-- OHLCV Candles (standard table, TimescaleDB hypertable created separately)
CREATE TABLE rx_candles (
    market_id TEXT NOT NULL,
    interval TEXT NOT NULL CHECK (interval IN (
        '1s', '1m', '3m', '5m', '15m', '30m', 
        '1h', '2h', '4h', '6h', '8h', '12h', 
        '1d', '3d', '1w', '1M'
    )),
    bucket TIMESTAMPTZ NOT NULL,
    open NUMERIC(36,18) NOT NULL,
    high NUMERIC(36,18) NOT NULL,
    low NUMERIC(36,18) NOT NULL,
    close NUMERIC(36,18) NOT NULL,
    volume NUMERIC(36,18) NOT NULL,
    quote_volume NUMERIC(36,18) NOT NULL,
    trade_count BIGINT NOT NULL,
    PRIMARY KEY (market_id, interval, bucket)
);

CREATE INDEX idx_rx_candles_lookup ON rx_candles(market_id, interval, bucket DESC);

-- ============================================================================
-- FEE TIERS
-- ============================================================================

CREATE TABLE fee_tiers (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    min_volume NUMERIC(36,18) NOT NULL,
    maker_fee NUMERIC(10,8) NOT NULL,
    taker_fee NUMERIC(10,8) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

INSERT INTO fee_tiers (name, min_volume, maker_fee, taker_fee) VALUES
    ('Standard', 0, 0.0008, 0.0010),
    ('Bronze', 50000, 0.0006, 0.0008),
    ('Silver', 500000, 0.0004, 0.0006),
    ('Gold', 5000000, 0.0002, 0.0004),
    ('Diamond', 50000000, 0.0000, 0.0002),
    ('VIP', 100000000, 0.0000, 0.0001),
    ('MarketMaker', 0, -0.0001, 0.0002);

CREATE TABLE user_fee_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    tier_id INTEGER NOT NULL REFERENCES fee_tiers(id),
    volume_30d NUMERIC(36,18) DEFAULT 0,
    obk_staked NUMERIC(36,18) DEFAULT 0,
    custom_maker_fee NUMERIC(10,8),
    custom_taker_fee NUMERIC(10,8),
    is_market_maker BOOLEAN DEFAULT FALSE,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- REFERRALS
-- ============================================================================

CREATE TABLE referrals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_id UUID NOT NULL REFERENCES users(id),
    referee_id UUID NOT NULL REFERENCES users(id) UNIQUE,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'suspended')),
    commission_rate NUMERIC(5,4) DEFAULT 0.20,
    total_commission NUMERIC(36,18) DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_referrals_referrer ON referrals(referrer_id);

CREATE TABLE referral_commissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referral_id UUID NOT NULL REFERENCES referrals(id),
    trade_id UUID NOT NULL REFERENCES rx_trades(id),
    fee_amount NUMERIC(36,18) NOT NULL,
    commission_amount NUMERIC(36,18) NOT NULL,
    currency TEXT NOT NULL,
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'paid')),
    paid_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_referral_commissions_referral ON referral_commissions(referral_id);

-- ============================================================================
-- RECEIPTS (Cryptographic Proofs)
-- ============================================================================

CREATE TABLE receipts (
    id UUID PRIMARY KEY,
    receipt_type TEXT NOT NULL CHECK (receipt_type IN (
        'trade', 'deposit', 'withdrawal', 'transfer', 
        'stake', 'unstake', 'liquidation', 'fee', 'reward'
    )),
    commitment_id UUID,
    user_id UUID REFERENCES users(id),
    payload JSONB NOT NULL,
    payload_hash BYTEA NOT NULL,
    signature BYTEA NOT NULL,
    signer_public_key BYTEA NOT NULL,
    chain_proof TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_receipts_user ON receipts(user_id, created_at DESC);
CREATE INDEX idx_receipts_type ON receipts(receipt_type, created_at DESC);
CREATE INDEX idx_receipts_commitment ON receipts(commitment_id) WHERE commitment_id IS NOT NULL;

-- ============================================================================
-- AUDIT LOG
-- ============================================================================

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    action TEXT NOT NULL,
    resource_type TEXT,
    resource_id TEXT,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    receipt_id UUID REFERENCES receipts(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_log_user ON audit_log(user_id, created_at DESC);
CREATE INDEX idx_audit_log_action ON audit_log(action, created_at DESC);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);

-- ============================================================================
-- ARENA (Trading Competitions)
-- ============================================================================

CREATE TABLE arena_competitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT,
    competition_type TEXT NOT NULL CHECK (competition_type IN (
        'pnl', 'sharpe', 'volume', 'drawdown', 'speed', 'custom'
    )),
    status TEXT DEFAULT 'scheduled' CHECK (status IN (
        'scheduled', 'registration', 'active', 'calculating', 'completed', 'cancelled'
    )),
    markets TEXT[] NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    registration_end TIMESTAMPTZ,
    initial_balance NUMERIC(36,18) NOT NULL,
    entry_fee NUMERIC(36,18) DEFAULT 0,
    prize_pool NUMERIC(36,18) DEFAULT 0,
    max_participants INTEGER,
    scoring_config JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_arena_status ON arena_competitions(status, start_time);

CREATE TABLE arena_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    competition_id UUID NOT NULL REFERENCES arena_competitions(id),
    user_id UUID NOT NULL REFERENCES users(id),
    wallet_id UUID NOT NULL REFERENCES wallets(id),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'disqualified', 'withdrawn')),
    entry_balance NUMERIC(36,18) NOT NULL,
    current_balance NUMERIC(36,18) NOT NULL,
    pnl NUMERIC(36,18) DEFAULT 0,
    pnl_percent NUMERIC(10,4) DEFAULT 0,
    trade_count INTEGER DEFAULT 0,
    win_rate NUMERIC(5,4) DEFAULT 0,
    sharpe_ratio NUMERIC(10,4),
    max_drawdown NUMERIC(10,4) DEFAULT 0,
    final_rank INTEGER,
    prize_amount NUMERIC(36,18) DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(competition_id, user_id)
);

CREATE INDEX idx_arena_participants_competition ON arena_participants(competition_id, pnl DESC);
CREATE INDEX idx_arena_participants_user ON arena_participants(user_id);

-- ============================================================================
-- TRIGGERS FOR updated_at
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER rx_markets_updated_at BEFORE UPDATE ON rx_markets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER rx_orders_updated_at BEFORE UPDATE ON rx_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER user_fee_profiles_updated_at BEFORE UPDATE ON user_fee_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER arena_participants_updated_at BEFORE UPDATE ON arena_participants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
