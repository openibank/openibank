//! Trade repository

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{DbResult, DbTrade};

pub struct TradeRepo {
    pool: PgPool,
}

impl TradeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, trade: &DbTrade) -> DbResult<DbTrade> {
        let t = sqlx::query_as::<_, DbTrade>(
            r#"
            INSERT INTO rx_trades (id, market_id, price, amount, quote_amount, maker_order_id, taker_order_id,
                maker_user_id, taker_user_id, maker_fee, taker_fee, maker_fee_currency, taker_fee_currency, is_buyer_maker)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#
        )
        .bind(trade.id)
        .bind(&trade.market_id)
        .bind(&trade.price)
        .bind(&trade.amount)
        .bind(&trade.quote_amount)
        .bind(trade.maker_order_id)
        .bind(trade.taker_order_id)
        .bind(trade.maker_user_id)
        .bind(trade.taker_user_id)
        .bind(&trade.maker_fee)
        .bind(&trade.taker_fee)
        .bind(&trade.maker_fee_currency)
        .bind(&trade.taker_fee_currency)
        .bind(trade.is_buyer_maker)
        .fetch_one(&self.pool)
        .await?;
        Ok(t)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbTrade>> {
        let trade = sqlx::query_as::<_, DbTrade>("SELECT * FROM rx_trades WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(trade)
    }

    pub async fn find_recent_by_market(&self, market_id: &str, limit: i64) -> DbResult<Vec<DbTrade>> {
        let trades = sqlx::query_as::<_, DbTrade>(
            "SELECT * FROM rx_trades WHERE market_id = $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(market_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(trades)
    }

    pub async fn find_by_user(&self, user_id: Uuid, market_id: Option<&str>, limit: i64, offset: i64) -> DbResult<Vec<DbTrade>> {
        let trades = if let Some(m) = market_id {
            sqlx::query_as::<_, DbTrade>(
                "SELECT * FROM rx_trades WHERE (maker_user_id = $1 OR taker_user_id = $1) AND market_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(user_id)
            .bind(m)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbTrade>(
                "SELECT * FROM rx_trades WHERE maker_user_id = $1 OR taker_user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(trades)
    }

    pub async fn get_24h_volume(&self, market_id: &str) -> DbResult<(Decimal, Decimal)> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(amount), 0) as volume, COALESCE(SUM(quote_amount), 0) as quote_volume
            FROM rx_trades
            WHERE market_id = $1 AND created_at > NOW() - INTERVAL '24 hours'
            "#
        )
        .bind(market_id)
        .fetch_one(&self.pool)
        .await?;

        let volume: Decimal = row.try_get("volume")?;
        let quote_volume: Decimal = row.try_get("quote_volume")?;
        Ok((volume, quote_volume))
    }
}
