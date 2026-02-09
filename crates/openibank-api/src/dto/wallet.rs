//! Wallet and deposit/withdrawal DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

// =============================================================================
// Deposit
// =============================================================================

/// Get deposit address request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetDepositAddressRequest {
    /// Asset/currency (e.g., "BTC", "ETH")
    pub coin: String,
    /// Network (e.g., "BTC", "ETH", "BSC")
    #[serde(default)]
    pub network: Option<String>,
}

/// Deposit address response (Binance-compatible)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DepositAddressResponse {
    /// Deposit address
    pub address: String,
    /// Asset
    pub coin: String,
    /// Tag/memo (for coins that require it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Network
    pub network: String,
    /// URL for QR code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Deposit history query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DepositHistoryQuery {
    /// Filter by asset
    #[serde(default)]
    pub coin: Option<String>,
    /// Filter by status (0: pending, 1: success, 6: credited)
    #[serde(default)]
    pub status: Option<i32>,
    /// Start time (Unix timestamp ms)
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time (Unix timestamp ms)
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Offset
    #[serde(default)]
    pub offset: Option<i64>,
    /// Limit (default 1000, max 1000)
    #[serde(default)]
    pub limit: Option<i64>,
}

/// Deposit record (Binance-compatible)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DepositRecord {
    /// Deposit ID
    pub id: Uuid,
    /// Amount
    pub amount: String,
    /// Asset
    pub coin: String,
    /// Network
    pub network: String,
    /// Status (0: pending, 1: success, 6: credited)
    pub status: i32,
    /// Deposit address
    pub address: String,
    /// Transaction hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
    /// Insert time
    pub insert_time: i64,
    /// Confirmation count
    pub confirmations: i32,
    /// Required confirmations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_times: Option<String>,
    /// Unlock confirmation count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlock_confirm: Option<i32>,
}

// =============================================================================
// Withdrawal
// =============================================================================

/// Withdrawal request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalRequest {
    /// Asset to withdraw
    pub coin: String,
    /// Withdrawal address
    #[validate(length(min = 10, message = "Invalid address"))]
    pub address: String,
    /// Amount to withdraw
    pub amount: String,
    /// Network (e.g., "ETH", "BSC", "TRC20")
    #[serde(default)]
    pub network: Option<String>,
    /// Address tag/memo (for coins that require it)
    #[serde(default)]
    pub address_tag: Option<String>,
    /// Withdrawal order ID (client-provided)
    #[serde(default)]
    pub withdraw_order_id: Option<String>,
    /// 2FA code
    #[serde(default)]
    pub two_factor_code: Option<String>,
}

/// Withdrawal response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalResponse {
    /// Withdrawal ID
    pub id: Uuid,
}

/// Withdrawal history query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalHistoryQuery {
    /// Filter by asset
    #[serde(default)]
    pub coin: Option<String>,
    /// Filter by withdrawal order ID
    #[serde(default)]
    pub withdraw_order_id: Option<String>,
    /// Filter by status
    #[serde(default)]
    pub status: Option<i32>,
    /// Start time
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Offset
    #[serde(default)]
    pub offset: Option<i64>,
    /// Limit
    #[serde(default)]
    pub limit: Option<i64>,
}

/// Withdrawal record (Binance-compatible)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalRecord {
    /// Withdrawal ID
    pub id: Uuid,
    /// Amount
    pub amount: String,
    /// Transaction fee
    pub transaction_fee: String,
    /// Asset
    pub coin: String,
    /// Status (0: email sent, 1: cancelled, 2: awaiting approval, 3: rejected, 4: processing, 5: failure, 6: completed)
    pub status: i32,
    /// Destination address
    pub address: String,
    /// Transaction hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
    /// Apply time
    pub apply_time: i64,
    /// Network
    pub network: String,
    /// Transfer type (0: external, 1: internal)
    #[serde(default)]
    pub transfer_type: i32,
    /// Withdraw order ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdraw_order_id: Option<String>,
    /// Info (failure reason etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    /// Confirmation count
    #[serde(default)]
    pub confirm_no: i32,
}

// =============================================================================
// Asset/Coin Info
// =============================================================================

/// Asset information
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    /// Asset symbol
    pub coin: String,
    /// Asset name
    pub name: String,
    /// Is deposit enabled
    pub deposit_all_enable: bool,
    /// Is withdrawal enabled
    pub withdraw_all_enable: bool,
    /// Free balance
    pub free: String,
    /// Locked balance
    pub locked: String,
    /// Freeze balance
    pub freeze: String,
    /// Withdrawing balance
    pub withdrawing: String,
    /// Is legal money
    pub is_legal_money: bool,
    /// Is trading
    pub trading: bool,
    /// Networks
    pub network_list: Vec<NetworkInfo>,
}

/// Network information
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    /// Network name
    pub network: String,
    /// Asset on this network
    pub coin: String,
    /// Withdrawal fee
    pub withdraw_fee: String,
    /// Minimum withdrawal
    pub withdraw_min: String,
    /// Maximum withdrawal
    pub withdraw_max: String,
    /// Withdrawal enabled
    pub withdraw_enable: bool,
    /// Deposit enabled
    pub deposit_enable: bool,
    /// Deposit description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_desc: Option<String>,
    /// Withdrawal description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdraw_desc: Option<String>,
    /// Special tips
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_tips: Option<String>,
    /// Confirmation count for deposit
    pub min_confirm: i32,
    /// Unlock confirmation count
    pub un_lock_confirm: i32,
    /// Is default network
    pub is_default: bool,
    /// Estimated arrival time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimate_arrival_time: Option<i32>,
    /// Address regex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_regex: Option<String>,
    /// Memo regex
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo_regex: Option<String>,
}

// =============================================================================
// Internal Transfer
// =============================================================================

/// Internal transfer request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InternalTransferRequest {
    /// Target account type
    pub target_type: String,
    /// Target account ID (email or user ID)
    pub target: String,
    /// Asset
    pub coin: String,
    /// Amount
    pub amount: String,
}

/// Internal transfer response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InternalTransferResponse {
    /// Transfer ID
    pub tran_id: Uuid,
}
