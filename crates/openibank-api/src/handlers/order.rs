//! Order Handlers
//!
//! Endpoints for order creation, management, and trade history.
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
    CreateOrderRequest, OrderInfo, OrderResultResponse,
    QueryOrderRequest, CancelOrderRequest, CancelOrderResponse,
    CancelAllOrdersRequest, OpenOrdersQuery, AllOrdersQuery, AccountTradesQuery,
    AccountTrade, OrderStatus, TimeInForce, SelfTradePreventionMode, OrderSide, OrderType,
};
use crate::error::{ApiError, ApiResult};
use crate::extractors::AuthenticatedUser;
use crate::state::AppState;

/// Convert database order status to DTO status
fn db_status_to_order_status(status: &str) -> OrderStatus {
    match status {
        "new" | "open" => OrderStatus::New,
        "partially_filled" => OrderStatus::PartiallyFilled,
        "filled" => OrderStatus::Filled,
        "cancelled" | "canceled" => OrderStatus::Canceled,
        "pending_cancel" => OrderStatus::PendingCancel,
        "rejected" => OrderStatus::Rejected,
        "expired" => OrderStatus::Expired,
        _ => OrderStatus::New,
    }
}

/// Convert database side to DTO side
fn db_side_to_order_side(side: &str) -> OrderSide {
    match side.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => OrderSide::Buy,
    }
}

/// Convert database order type to DTO order type
fn db_type_to_order_type(order_type: &str) -> OrderType {
    match order_type.to_lowercase().as_str() {
        "limit" => OrderType::Limit,
        "market" => OrderType::Market,
        "stop_loss" => OrderType::StopLoss,
        "stop_loss_limit" => OrderType::StopLossLimit,
        "take_profit" => OrderType::TakeProfit,
        "take_profit_limit" => OrderType::TakeProfitLimit,
        "limit_maker" => OrderType::LimitMaker,
        _ => OrderType::Limit,
    }
}

/// Convert database time in force to DTO time in force
fn db_tif_to_time_in_force(tif: &str) -> TimeInForce {
    match tif.to_uppercase().as_str() {
        "GTC" => TimeInForce::Gtc,
        "IOC" => TimeInForce::Ioc,
        "FOK" => TimeInForce::Fok,
        "GTD" => TimeInForce::Gtd,
        _ => TimeInForce::Gtc,
    }
}

/// Convert DbOrder to OrderInfo DTO
fn db_order_to_order_info(order: openibank_db::DbOrder) -> OrderInfo {
    OrderInfo {
        symbol: order.market_id.clone(),
        order_id: order.id.as_u128() as i64, // Convert UUID to numeric ID
        order_list_id: -1,
        client_order_id: order.client_order_id.unwrap_or_default(),
        price: order.price.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string()),
        orig_qty: order.amount.to_string(),
        executed_qty: order.filled.to_string(),
        cummulative_quote_qty: order.quote_filled.to_string(),
        status: db_status_to_order_status(&order.status),
        time_in_force: db_tif_to_time_in_force(&order.time_in_force),
        order_type: db_type_to_order_type(&order.order_type),
        side: db_side_to_order_side(&order.side),
        stop_price: order.stop_price.map(|p| p.to_string()),
        iceberg_qty: order.iceberg_qty.map(|q| q.to_string()),
        time: order.created_at.timestamp_millis(),
        update_time: order.updated_at.timestamp_millis(),
        is_working: order.status == "new" || order.status == "open" || order.status == "partially_filled",
        working_time: order.created_at.timestamp_millis(),
        orig_quote_order_qty: "0".to_string(),
        self_trade_prevention_mode: SelfTradePreventionMode::None,
    }
}

/// Parse symbol into base and quote currencies
fn parse_symbol(symbol: &str) -> Result<(String, String), ApiError> {
    // Common quote currencies in order of precedence
    let quote_currencies = ["USDT", "USDC", "BUSD", "USD", "BTC", "ETH", "BNB"];

    for quote in quote_currencies {
        if symbol.ends_with(quote) && symbol.len() > quote.len() {
            let base = &symbol[..symbol.len() - quote.len()];
            return Ok((base.to_string(), quote.to_string()));
        }
    }

    // Default fallback - assume last 4 chars are quote
    if symbol.len() >= 5 {
        let (base, quote) = symbol.split_at(symbol.len() - 4);
        return Ok((base.to_string(), quote.to_string()));
    }

    Err(ApiError::InvalidParameter(format!("Invalid symbol format: {}", symbol)))
}

/// Create new order
#[utoipa::path(
    post,
    path = "/api/v1/order",
    tag = "Trading",
    request_body = CreateOrderRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Order created"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Trading not allowed")
    )
)]
pub async fn create_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(request): Json<CreateOrderRequest>,
) -> ApiResult<Json<OrderResultResponse>> {
    // Validate quantity or quote_order_qty is provided
    let quantity = match (&request.quantity, &request.quote_order_qty) {
        (Some(qty), _) => Decimal::from_str(qty)
            .map_err(|_| ApiError::InvalidParameter("Invalid quantity format".to_string()))?,
        (None, Some(_)) if request.order_type == OrderType::Market => {
            // For market orders with quote_order_qty, we'll need to calculate later
            Decimal::ZERO
        }
        _ => return Err(ApiError::InvalidParameter("Quantity is required".to_string())),
    };

    if quantity <= Decimal::ZERO && request.quote_order_qty.is_none() {
        return Err(ApiError::InvalidParameter("Quantity must be positive".to_string()));
    }

    // Parse price for limit orders
    let price = match &request.price {
        Some(p) => Some(Decimal::from_str(p)
            .map_err(|_| ApiError::InvalidParameter("Invalid price format".to_string()))?),
        None if request.order_type == OrderType::Limit ||
                request.order_type == OrderType::StopLossLimit ||
                request.order_type == OrderType::TakeProfitLimit ||
                request.order_type == OrderType::LimitMaker => {
            return Err(ApiError::InvalidParameter("Price is required for limit orders".to_string()));
        }
        None => None,
    };

    // Parse stop price if provided
    let stop_price = match &request.stop_price {
        Some(p) => Some(Decimal::from_str(p)
            .map_err(|_| ApiError::InvalidParameter("Invalid stop price format".to_string()))?),
        None => None,
    };

    // Parse iceberg quantity if provided
    let iceberg_qty = match &request.iceberg_qty {
        Some(q) => Some(Decimal::from_str(q)
            .map_err(|_| ApiError::InvalidParameter("Invalid iceberg quantity format".to_string()))?),
        None => None,
    };

    // Get user's spot wallet for balance checking
    let wallet = state.db.wallet_repo()
        .find_spot_wallet(user.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::WalletNotFound)?;

    // Determine base and quote currencies from symbol
    let (base_currency, quote_currency) = parse_symbol(&request.symbol)?;

    // Check balance depending on order side
    match request.side {
        OrderSide::Buy => {
            // Need quote currency (e.g., USDT) to buy
            let required = match (&price, request.order_type) {
                (Some(p), _) => quantity * p,
                (None, OrderType::Market) => {
                    // For market orders, we'd need to estimate from order book
                    // For now, just check basic balance exists
                    quantity * Decimal::from(1) // Placeholder
                }
                _ => Decimal::ZERO,
            };

            let balance = state.db.wallet_repo()
                .get_balance(wallet.id, &quote_currency)
                .await
                .map_err(ApiError::from)?;

            if balance.available < required {
                return Err(ApiError::InsufficientBalance);
            }
        }
        OrderSide::Sell => {
            // Need base currency (e.g., BTC) to sell
            let balance = state.db.wallet_repo()
                .get_balance(wallet.id, &base_currency)
                .await
                .map_err(ApiError::from)?;

            if balance.available < quantity {
                return Err(ApiError::InsufficientBalance);
            }
        }
    }

    // Generate order ID
    let order_id = Uuid::new_v4();
    let client_order_id = request.new_client_order_id
        .clone()
        .unwrap_or_else(|| order_id.to_string());

    // Create order in database
    let db_order = openibank_db::DbOrder {
        id: order_id,
        user_id: user.user_id,
        market_id: request.symbol.clone(),
        client_order_id: Some(client_order_id.clone()),
        side: match request.side {
            OrderSide::Buy => "buy".to_string(),
            OrderSide::Sell => "sell".to_string(),
        },
        order_type: match request.order_type {
            OrderType::Limit => "limit".to_string(),
            OrderType::Market => "market".to_string(),
            OrderType::StopLoss => "stop_loss".to_string(),
            OrderType::StopLossLimit => "stop_loss_limit".to_string(),
            OrderType::TakeProfit => "take_profit".to_string(),
            OrderType::TakeProfitLimit => "take_profit_limit".to_string(),
            OrderType::LimitMaker => "limit_maker".to_string(),
        },
        price,
        stop_price,
        trailing_delta: request.trailing_delta.map(Decimal::from),
        amount: quantity,
        filled: Decimal::ZERO,
        remaining: quantity,
        quote_filled: Decimal::ZERO,
        fee_total: Decimal::ZERO,
        fee_currency: Some(quote_currency.clone()),
        time_in_force: match request.time_in_force.unwrap_or(TimeInForce::Gtc) {
            TimeInForce::Gtc => "GTC".to_string(),
            TimeInForce::Ioc => "IOC".to_string(),
            TimeInForce::Fok => "FOK".to_string(),
            TimeInForce::Gtd => "GTD".to_string(),
        },
        expire_at: None,
        post_only: request.order_type == OrderType::LimitMaker,
        reduce_only: false,
        iceberg_qty,
        status: "new".to_string(),
        reject_reason: None,
        permit_id: None,
        commitment_id: None,
        receipt_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let _created_order = state.db.order_repo()
        .create(&db_order)
        .await
        .map_err(ApiError::from)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    tracing::info!(
        user_id = %user.user_id,
        order_id = %order_id,
        symbol = %request.symbol,
        side = ?request.side,
        order_type = ?request.order_type,
        quantity = %quantity,
        price = ?price,
        "Order created"
    );

    Ok(Json(OrderResultResponse {
        symbol: request.symbol,
        order_id: order_id.as_u128() as i64,
        order_list_id: -1,
        client_order_id,
        transact_time: now,
        price: price.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string()),
        orig_qty: quantity.to_string(),
        executed_qty: "0".to_string(),
        cummulative_quote_qty: "0".to_string(),
        status: OrderStatus::New,
        time_in_force: request.time_in_force.unwrap_or(TimeInForce::Gtc),
        order_type: request.order_type,
        side: request.side,
        working_time: now,
        self_trade_prevention_mode: request.self_trade_prevention_mode.unwrap_or(SelfTradePreventionMode::None),
    }))
}

/// Query order
#[utoipa::path(
    get,
    path = "/api/v1/order",
    tag = "Trading",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("orderId" = Option<i64>, Query, description = "Order ID"),
        ("origClientOrderId" = Option<String>, Query, description = "Client order ID")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Order info", body = OrderInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found")
    )
)]
pub async fn query_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(request): Query<QueryOrderRequest>,
) -> ApiResult<Json<OrderInfo>> {
    // We need either order_id or orig_client_order_id
    if request.order_id.is_none() && request.orig_client_order_id.is_none() {
        return Err(ApiError::InvalidParameter("Either orderId or origClientOrderId is required".to_string()));
    }

    // Get user's orders for this symbol
    let orders = state.db.order_repo()
        .find_by_user(user.user_id, Some(&request.symbol), 1000, 0)
        .await
        .map_err(ApiError::from)?;

    // Find matching order
    let order = orders.into_iter().find(|o| {
        // Check by order_id (converted from UUID)
        if let Some(order_id) = request.order_id {
            if o.id.as_u128() as i64 == order_id {
                return true;
            }
        }
        // Check by client_order_id
        if let Some(ref client_id) = request.orig_client_order_id {
            if o.client_order_id.as_ref() == Some(client_id) {
                return true;
            }
        }
        false
    });

    match order {
        Some(o) => Ok(Json(db_order_to_order_info(o))),
        None => Err(ApiError::NotFound("Order not found".to_string())),
    }
}

/// Cancel order
#[utoipa::path(
    delete,
    path = "/api/v1/order",
    tag = "Trading",
    request_body = CancelOrderRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Order cancelled", body = CancelOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found")
    )
)]
pub async fn cancel_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(request): Json<CancelOrderRequest>,
) -> ApiResult<Json<CancelOrderResponse>> {
    // We need either order_id or orig_client_order_id
    if request.order_id.is_none() && request.orig_client_order_id.is_none() {
        return Err(ApiError::InvalidParameter("Either orderId or origClientOrderId is required".to_string()));
    }

    // Get user's orders for this symbol to find the one to cancel
    let orders = state.db.order_repo()
        .find_open_by_user(user.user_id, Some(&request.symbol))
        .await
        .map_err(ApiError::from)?;

    // Find matching order
    let order = orders.into_iter().find(|o| {
        if let Some(order_id) = request.order_id {
            if o.id.as_u128() as i64 == order_id {
                return true;
            }
        }
        if let Some(ref client_id) = request.orig_client_order_id {
            if o.client_order_id.as_ref() == Some(client_id) {
                return true;
            }
        }
        false
    });

    let order = order.ok_or(ApiError::NotFound("Order not found or already filled/cancelled".to_string()))?;

    // Cancel the order
    let cancelled_order = state.db.order_repo()
        .cancel(order.id)
        .await
        .map_err(ApiError::from)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    tracing::info!(
        user_id = %user.user_id,
        order_id = %order.id,
        symbol = %request.symbol,
        "Order cancelled"
    );

    Ok(Json(CancelOrderResponse {
        symbol: cancelled_order.market_id.clone(),
        orig_client_order_id: cancelled_order.client_order_id.clone().unwrap_or_default(),
        order_id: cancelled_order.id.as_u128() as i64,
        order_list_id: -1,
        client_order_id: request.new_client_order_id.unwrap_or_else(|| cancelled_order.client_order_id.clone().unwrap_or_default()),
        transact_time: now,
        price: cancelled_order.price.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string()),
        orig_qty: cancelled_order.amount.to_string(),
        executed_qty: cancelled_order.filled.to_string(),
        cummulative_quote_qty: cancelled_order.quote_filled.to_string(),
        status: OrderStatus::Canceled,
        time_in_force: db_tif_to_time_in_force(&cancelled_order.time_in_force),
        order_type: db_type_to_order_type(&cancelled_order.order_type),
        side: db_side_to_order_side(&cancelled_order.side),
        self_trade_prevention_mode: SelfTradePreventionMode::None,
    }))
}

/// Cancel all open orders for a symbol
#[utoipa::path(
    delete,
    path = "/api/v1/openOrders",
    tag = "Trading",
    request_body = CancelAllOrdersRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Orders cancelled", body = Vec<CancelOrderResponse>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn cancel_all_orders(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(request): Json<CancelAllOrdersRequest>,
) -> ApiResult<Json<Vec<CancelOrderResponse>>> {
    // Get all open orders for this symbol first
    let open_orders = state.db.order_repo()
        .find_open_by_user(user.user_id, Some(&request.symbol))
        .await
        .map_err(ApiError::from)?;

    // Cancel all orders
    let cancelled_count = state.db.order_repo()
        .cancel_all_by_user(user.user_id, Some(&request.symbol))
        .await
        .map_err(ApiError::from)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    tracing::info!(
        user_id = %user.user_id,
        symbol = %request.symbol,
        cancelled_count = %cancelled_count,
        "All orders cancelled"
    );

    // Convert to response format
    let responses: Vec<CancelOrderResponse> = open_orders
        .into_iter()
        .map(|o| CancelOrderResponse {
            symbol: o.market_id.clone(),
            orig_client_order_id: o.client_order_id.clone().unwrap_or_default(),
            order_id: o.id.as_u128() as i64,
            order_list_id: -1,
            client_order_id: o.client_order_id.clone().unwrap_or_default(),
            transact_time: now,
            price: o.price.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string()),
            orig_qty: o.amount.to_string(),
            executed_qty: o.filled.to_string(),
            cummulative_quote_qty: o.quote_filled.to_string(),
            status: OrderStatus::Canceled,
            time_in_force: db_tif_to_time_in_force(&o.time_in_force),
            order_type: db_type_to_order_type(&o.order_type),
            side: db_side_to_order_side(&o.side),
            self_trade_prevention_mode: SelfTradePreventionMode::None,
        })
        .collect();

    Ok(Json(responses))
}

/// Get open orders
#[utoipa::path(
    get,
    path = "/api/v1/openOrders",
    tag = "Trading",
    params(
        ("symbol" = Option<String>, Query, description = "Symbol filter")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Open orders", body = Vec<OrderInfo>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_open_orders(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(query): Query<OpenOrdersQuery>,
) -> ApiResult<Json<Vec<OrderInfo>>> {
    let orders = state.db.order_repo()
        .find_open_by_user(user.user_id, query.symbol.as_deref())
        .await
        .map_err(ApiError::from)?;

    let order_infos: Vec<OrderInfo> = orders
        .into_iter()
        .map(db_order_to_order_info)
        .collect();

    Ok(Json(order_infos))
}

/// Get all orders (history)
#[utoipa::path(
    get,
    path = "/api/v1/allOrders",
    tag = "Trading",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("orderId" = Option<i64>, Query, description = "Order ID to start from"),
        ("startTime" = Option<i64>, Query, description = "Start time"),
        ("endTime" = Option<i64>, Query, description = "End time"),
        ("limit" = Option<i32>, Query, description = "Limit")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "All orders", body = Vec<OrderInfo>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_all_orders(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(query): Query<AllOrdersQuery>,
) -> ApiResult<Json<Vec<OrderInfo>>> {
    let limit = query.limit.unwrap_or(500).min(1000) as i64;

    let orders = state.db.order_repo()
        .find_by_user(user.user_id, Some(&query.symbol), limit, 0)
        .await
        .map_err(ApiError::from)?;

    // Filter by time range if specified
    let filtered: Vec<OrderInfo> = orders
        .into_iter()
        .filter(|o| {
            if let Some(start_time) = query.start_time {
                if o.created_at.timestamp_millis() < start_time {
                    return false;
                }
            }
            if let Some(end_time) = query.end_time {
                if o.created_at.timestamp_millis() > end_time {
                    return false;
                }
            }
            true
        })
        .map(db_order_to_order_info)
        .collect();

    Ok(Json(filtered))
}

/// Get account trades
#[utoipa::path(
    get,
    path = "/api/v1/myTrades",
    tag = "Trading",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("orderId" = Option<i64>, Query, description = "Order ID"),
        ("startTime" = Option<i64>, Query, description = "Start time"),
        ("endTime" = Option<i64>, Query, description = "End time"),
        ("fromId" = Option<i64>, Query, description = "Trade ID to start from"),
        ("limit" = Option<i32>, Query, description = "Limit")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Account trades", body = Vec<AccountTrade>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_account_trades(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(query): Query<AccountTradesQuery>,
) -> ApiResult<Json<Vec<AccountTrade>>> {
    let limit = query.limit.unwrap_or(500).min(1000) as i64;

    let trades = state.db.trade_repo()
        .find_by_user(user.user_id, Some(&query.symbol), limit, 0)
        .await
        .map_err(ApiError::from)?;

    // Filter and convert to DTO
    let account_trades: Vec<AccountTrade> = trades
        .into_iter()
        .filter(|t| {
            if let Some(start_time) = query.start_time {
                if t.created_at.timestamp_millis() < start_time {
                    return false;
                }
            }
            if let Some(end_time) = query.end_time {
                if t.created_at.timestamp_millis() > end_time {
                    return false;
                }
            }
            true
        })
        .map(|t| {
            // Determine if user is buyer/maker
            let is_maker = t.maker_user_id == user.user_id;
            let is_buyer = if is_maker { t.is_buyer_maker } else { !t.is_buyer_maker };

            let (commission, commission_asset) = if is_maker {
                (t.maker_fee.to_string(), t.maker_fee_currency.clone())
            } else {
                (t.taker_fee.to_string(), t.taker_fee_currency.clone())
            };

            // Get order_id - use maker or taker order based on role
            let order_id = if is_maker {
                t.maker_order_id.as_u128() as i64
            } else {
                t.taker_order_id.as_u128() as i64
            };

            AccountTrade {
                symbol: t.market_id.clone(),
                id: t.id.as_u128() as i64,
                order_id,
                order_list_id: -1,
                price: t.price.to_string(),
                qty: t.amount.to_string(),
                quote_qty: t.quote_amount.to_string(),
                commission,
                commission_asset,
                time: t.created_at.timestamp_millis(),
                is_buyer,
                is_maker,
                is_best_match: true,
            }
        })
        .collect();

    Ok(Json(account_trades))
}
