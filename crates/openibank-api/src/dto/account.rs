//! Account-related DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use rust_decimal::Decimal;

/// Account information (Binance-compatible)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    /// Maker commission rate (basis points)
    pub maker_commission: i32,
    /// Taker commission rate (basis points)
    pub taker_commission: i32,
    /// Buyer commission rate
    pub buyer_commission: i32,
    /// Seller commission rate
    pub seller_commission: i32,
    /// Commission rates (detailed)
    pub commission_rates: CommissionRates,
    /// Can trade
    pub can_trade: bool,
    /// Can withdraw
    pub can_withdraw: bool,
    /// Can deposit
    pub can_deposit: bool,
    /// Is brokered account
    pub brokered: bool,
    /// Require self trade prevention
    pub require_self_trade_prevention: bool,
    /// Prevent SOR
    pub prevent_sor: bool,
    /// Account update time
    pub update_time: i64,
    /// Account type
    pub account_type: String,
    /// Balances
    pub balances: Vec<BalanceInfo>,
    /// Permissions
    pub permissions: Vec<String>,
    /// User ID
    pub uid: i64,
}

/// Commission rates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommissionRates {
    /// Maker rate
    pub maker: String,
    /// Taker rate
    pub taker: String,
    /// Buyer rate
    pub buyer: String,
    /// Seller rate
    pub seller: String,
}

/// Balance information (Binance-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BalanceInfo {
    /// Asset symbol (e.g., "BTC")
    pub asset: String,
    /// Free (available) balance
    pub free: String,
    /// Locked balance
    pub locked: String,
}

impl BalanceInfo {
    pub fn new(asset: String, free: Decimal, locked: Decimal) -> Self {
        Self {
            asset,
            free: free.to_string(),
            locked: locked.to_string(),
        }
    }

    pub fn total(&self) -> Decimal {
        let free: Decimal = self.free.parse().unwrap_or_default();
        let locked: Decimal = self.locked.parse().unwrap_or_default();
        free + locked
    }
}

/// Account status response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountStatusResponse {
    /// Status data
    pub data: String,
}

/// Trading fee information
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeFeeInfo {
    /// Symbol
    pub symbol: String,
    /// Maker commission
    pub maker_commission: String,
    /// Taker commission
    pub taker_commission: String,
}

/// Update account request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    /// Settings to update
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
}

/// Account status (detailed)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    /// Account ID
    pub id: Uuid,
    /// Email
    pub email: String,
    /// Account status
    pub status: String,
    /// KYC level
    pub kyc_level: u8,
    /// Fee tier
    pub fee_tier: String,
    /// 2FA enabled
    pub two_factor_enabled: bool,
    /// Email verified
    pub email_verified: bool,
    /// Withdrawal enabled
    pub withdrawal_enabled: bool,
    /// Trading enabled
    pub trading_enabled: bool,
    /// Deposit enabled
    pub deposit_enabled: bool,
    /// Daily withdrawal limit
    pub daily_withdrawal_limit: String,
    /// Used withdrawal today
    pub daily_withdrawal_used: String,
}

/// Account activity/audit log entry
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLogEntry {
    /// Activity ID
    pub id: Uuid,
    /// Action type
    pub action: String,
    /// Description
    pub description: String,
    /// IP address
    pub ip_address: String,
    /// User agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Success indicator
    pub success: bool,
}

/// Referral info
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferralInfo {
    /// Referral code
    pub referral_code: String,
    /// Total referrals
    pub total_referrals: i64,
    /// Active referrals
    pub active_referrals: i64,
    /// Total commission earned
    pub total_commission_earned: String,
    /// Commission rate
    pub commission_rate: String,
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    /// Email notifications
    pub email_notifications: bool,
    /// Trade notifications
    pub trade_notifications: bool,
    /// Deposit notifications
    pub deposit_notifications: bool,
    /// Withdrawal notifications
    pub withdrawal_notifications: bool,
    /// Price alerts
    pub price_alerts: bool,
    /// Security alerts (always on)
    pub security_alerts: bool,
    /// Marketing emails
    pub marketing_emails: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            email_notifications: true,
            trade_notifications: true,
            deposit_notifications: true,
            withdrawal_notifications: true,
            price_alerts: false,
            security_alerts: true,
            marketing_emails: false,
        }
    }
}
