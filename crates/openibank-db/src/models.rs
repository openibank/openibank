//! Database models - mapped from PostgreSQL tables

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// User Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbUser {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: String,
    pub username: Option<String>,
    pub phone: Option<String>,
    pub phone_verified: bool,
    pub kyc_tier: i16,
    pub status: String,
    pub referral_code: Option<String>,
    pub referred_by: Option<Uuid>,
    pub anti_phishing_code: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub secret_hash: String,
    pub label: Option<String>,
    pub permissions: serde_json::Value,
    pub ip_whitelist: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Wallet Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub wallet_type: String,
    pub agent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbBalance {
    pub wallet_id: Uuid,
    pub currency: String,
    pub available: Decimal,
    pub locked: Decimal,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbBalanceChange {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub currency: String,
    pub change_type: String,
    pub amount: Decimal,
    pub balance_before: Decimal,
    pub balance_after: Decimal,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub receipt_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Deposit/Withdrawal Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbDepositAddress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: String,
    pub network: String,
    pub address: String,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbDeposit {
    pub id: Uuid,
    pub user_id: Uuid,
    pub wallet_id: Uuid,
    pub currency: String,
    pub network: String,
    pub amount: Decimal,
    pub tx_hash: Option<String>,
    pub from_address: Option<String>,
    pub confirmations: i32,
    pub required_confirmations: i32,
    pub status: String,
    pub credited_at: Option<DateTime<Utc>>,
    pub receipt_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbWithdrawal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub wallet_id: Uuid,
    pub currency: String,
    pub network: String,
    pub amount: Decimal,
    pub fee: Decimal,
    pub to_address: String,
    pub memo: Option<String>,
    pub tx_hash: Option<String>,
    pub status: String,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub receipt_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Trading Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbMarket {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub status: String,
    pub price_precision: i16,
    pub amount_precision: i16,
    pub min_amount: Decimal,
    pub max_amount: Option<Decimal>,
    pub min_notional: Option<Decimal>,
    pub tick_size: Decimal,
    pub lot_size: Decimal,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: String,
    pub client_order_id: Option<String>,
    pub side: String,
    pub order_type: String,
    pub price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trailing_delta: Option<Decimal>,
    pub amount: Decimal,
    pub filled: Decimal,
    pub remaining: Decimal,
    pub quote_filled: Decimal,
    pub fee_total: Decimal,
    pub fee_currency: Option<String>,
    pub time_in_force: String,
    pub expire_at: Option<DateTime<Utc>>,
    pub post_only: bool,
    pub reduce_only: bool,
    pub iceberg_qty: Option<Decimal>,
    pub status: String,
    pub reject_reason: Option<String>,
    pub permit_id: Option<Uuid>,
    pub commitment_id: Option<Uuid>,
    pub receipt_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTrade {
    pub id: Uuid,
    pub market_id: String,
    pub price: Decimal,
    pub amount: Decimal,
    pub quote_amount: Decimal,
    pub maker_order_id: Uuid,
    pub taker_order_id: Uuid,
    pub maker_user_id: Uuid,
    pub taker_user_id: Uuid,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub maker_fee_currency: String,
    pub taker_fee_currency: String,
    pub is_buyer_maker: bool,
    pub maker_receipt_id: Option<Uuid>,
    pub taker_receipt_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbCandle {
    pub market_id: String,
    pub interval: String,
    pub bucket: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
    pub trade_count: i64,
}

// ============================================================================
// Receipt & Audit Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbReceipt {
    pub id: Uuid,
    pub receipt_type: String,
    pub commitment_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub payload_hash: Vec<u8>,
    pub signature: Vec<u8>,
    pub signer_public_key: Vec<u8>,
    pub chain_proof: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbAuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub receipt_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Arena Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbArenaCompetition {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub competition_type: String,
    pub status: String,
    pub markets: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub registration_end: Option<DateTime<Utc>>,
    pub initial_balance: Decimal,
    pub entry_fee: Decimal,
    pub prize_pool: Decimal,
    pub max_participants: Option<i32>,
    pub scoring_config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbArenaParticipant {
    pub id: Uuid,
    pub competition_id: Uuid,
    pub user_id: Uuid,
    pub wallet_id: Uuid,
    pub status: String,
    pub entry_balance: Decimal,
    pub current_balance: Decimal,
    pub pnl: Decimal,
    pub pnl_percent: Decimal,
    pub trade_count: i32,
    pub win_rate: Decimal,
    pub sharpe_ratio: Option<Decimal>,
    pub max_drawdown: Decimal,
    pub final_rank: Option<i32>,
    pub prize_amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
