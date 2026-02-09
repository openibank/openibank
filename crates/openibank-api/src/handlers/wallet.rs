//! Wallet Handlers
//!
//! Endpoints for deposits, withdrawals, and wallet operations.
//! Fully integrated with database layer.

use axum::{
    extract::{Query, State},
    Json,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::dto::{
    GetDepositAddressRequest, DepositAddressResponse, DepositHistoryQuery, DepositRecord,
    WithdrawalRequest, WithdrawalResponse, WithdrawalHistoryQuery, WithdrawalRecord,
    AssetInfo, NetworkInfo, InternalTransferRequest, InternalTransferResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::extractors::{AuthenticatedUser, Require2FA};
use crate::state::AppState;

/// Convert database deposit status to Binance-compatible status code
fn deposit_status_to_code(status: &str) -> i32 {
    match status {
        "pending" | "confirming" => 0,
        "completed" => 1,
        "credited" => 6,
        "failed" => 5,
        _ => 0,
    }
}

/// Convert database withdrawal status to Binance-compatible status code
fn withdrawal_status_to_code(status: &str) -> i32 {
    match status {
        "pending" => 0,
        "cancelled" => 1,
        "awaiting_approval" => 2,
        "rejected" => 3,
        "processing" => 4,
        "failed" => 5,
        "completed" => 6,
        _ => 0,
    }
}

/// Get deposit address
#[utoipa::path(
    get,
    path = "/api/v1/capital/deposit/address",
    tag = "Wallet",
    params(
        ("coin" = String, Query, description = "Asset symbol"),
        ("network" = Option<String>, Query, description = "Network")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Deposit address", body = DepositAddressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid asset or network")
    )
)]
pub async fn get_deposit_address(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(request): Query<GetDepositAddressRequest>,
) -> ApiResult<Json<DepositAddressResponse>> {
    let network = request.network.clone().unwrap_or_else(|| request.coin.clone());

    // Try to find existing deposit address
    let existing = state.db.deposit_repo()
        .find_address(user.user_id, &request.coin, &network)
        .await
        .map_err(ApiError::from)?;

    if let Some(addr) = existing {
        return Ok(Json(DepositAddressResponse {
            address: addr.address,
            coin: addr.currency,
            tag: addr.memo,
            network: addr.network,
            url: None,
        }));
    }

    // Generate a new deposit address
    // In production, this would call a blockchain service to generate an address
    // For now, we generate a deterministic placeholder based on user_id and coin
    let generated_address = format!(
        "0x{}",
        hex::encode(&user.user_id.as_bytes()[..20])
    );

    // Create new deposit address record
    let new_address = openibank_db::DbDepositAddress {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        currency: request.coin.clone(),
        network: network.clone(),
        address: generated_address.clone(),
        memo: None,
        created_at: chrono::Utc::now(),
    };

    let saved_address = state.db.deposit_repo()
        .create_address(&new_address)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(DepositAddressResponse {
        address: saved_address.address,
        coin: saved_address.currency,
        tag: saved_address.memo,
        network: saved_address.network,
        url: None,
    }))
}

/// Get deposit history
#[utoipa::path(
    get,
    path = "/api/v1/capital/deposit/hisrec",
    tag = "Wallet",
    params(
        ("coin" = Option<String>, Query, description = "Filter by asset"),
        ("status" = Option<i32>, Query, description = "Filter by status"),
        ("startTime" = Option<i64>, Query, description = "Start time"),
        ("endTime" = Option<i64>, Query, description = "End time"),
        ("offset" = Option<i64>, Query, description = "Offset"),
        ("limit" = Option<i64>, Query, description = "Limit")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Deposit history", body = Vec<DepositRecord>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_deposit_history(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(query): Query<DepositHistoryQuery>,
) -> ApiResult<Json<Vec<DepositRecord>>> {
    let limit = query.limit.unwrap_or(1000).min(1000);
    let offset = query.offset.unwrap_or(0);

    // Get deposits from database
    let deposits = state.db.deposit_repo()
        .list_by_user(user.user_id, limit, offset)
        .await
        .map_err(ApiError::from)?;

    // Filter by coin if specified
    let filtered = deposits.into_iter()
        .filter(|d| {
            if let Some(ref coin) = query.coin {
                &d.currency == coin
            } else {
                true
            }
        })
        .filter(|d| {
            if let Some(status_code) = query.status {
                deposit_status_to_code(&d.status) == status_code
            } else {
                true
            }
        })
        .filter(|d| {
            if let Some(start_time) = query.start_time {
                d.created_at.timestamp_millis() >= start_time
            } else {
                true
            }
        })
        .filter(|d| {
            if let Some(end_time) = query.end_time {
                d.created_at.timestamp_millis() <= end_time
            } else {
                true
            }
        });

    // Get deposit addresses for the user to include address info
    let addresses = state.db.deposit_repo()
        .list_user_addresses(user.user_id)
        .await
        .map_err(ApiError::from)?;

    // Convert to DTOs
    let records: Vec<DepositRecord> = filtered
        .map(|d| {
            // Find address for this deposit
            let address = addresses.iter()
                .find(|a| a.currency == d.currency && a.network == d.network)
                .map(|a| a.address.clone())
                .unwrap_or_default();

            DepositRecord {
                id: d.id,
                amount: d.amount.to_string(),
                coin: d.currency,
                network: d.network,
                status: deposit_status_to_code(&d.status),
                address,
                tx_id: d.tx_hash,
                insert_time: d.created_at.timestamp_millis(),
                confirmations: d.confirmations,
                confirm_times: Some(format!("{}/{}", d.confirmations, d.required_confirmations)),
                unlock_confirm: Some(d.required_confirmations),
            }
        })
        .collect();

    Ok(Json(records))
}

/// Submit withdrawal request
#[utoipa::path(
    post,
    path = "/api/v1/capital/withdraw/apply",
    tag = "Wallet",
    request_body = WithdrawalRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Withdrawal submitted", body = WithdrawalResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "2FA required")
    )
)]
pub async fn submit_withdrawal(
    State(state): State<Arc<AppState>>,
    user: Require2FA,
    Json(request): Json<WithdrawalRequest>,
) -> ApiResult<Json<WithdrawalResponse>> {
    // Parse amount
    let amount = Decimal::from_str(&request.amount)
        .map_err(|_| ApiError::InvalidParameter("Invalid amount format".to_string()))?;

    if amount <= Decimal::ZERO {
        return Err(ApiError::InvalidParameter("Amount must be positive".to_string()));
    }

    // Get user's spot wallet
    let wallet = state.db.wallet_repo()
        .find_spot_wallet(user.0.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::WalletNotFound)?;

    // Get current balance
    let balance = state.db.wallet_repo()
        .get_balance(wallet.id, &request.coin)
        .await
        .map_err(ApiError::from)?;

    // Check sufficient balance (amount + fee)
    // For now, use a simple fee structure (0.1% with min 0.0001)
    let fee = (amount * Decimal::from_str("0.001").unwrap())
        .max(Decimal::from_str("0.0001").unwrap());
    let total_required = amount + fee;

    if balance.available < total_required {
        return Err(ApiError::InsufficientBalance);
    }

    // Check daily withdrawal limit
    let daily_total = state.db.withdrawal_repo()
        .get_daily_total(user.0.user_id, &request.coin)
        .await
        .map_err(ApiError::from)?;

    // Simple limit: 10 coins per day (would be configurable in production)
    let daily_limit = Decimal::from(10);
    if daily_total + amount > daily_limit {
        return Err(ApiError::WithdrawalLimitExceeded);
    }

    let network = request.network.clone().unwrap_or_else(|| request.coin.clone());

    // Lock the funds first
    state.db.wallet_repo()
        .lock(
            wallet.id,
            &request.coin,
            total_required,
        )
        .await
        .map_err(ApiError::from)?;

    // Create withdrawal record
    let withdrawal_id = Uuid::new_v4();
    let withdrawal = openibank_db::DbWithdrawal {
        id: withdrawal_id,
        user_id: user.0.user_id,
        wallet_id: wallet.id,
        currency: request.coin.clone(),
        network,
        amount,
        fee,
        to_address: request.address.clone(),
        memo: request.address_tag.clone(),
        tx_hash: None,
        status: "pending".to_string(),
        approved_by: None,
        approved_at: None,
        completed_at: None,
        failure_reason: None,
        receipt_id: None,
        created_at: chrono::Utc::now(),
    };

    state.db.withdrawal_repo()
        .create(&withdrawal)
        .await
        .map_err(ApiError::from)?;

    tracing::info!(
        user_id = %user.0.user_id,
        withdrawal_id = %withdrawal_id,
        coin = %request.coin,
        amount = %amount,
        address = %request.address,
        "Withdrawal request submitted"
    );

    Ok(Json(WithdrawalResponse { id: withdrawal_id }))
}

/// Get withdrawal history
#[utoipa::path(
    get,
    path = "/api/v1/capital/withdraw/history",
    tag = "Wallet",
    params(
        ("coin" = Option<String>, Query, description = "Filter by asset"),
        ("withdrawOrderId" = Option<String>, Query, description = "Filter by order ID"),
        ("status" = Option<i32>, Query, description = "Filter by status"),
        ("startTime" = Option<i64>, Query, description = "Start time"),
        ("endTime" = Option<i64>, Query, description = "End time"),
        ("offset" = Option<i64>, Query, description = "Offset"),
        ("limit" = Option<i64>, Query, description = "Limit")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Withdrawal history", body = Vec<WithdrawalRecord>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_withdrawal_history(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(query): Query<WithdrawalHistoryQuery>,
) -> ApiResult<Json<Vec<WithdrawalRecord>>> {
    let limit = query.limit.unwrap_or(1000).min(1000);
    let offset = query.offset.unwrap_or(0);

    // Get withdrawals from database
    let withdrawals = state.db.withdrawal_repo()
        .list_by_user(user.user_id, limit, offset)
        .await
        .map_err(ApiError::from)?;

    // Filter by parameters
    let filtered = withdrawals.into_iter()
        .filter(|w| {
            if let Some(ref coin) = query.coin {
                &w.currency == coin
            } else {
                true
            }
        })
        .filter(|w| {
            if let Some(status_code) = query.status {
                withdrawal_status_to_code(&w.status) == status_code
            } else {
                true
            }
        })
        .filter(|w| {
            if let Some(start_time) = query.start_time {
                w.created_at.timestamp_millis() >= start_time
            } else {
                true
            }
        })
        .filter(|w| {
            if let Some(end_time) = query.end_time {
                w.created_at.timestamp_millis() <= end_time
            } else {
                true
            }
        });

    // Convert to DTOs
    let records: Vec<WithdrawalRecord> = filtered
        .map(|w| WithdrawalRecord {
            id: w.id,
            amount: w.amount.to_string(),
            transaction_fee: w.fee.to_string(),
            coin: w.currency,
            status: withdrawal_status_to_code(&w.status),
            address: w.to_address,
            tx_id: w.tx_hash,
            apply_time: w.created_at.timestamp_millis(),
            network: w.network,
            transfer_type: 0, // External withdrawal
            withdraw_order_id: None,
            info: w.failure_reason,
            confirm_no: 0,
        })
        .collect();

    Ok(Json(records))
}

/// Get all coin information
#[utoipa::path(
    get,
    path = "/api/v1/capital/config/getall",
    tag = "Wallet",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "All coin information", body = Vec<AssetInfo>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_all_coins_info(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<AssetInfo>>> {
    // Get user's spot wallet balances
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

    // Create a default list of supported assets
    // In production, this would come from configuration or database
    let default_assets = vec!["BTC", "ETH", "USDT", "USDC"];

    let mut assets: Vec<AssetInfo> = Vec::new();

    for coin in default_assets {
        // Find balance for this coin
        let balance = balances.iter().find(|b| b.currency == coin);
        let (free, locked) = match balance {
            Some(b) => (b.available.to_string(), b.locked.to_string()),
            None => ("0".to_string(), "0".to_string()),
        };

        // Create network info
        let networks = get_networks_for_coin(coin);

        assets.push(AssetInfo {
            coin: coin.to_string(),
            name: get_coin_name(coin),
            deposit_all_enable: true,
            withdraw_all_enable: true,
            free,
            locked,
            freeze: "0".to_string(),
            withdrawing: "0".to_string(),
            is_legal_money: coin == "USDT" || coin == "USDC",
            trading: true,
            network_list: networks,
        });
    }

    Ok(Json(assets))
}

/// Helper to get full coin name
fn get_coin_name(coin: &str) -> String {
    match coin {
        "BTC" => "Bitcoin".to_string(),
        "ETH" => "Ethereum".to_string(),
        "USDT" => "Tether USD".to_string(),
        "USDC" => "USD Coin".to_string(),
        _ => coin.to_string(),
    }
}

/// Helper to get supported networks for a coin
fn get_networks_for_coin(coin: &str) -> Vec<NetworkInfo> {
    match coin {
        "BTC" => vec![
            NetworkInfo {
                network: "BTC".to_string(),
                coin: "BTC".to_string(),
                withdraw_fee: "0.0005".to_string(),
                withdraw_min: "0.001".to_string(),
                withdraw_max: "100".to_string(),
                withdraw_enable: true,
                deposit_enable: true,
                deposit_desc: None,
                withdraw_desc: None,
                special_tips: None,
                min_confirm: 3,
                un_lock_confirm: 6,
                is_default: true,
                estimate_arrival_time: Some(60),
                address_regex: Some("^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$|^bc1[ac-hj-np-z02-9]{11,87}$".to_string()),
                memo_regex: None,
            },
        ],
        "ETH" => vec![
            NetworkInfo {
                network: "ETH".to_string(),
                coin: "ETH".to_string(),
                withdraw_fee: "0.005".to_string(),
                withdraw_min: "0.01".to_string(),
                withdraw_max: "1000".to_string(),
                withdraw_enable: true,
                deposit_enable: true,
                deposit_desc: None,
                withdraw_desc: None,
                special_tips: None,
                min_confirm: 12,
                un_lock_confirm: 64,
                is_default: true,
                estimate_arrival_time: Some(5),
                address_regex: Some("^0x[a-fA-F0-9]{40}$".to_string()),
                memo_regex: None,
            },
        ],
        "USDT" => vec![
            NetworkInfo {
                network: "ETH".to_string(),
                coin: "USDT".to_string(),
                withdraw_fee: "10".to_string(),
                withdraw_min: "20".to_string(),
                withdraw_max: "1000000".to_string(),
                withdraw_enable: true,
                deposit_enable: true,
                deposit_desc: None,
                withdraw_desc: None,
                special_tips: None,
                min_confirm: 12,
                un_lock_confirm: 64,
                is_default: true,
                estimate_arrival_time: Some(5),
                address_regex: Some("^0x[a-fA-F0-9]{40}$".to_string()),
                memo_regex: None,
            },
            NetworkInfo {
                network: "TRX".to_string(),
                coin: "USDT".to_string(),
                withdraw_fee: "1".to_string(),
                withdraw_min: "10".to_string(),
                withdraw_max: "1000000".to_string(),
                withdraw_enable: true,
                deposit_enable: true,
                deposit_desc: None,
                withdraw_desc: None,
                special_tips: None,
                min_confirm: 20,
                un_lock_confirm: 20,
                is_default: false,
                estimate_arrival_time: Some(1),
                address_regex: Some("^T[a-zA-Z0-9]{33}$".to_string()),
                memo_regex: None,
            },
        ],
        "USDC" => vec![
            NetworkInfo {
                network: "ETH".to_string(),
                coin: "USDC".to_string(),
                withdraw_fee: "10".to_string(),
                withdraw_min: "20".to_string(),
                withdraw_max: "1000000".to_string(),
                withdraw_enable: true,
                deposit_enable: true,
                deposit_desc: None,
                withdraw_desc: None,
                special_tips: None,
                min_confirm: 12,
                un_lock_confirm: 64,
                is_default: true,
                estimate_arrival_time: Some(5),
                address_regex: Some("^0x[a-fA-F0-9]{40}$".to_string()),
                memo_regex: None,
            },
        ],
        _ => vec![],
    }
}

/// Internal transfer (between users)
#[utoipa::path(
    post,
    path = "/api/v1/capital/transfer",
    tag = "Wallet",
    request_body = InternalTransferRequest,
    security(
        ("bearer" = [])
    ),
    responses(
        (status = 200, description = "Transfer completed", body = InternalTransferResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn internal_transfer(
    State(state): State<Arc<AppState>>,
    user: Require2FA,
    Json(request): Json<InternalTransferRequest>,
) -> ApiResult<Json<InternalTransferResponse>> {
    // Parse amount
    let amount = Decimal::from_str(&request.amount)
        .map_err(|_| ApiError::InvalidParameter("Invalid amount format".to_string()))?;

    if amount <= Decimal::ZERO {
        return Err(ApiError::InvalidParameter("Amount must be positive".to_string()));
    }

    // Find target user
    let target_user = match request.target_type.as_str() {
        "email" => {
            state.db.user_repo()
                .find_by_email(&request.target)
                .await
                .map_err(ApiError::from)?
        }
        "uid" => {
            let target_id = Uuid::from_str(&request.target)
                .map_err(|_| ApiError::InvalidParameter("Invalid user ID".to_string()))?;
            state.db.user_repo()
                .find_by_id(target_id)
                .await
                .map_err(ApiError::from)?
        }
        _ => return Err(ApiError::InvalidParameter("Invalid target_type".to_string())),
    };

    let target_user = target_user.ok_or(ApiError::AccountNotFound)?;

    // Cannot transfer to self
    if target_user.id == user.0.user_id {
        return Err(ApiError::InvalidParameter("Cannot transfer to yourself".to_string()));
    }

    // Get sender's wallet
    let sender_wallet = state.db.wallet_repo()
        .find_spot_wallet(user.0.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::WalletNotFound)?;

    // Get or create receiver's wallet
    let receiver_wallet = state.db.wallet_repo()
        .find_spot_wallet(target_user.id)
        .await
        .map_err(ApiError::from)?;

    let receiver_wallet = match receiver_wallet {
        Some(w) => w,
        None => {
            // Create wallet for receiver
            state.db.wallet_repo()
                .create(target_user.id, "spot", None)
                .await
                .map_err(ApiError::from)?
        }
    };

    // Check sender balance
    let sender_balance = state.db.wallet_repo()
        .get_balance(sender_wallet.id, &request.coin)
        .await
        .map_err(ApiError::from)?;

    if sender_balance.available < amount {
        return Err(ApiError::InsufficientBalance);
    }

    let transfer_id = Uuid::new_v4();

    // Debit sender
    state.db.wallet_repo()
        .debit(
            sender_wallet.id,
            &request.coin,
            amount,
            "internal_transfer",
            Some("transfer"),
            Some(transfer_id),
            None,
        )
        .await
        .map_err(ApiError::from)?;

    // Credit receiver
    state.db.wallet_repo()
        .credit(
            receiver_wallet.id,
            &request.coin,
            amount,
            "internal_transfer",
            Some("transfer"),
            Some(transfer_id),
            None,
        )
        .await
        .map_err(ApiError::from)?;

    tracing::info!(
        from_user = %user.0.user_id,
        to_user = %target_user.id,
        transfer_id = %transfer_id,
        coin = %request.coin,
        amount = %amount,
        "Internal transfer completed"
    );

    Ok(Json(InternalTransferResponse { tran_id: transfer_id }))
}
