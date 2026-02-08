//! Candle repository for OHLCV data

use sqlx::PgPool;
use chrono::{DateTime, Utc};

use crate::{DbResult, DbCandle};

pub struct CandleRepo {
    pool: PgPool,
}

impl CandleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, candle: &DbCandle) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO rx_candles (market_id, interval, bucket, open, high, low, close, volume, quote_volume, trade_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (market_id, interval, bucket)
            DO UPDATE SET high = GREATEST(rx_candles.high, EXCLUDED.high),
                          low = LEAST(rx_candles.low, EXCLUDED.low),
                          close = EXCLUDED.close,
                          volume = rx_candles.volume + EXCLUDED.volume,
                          quote_volume = rx_candles.quote_volume + EXCLUDED.quote_volume,
                          trade_count = rx_candles.trade_count + EXCLUDED.trade_count
            "#
        )
        .bind(&candle.market_id)
        .bind(&candle.interval)
        .bind(candle.bucket)
        .bind(&candle.open)
        .bind(&candle.high)
        .bind(&candle.low)
        .bind(&candle.close)
        .bind(&candle.volume)
        .bind(&candle.quote_volume)
        .bind(candle.trade_count)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_candles(
        &self,
        market_id: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
    ) -> DbResult<Vec<DbCandle>> {
        let candles = sqlx::query_as::<_, DbCandle>(
            r#"
            SELECT market_id, interval, bucket, open, high, low, close, volume, quote_volume, trade_count
            FROM rx_candles
            WHERE market_id = $1 AND interval = $2 AND bucket >= $3 AND bucket <= $4
            ORDER BY bucket DESC
            LIMIT $5
            "#
        )
        .bind(market_id)
        .bind(interval)
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(candles)
    }

    pub async fn get_latest(&self, market_id: &str, interval: &str) -> DbResult<Option<DbCandle>> {
        let candle = sqlx::query_as::<_, DbCandle>(
            "SELECT * FROM rx_candles WHERE market_id = $1 AND interval = $2 ORDER BY bucket DESC LIMIT 1"
        )
        .bind(market_id)
        .bind(interval)
        .fetch_optional(&self.pool)
        .await?;
        Ok(candle)
    }
}
