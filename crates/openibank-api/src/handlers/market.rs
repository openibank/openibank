//! Market Data Handlers
//!
//! Public endpoints for market data (no authentication required).
//! Fully integrated with database layer.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Duration, Utc};
use std::sync::Arc;

use crate::dto::market::{
    ExchangeInfo, RateLimit, SymbolInfo, SymbolFilter,
    OrderBookQuery, OrderBook,
    RecentTradesQuery, TradeRecord,
    AggTradesQuery, AggTrade,
    KlinesQuery, Kline, KlineInterval,
    TickerQuery, Ticker24hr,
    PriceTicker, BookTicker,
    AvgPriceQuery, AvgPrice,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Convert KlineInterval enum to string representation
fn interval_to_str(interval: KlineInterval) -> &'static str {
    match interval {
        KlineInterval::OneSecond => "1s",
        KlineInterval::OneMinute => "1m",
        KlineInterval::ThreeMinutes => "3m",
        KlineInterval::FiveMinutes => "5m",
        KlineInterval::FifteenMinutes => "15m",
        KlineInterval::ThirtyMinutes => "30m",
        KlineInterval::OneHour => "1h",
        KlineInterval::TwoHours => "2h",
        KlineInterval::FourHours => "4h",
        KlineInterval::SixHours => "6h",
        KlineInterval::EightHours => "8h",
        KlineInterval::TwelveHours => "12h",
        KlineInterval::OneDay => "1d",
        KlineInterval::ThreeDays => "3d",
        KlineInterval::OneWeek => "1w",
        KlineInterval::OneMonth => "1M",
    }
}

/// Get interval duration
fn interval_to_duration(interval: KlineInterval) -> Duration {
    match interval {
        KlineInterval::OneSecond => Duration::seconds(1),
        KlineInterval::OneMinute => Duration::minutes(1),
        KlineInterval::ThreeMinutes => Duration::minutes(3),
        KlineInterval::FiveMinutes => Duration::minutes(5),
        KlineInterval::FifteenMinutes => Duration::minutes(15),
        KlineInterval::ThirtyMinutes => Duration::minutes(30),
        KlineInterval::OneHour => Duration::hours(1),
        KlineInterval::TwoHours => Duration::hours(2),
        KlineInterval::FourHours => Duration::hours(4),
        KlineInterval::SixHours => Duration::hours(6),
        KlineInterval::EightHours => Duration::hours(8),
        KlineInterval::TwelveHours => Duration::hours(12),
        KlineInterval::OneDay => Duration::days(1),
        KlineInterval::ThreeDays => Duration::days(3),
        KlineInterval::OneWeek => Duration::weeks(1),
        KlineInterval::OneMonth => Duration::days(30),
    }
}

/// Get exchange information
#[utoipa::path(
    get,
    path = "/api/v1/exchangeInfo",
    tag = "Market Data",
    responses(
        (status = 200, description = "Exchange information", body = ExchangeInfo)
    )
)]
pub async fn get_exchange_info(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ExchangeInfo>> {
    let server_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Get all active markets from database
    let markets = state.db.market_repo()
        .list_active()
        .await
        .map_err(ApiError::from)?;

    // Convert markets to symbol info
    let symbols: Vec<SymbolInfo> = markets
        .into_iter()
        .map(|m| {
            let mut filters = vec![];

            // Add price filter
            filters.push(SymbolFilter {
                filter_type: "PRICE_FILTER".to_string(),
                min_price: Some(m.tick_size.to_string()),
                max_price: Some("100000000".to_string()),
                tick_size: Some(m.tick_size.to_string()),
                ..Default::default()
            });

            // Add lot size filter
            filters.push(SymbolFilter {
                filter_type: "LOT_SIZE".to_string(),
                min_qty: Some(m.min_amount.to_string()),
                max_qty: m.max_amount.map(|v| v.to_string()),
                step_size: Some(m.lot_size.to_string()),
                ..Default::default()
            });

            // Add notional filter if present
            if let Some(min_notional) = m.min_notional {
                filters.push(SymbolFilter {
                    filter_type: "NOTIONAL".to_string(),
                    min_notional: Some(min_notional.to_string()),
                    apply_to_market: Some(true),
                    avg_price_mins: Some(5),
                    ..Default::default()
                });
            }

            let status = match m.status.as_str() {
                "active" => "TRADING",
                "halted" => "HALT",
                "auction" => "AUCTION_MATCH",
                _ => "BREAK",
            };

            SymbolInfo {
                symbol: m.id,
                status: status.to_string(),
                base_asset: m.base_currency,
                base_asset_precision: m.amount_precision as i32,
                quote_asset: m.quote_currency,
                quote_precision: m.price_precision as i32,
                quote_asset_precision: m.price_precision as i32,
                base_commission_precision: 8,
                quote_commission_precision: 8,
                order_types: vec![
                    "LIMIT".to_string(),
                    "MARKET".to_string(),
                    "STOP_LOSS_LIMIT".to_string(),
                    "TAKE_PROFIT_LIMIT".to_string(),
                ],
                iceberg_allowed: true,
                oco_allowed: true,
                quote_order_qty_market_allowed: true,
                allow_trailing_stop: true,
                cancel_replace_allowed: true,
                is_spot_trading_allowed: true,
                is_margin_trading_allowed: false,
                filters,
                permissions: vec!["SPOT".to_string()],
                default_self_trade_prevention_mode: "NONE".to_string(),
                allowed_self_trade_prevention_modes: vec!["NONE".to_string()],
            }
        })
        .collect();

    // If no markets in DB, return a default BTCUSDT
    let symbols = if symbols.is_empty() {
        vec![SymbolInfo {
            symbol: "BTCUSDT".to_string(),
            status: "TRADING".to_string(),
            base_asset: "BTC".to_string(),
            base_asset_precision: 8,
            quote_asset: "USDT".to_string(),
            quote_precision: 8,
            quote_asset_precision: 8,
            base_commission_precision: 8,
            quote_commission_precision: 8,
            order_types: vec!["LIMIT".to_string(), "MARKET".to_string()],
            iceberg_allowed: true,
            oco_allowed: true,
            quote_order_qty_market_allowed: true,
            allow_trailing_stop: true,
            cancel_replace_allowed: true,
            is_spot_trading_allowed: true,
            is_margin_trading_allowed: false,
            filters: vec![],
            permissions: vec!["SPOT".to_string()],
            default_self_trade_prevention_mode: "NONE".to_string(),
            allowed_self_trade_prevention_modes: vec!["NONE".to_string()],
        }]
    } else {
        symbols
    };

    Ok(Json(ExchangeInfo {
        timezone: "UTC".to_string(),
        server_time,
        rate_limits: vec![
            RateLimit {
                rate_limit_type: "REQUEST_WEIGHT".to_string(),
                interval: "MINUTE".to_string(),
                interval_num: 1,
                limit: 1200,
            },
            RateLimit {
                rate_limit_type: "ORDERS".to_string(),
                interval: "SECOND".to_string(),
                interval_num: 10,
                limit: 100,
            },
            RateLimit {
                rate_limit_type: "ORDERS".to_string(),
                interval: "DAY".to_string(),
                interval_num: 1,
                limit: 200000,
            },
        ],
        exchange_filters: vec![],
        symbols,
    }))
}

/// Get order book depth
#[utoipa::path(
    get,
    path = "/api/v1/depth",
    tag = "Market Data",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("limit" = Option<i32>, Query, description = "Limit")
    ),
    responses(
        (status = 200, description = "Order book", body = OrderBook),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_order_book(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OrderBookQuery>,
) -> ApiResult<Json<OrderBook>> {
    // Validate market exists
    let market = state.db.market_repo()
        .find_by_id(&query.symbol)
        .await
        .map_err(ApiError::from)?;

    if market.is_none() {
        return Err(ApiError::InvalidSymbol(query.symbol));
    }

    let limit = query.limit.unwrap_or(100).min(5000) as usize;

    // Get open orders for this market to build order book
    // Note: This is a simplified implementation - a real system would use
    // an in-memory order book structure for performance
    let orders = state.db.order_repo()
        .find_open_by_market(&query.symbol)
        .await
        .map_err(ApiError::from)?;

    let mut bids: Vec<(String, String)> = vec![];
    let mut asks: Vec<(String, String)> = vec![];

    for order in orders {
        if let Some(price) = order.price {
            let qty = order.remaining.to_string();
            let price_str = price.to_string();

            match order.side.as_str() {
                "buy" => bids.push((price_str, qty)),
                "sell" => asks.push((price_str, qty)),
                _ => {}
            }
        }
    }

    // Sort bids descending (highest first), asks ascending (lowest first)
    bids.sort_by(|a, b| {
        b.0.parse::<f64>().unwrap_or(0.0)
            .partial_cmp(&a.0.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    asks.sort_by(|a, b| {
        a.0.parse::<f64>().unwrap_or(0.0)
            .partial_cmp(&b.0.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Truncate to limit
    bids.truncate(limit);
    asks.truncate(limit);

    // Convert to Binance format [price, qty]
    let bids: Vec<[String; 2]> = bids.into_iter().map(|(p, q)| [p, q]).collect();
    let asks: Vec<[String; 2]> = asks.into_iter().map(|(p, q)| [p, q]).collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Ok(Json(OrderBook {
        last_update_id: now,
        bids,
        asks,
    }))
}

/// Get recent trades
#[utoipa::path(
    get,
    path = "/api/v1/trades",
    tag = "Market Data",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("limit" = Option<i32>, Query, description = "Limit")
    ),
    responses(
        (status = 200, description = "Recent trades", body = Vec<TradeRecord>),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_recent_trades(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecentTradesQuery>,
) -> ApiResult<Json<Vec<TradeRecord>>> {
    // Validate market exists
    let market = state.db.market_repo()
        .find_by_id(&query.symbol)
        .await
        .map_err(ApiError::from)?;

    if market.is_none() {
        return Err(ApiError::InvalidSymbol(query.symbol));
    }

    let limit = query.limit.unwrap_or(500).min(1000) as i64;

    // Get recent trades from database
    let trades = state.db.trade_repo()
        .find_recent_by_market(&query.symbol, limit)
        .await
        .map_err(ApiError::from)?;

    // Convert to API format
    let trade_records: Vec<TradeRecord> = trades
        .into_iter()
        .map(|t| TradeRecord {
            id: t.id.as_u128() as i64,
            price: t.price.to_string(),
            qty: t.amount.to_string(),
            quote_qty: t.quote_amount.to_string(),
            time: t.created_at.timestamp_millis(),
            is_buyer_maker: t.is_buyer_maker,
            is_best_match: true,
        })
        .collect();

    Ok(Json(trade_records))
}

/// Get aggregate trades
#[utoipa::path(
    get,
    path = "/api/v1/aggTrades",
    tag = "Market Data",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("fromId" = Option<i64>, Query, description = "From aggregate trade ID"),
        ("startTime" = Option<i64>, Query, description = "Start time"),
        ("endTime" = Option<i64>, Query, description = "End time"),
        ("limit" = Option<i32>, Query, description = "Limit")
    ),
    responses(
        (status = 200, description = "Aggregate trades", body = Vec<AggTrade>),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_agg_trades(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AggTradesQuery>,
) -> ApiResult<Json<Vec<AggTrade>>> {
    // Validate market exists
    let market = state.db.market_repo()
        .find_by_id(&query.symbol)
        .await
        .map_err(ApiError::from)?;

    if market.is_none() {
        return Err(ApiError::InvalidSymbol(query.symbol));
    }

    let limit = query.limit.unwrap_or(500).min(1000) as i64;

    // Get recent trades (aggregate trades are simplified here - each trade is its own aggregate)
    let trades = state.db.trade_repo()
        .find_recent_by_market(&query.symbol, limit)
        .await
        .map_err(ApiError::from)?;

    // Convert to aggregate trade format
    let agg_trades: Vec<AggTrade> = trades
        .into_iter()
        .map(|t| {
            let trade_id = t.id.as_u128() as i64;
            AggTrade {
                agg_trade_id: trade_id,
                price: t.price.to_string(),
                qty: t.amount.to_string(),
                first_id: trade_id,
                last_id: trade_id,
                timestamp: t.created_at.timestamp_millis(),
                is_buyer_maker: t.is_buyer_maker,
                is_best_match: true,
            }
        })
        .collect();

    Ok(Json(agg_trades))
}

/// Get klines (candlestick data)
#[utoipa::path(
    get,
    path = "/api/v1/klines",
    tag = "Market Data",
    params(
        ("symbol" = String, Query, description = "Symbol"),
        ("interval" = String, Query, description = "Kline interval"),
        ("startTime" = Option<i64>, Query, description = "Start time"),
        ("endTime" = Option<i64>, Query, description = "End time"),
        ("limit" = Option<i32>, Query, description = "Limit")
    ),
    responses(
        (status = 200, description = "Klines", body = Vec<Kline>),
        (status = 400, description = "Invalid parameters")
    )
)]
pub async fn get_klines(
    State(state): State<Arc<AppState>>,
    Query(query): Query<KlinesQuery>,
) -> ApiResult<Json<Vec<Kline>>> {
    // Validate market exists
    let market = state.db.market_repo()
        .find_by_id(&query.symbol)
        .await
        .map_err(ApiError::from)?;

    if market.is_none() {
        return Err(ApiError::InvalidSymbol(query.symbol));
    }

    let limit = query.limit.unwrap_or(500).min(1000) as i64;
    let interval_str = interval_to_str(query.interval);
    let interval_duration = interval_to_duration(query.interval);

    // Calculate time range
    let end_time = query.end_time
        .map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now))
        .unwrap_or_else(Utc::now);

    let start_time = query.start_time
        .map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now))
        .unwrap_or_else(|| end_time - interval_duration * limit as i32);

    // Get candles from database
    let candles = state.db.candle_repo()
        .get_candles(&query.symbol, interval_str, start_time, end_time, limit)
        .await
        .map_err(ApiError::from)?;

    // Convert to Binance kline tuple format
    let klines: Vec<Kline> = candles
        .into_iter()
        .map(|c| {
            let open_time = c.bucket.timestamp_millis();
            let close_time = open_time + interval_duration.num_milliseconds() - 1;

            (
                open_time,
                c.open.to_string(),
                c.high.to_string(),
                c.low.to_string(),
                c.close.to_string(),
                c.volume.to_string(),
                close_time,
                c.quote_volume.to_string(),
                c.trade_count,
                (c.volume / rust_decimal::Decimal::TWO).to_string(),
                (c.quote_volume / rust_decimal::Decimal::TWO).to_string(),
                "0".to_string(),
            )
        })
        .collect();

    Ok(Json(klines))
}

/// Get 24hr ticker statistics
#[utoipa::path(
    get,
    path = "/api/v1/ticker/24hr",
    tag = "Market Data",
    params(
        ("symbol" = Option<String>, Query, description = "Symbol"),
        ("symbols" = Option<Vec<String>>, Query, description = "Symbols array")
    ),
    responses(
        (status = 200, description = "24hr ticker", body = Vec<Ticker24hr>),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_ticker_24hr(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TickerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // Track if single symbol was requested
    let single_symbol = query.symbol.is_some();

    // Build list of symbols to query
    let symbols: Vec<String> = if let Some(symbol) = query.symbol {
        vec![symbol]
    } else if let Some(symbols) = query.symbols {
        symbols
    } else {
        // Get all active markets
        state.db.market_repo()
            .list_active()
            .await
            .map_err(ApiError::from)?
            .into_iter()
            .map(|m| m.id)
            .collect()
    };

    let mut tickers: Vec<Ticker24hr> = vec![];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    for symbol in symbols {
        // Get market info
        let market = state.db.market_repo()
            .find_by_id(&symbol)
            .await
            .map_err(ApiError::from)?;

        if market.is_none() {
            continue;
        }

        // Get 24h volume
        let (volume, quote_volume) = state.db.trade_repo()
            .get_24h_volume(&symbol)
            .await
            .map_err(ApiError::from)?;

        // Get latest candle for price info
        let latest_candle = state.db.candle_repo()
            .get_latest(&symbol, "1h")
            .await
            .map_err(ApiError::from)?;

        let (last_price, high_price, low_price, open_price) = if let Some(candle) = latest_candle {
            (
                candle.close.to_string(),
                candle.high.to_string(),
                candle.low.to_string(),
                candle.open.to_string(),
            )
        } else {
            ("0".to_string(), "0".to_string(), "0".to_string(), "0".to_string())
        };

        // Calculate price change
        let last: f64 = last_price.parse().unwrap_or(0.0);
        let open: f64 = open_price.parse().unwrap_or(0.0);
        let price_change = last - open;
        let price_change_percent = if open > 0.0 { (price_change / open) * 100.0 } else { 0.0 };

        tickers.push(Ticker24hr {
            symbol: symbol.clone(),
            price_change: format!("{:.8}", price_change),
            price_change_percent: format!("{:.2}", price_change_percent),
            weighted_avg_price: last_price.clone(),
            prev_close_price: open_price.clone(),
            last_price: last_price.clone(),
            last_qty: "0".to_string(),
            bid_price: "0".to_string(),
            bid_qty: "0".to_string(),
            ask_price: "0".to_string(),
            ask_qty: "0".to_string(),
            open_price,
            high_price,
            low_price,
            volume: volume.to_string(),
            quote_volume: quote_volume.to_string(),
            open_time: now - 86400000,
            close_time: now,
            first_id: 0,
            last_id: 0,
            count: 0,
        });
    }

    // Return single object if one symbol, array otherwise
    if tickers.len() == 1 && single_symbol {
        Ok(Json(serde_json::to_value(&tickers[0]).unwrap_or_default()))
    } else {
        Ok(Json(serde_json::to_value(&tickers).unwrap_or_default()))
    }
}

/// Get price ticker
#[utoipa::path(
    get,
    path = "/api/v1/ticker/price",
    tag = "Market Data",
    params(
        ("symbol" = Option<String>, Query, description = "Symbol")
    ),
    responses(
        (status = 200, description = "Price ticker", body = Vec<PriceTicker>),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_price_ticker(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TickerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // Track if single symbol was requested
    let single_symbol = query.symbol.is_some();

    // Build list of symbols to query
    let symbols: Vec<String> = if let Some(symbol) = query.symbol {
        vec![symbol]
    } else if let Some(symbols) = query.symbols {
        symbols
    } else {
        state.db.market_repo()
            .list_active()
            .await
            .map_err(ApiError::from)?
            .into_iter()
            .map(|m| m.id)
            .collect()
    };

    let mut tickers: Vec<PriceTicker> = vec![];

    for symbol in symbols {
        // Get latest candle for price
        let latest_candle = state.db.candle_repo()
            .get_latest(&symbol, "1h")
            .await
            .map_err(ApiError::from)?;

        let price = if let Some(candle) = latest_candle {
            candle.close.to_string()
        } else {
            "0".to_string()
        };

        tickers.push(PriceTicker {
            symbol: symbol.clone(),
            price,
        });
    }

    // Return single object if one symbol, array otherwise
    if tickers.len() == 1 && single_symbol {
        Ok(Json(serde_json::to_value(&tickers[0]).unwrap_or_default()))
    } else {
        Ok(Json(serde_json::to_value(&tickers).unwrap_or_default()))
    }
}

/// Get book ticker (best bid/ask)
#[utoipa::path(
    get,
    path = "/api/v1/ticker/bookTicker",
    tag = "Market Data",
    params(
        ("symbol" = Option<String>, Query, description = "Symbol")
    ),
    responses(
        (status = 200, description = "Book ticker", body = Vec<BookTicker>),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_book_ticker(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TickerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // Track if single symbol was requested
    let single_symbol = query.symbol.is_some();

    // Build list of symbols to query
    let symbols: Vec<String> = if let Some(symbol) = query.symbol {
        vec![symbol]
    } else if let Some(symbols) = query.symbols {
        symbols
    } else {
        state.db.market_repo()
            .list_active()
            .await
            .map_err(ApiError::from)?
            .into_iter()
            .map(|m| m.id)
            .collect()
    };

    let mut tickers: Vec<BookTicker> = vec![];

    for symbol in symbols {
        // Get open orders for this market
        let orders = state.db.order_repo()
            .find_open_by_market(&symbol)
            .await
            .map_err(ApiError::from)?;

        // Get best bid (highest buy order)
        let (bid_price, bid_qty) = orders
            .iter()
            .filter(|o| o.side == "buy" && o.price.is_some())
            .max_by(|a, b| a.price.cmp(&b.price))
            .map(|o| (o.price.unwrap().to_string(), o.remaining.to_string()))
            .unwrap_or(("0".to_string(), "0".to_string()));

        // Get best ask (lowest sell order)
        let (ask_price, ask_qty) = orders
            .iter()
            .filter(|o| o.side == "sell" && o.price.is_some())
            .min_by(|a, b| a.price.cmp(&b.price))
            .map(|o| (o.price.unwrap().to_string(), o.remaining.to_string()))
            .unwrap_or(("0".to_string(), "0".to_string()));

        tickers.push(BookTicker {
            symbol: symbol.clone(),
            bid_price,
            bid_qty,
            ask_price,
            ask_qty,
        });
    }

    // Return single object if one symbol, array otherwise
    if tickers.len() == 1 && single_symbol {
        Ok(Json(serde_json::to_value(&tickers[0]).unwrap_or_default()))
    } else {
        Ok(Json(serde_json::to_value(&tickers).unwrap_or_default()))
    }
}

/// Get average price
#[utoipa::path(
    get,
    path = "/api/v1/avgPrice",
    tag = "Market Data",
    params(
        ("symbol" = String, Query, description = "Symbol")
    ),
    responses(
        (status = 200, description = "Average price", body = AvgPrice),
        (status = 400, description = "Invalid symbol")
    )
)]
pub async fn get_avg_price(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AvgPriceQuery>,
) -> ApiResult<Json<AvgPrice>> {
    // Validate market exists
    let market = state.db.market_repo()
        .find_by_id(&query.symbol)
        .await
        .map_err(ApiError::from)?;

    if market.is_none() {
        return Err(ApiError::InvalidSymbol(query.symbol));
    }

    // Get candles for last 5 minutes
    let end_time = Utc::now();
    let start_time = end_time - Duration::minutes(5);

    let candles = state.db.candle_repo()
        .get_candles(&query.symbol, "1m", start_time, end_time, 5)
        .await
        .map_err(ApiError::from)?;

    // Calculate volume-weighted average price
    let (total_value, total_volume): (rust_decimal::Decimal, rust_decimal::Decimal) = candles
        .iter()
        .map(|c| (c.close * c.volume, c.volume))
        .fold(
            (rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO),
            |(sum_val, sum_vol), (val, vol)| (sum_val + val, sum_vol + vol),
        );

    let avg_price = if total_volume > rust_decimal::Decimal::ZERO {
        total_value / total_volume
    } else {
        // Fall back to latest candle close price
        candles.first().map(|c| c.close).unwrap_or(rust_decimal::Decimal::ZERO)
    };

    let close_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Ok(Json(AvgPrice {
        mins: 5,
        price: avg_price.to_string(),
        close_time,
    }))
}

// Re-export SymbolFilter with Default impl
impl Default for SymbolFilter {
    fn default() -> Self {
        Self {
            filter_type: String::new(),
            min_price: None,
            max_price: None,
            tick_size: None,
            min_qty: None,
            max_qty: None,
            step_size: None,
            limit: None,
            min_notional: None,
            apply_to_market: None,
            avg_price_mins: None,
            max_num_orders: None,
            max_num_algo_orders: None,
            max_position: None,
            multiplier_up: None,
            multiplier_down: None,
            multiplier_decimal: None,
        }
    }
}
