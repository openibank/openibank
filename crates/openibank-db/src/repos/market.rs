//! Market repository

use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::{DbResult, DbMarket};

pub struct MarketRepo {
    pool: PgPool,
}

impl MarketRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, market: &DbMarket) -> DbResult<DbMarket> {
        let m = sqlx::query_as::<_, DbMarket>(
            r#"
            INSERT INTO rx_markets (id, base_currency, quote_currency, status, price_precision, amount_precision,
                min_amount, max_amount, min_notional, tick_size, lot_size, maker_fee, taker_fee)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#
        )
        .bind(&market.id)
        .bind(&market.base_currency)
        .bind(&market.quote_currency)
        .bind(&market.status)
        .bind(market.price_precision)
        .bind(market.amount_precision)
        .bind(&market.min_amount)
        .bind(&market.max_amount)
        .bind(&market.min_notional)
        .bind(&market.tick_size)
        .bind(&market.lot_size)
        .bind(&market.maker_fee)
        .bind(&market.taker_fee)
        .fetch_one(&self.pool)
        .await?;
        Ok(m)
    }

    pub async fn find_by_id(&self, id: &str) -> DbResult<Option<DbMarket>> {
        let market = sqlx::query_as::<_, DbMarket>("SELECT * FROM rx_markets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(market)
    }

    pub async fn list_active(&self) -> DbResult<Vec<DbMarket>> {
        let markets = sqlx::query_as::<_, DbMarket>(
            "SELECT * FROM rx_markets WHERE status = 'active' ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(markets)
    }

    pub async fn list_all(&self) -> DbResult<Vec<DbMarket>> {
        let markets = sqlx::query_as::<_, DbMarket>("SELECT * FROM rx_markets ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(markets)
    }

    pub async fn update_status(&self, id: &str, status: &str) -> DbResult<()> {
        sqlx::query("UPDATE rx_markets SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_fees(&self, id: &str, maker_fee: Decimal, taker_fee: Decimal) -> DbResult<()> {
        sqlx::query("UPDATE rx_markets SET maker_fee = $2, taker_fee = $3 WHERE id = $1")
            .bind(id)
            .bind(maker_fee)
            .bind(taker_fee)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
