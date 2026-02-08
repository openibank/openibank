//! Order repository

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbResult, DbError, DbOrder};

pub struct OrderRepo {
    pool: PgPool,
}

impl OrderRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, order: &DbOrder) -> DbResult<DbOrder> {
        let o = sqlx::query_as::<_, DbOrder>(
            r#"
            INSERT INTO rx_orders (id, user_id, market_id, client_order_id, side, order_type,
                price, stop_price, amount, filled, remaining, status, time_in_force, post_only, reduce_only)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#
        )
        .bind(order.id)
        .bind(order.user_id)
        .bind(&order.market_id)
        .bind(&order.client_order_id)
        .bind(&order.side)
        .bind(&order.order_type)
        .bind(&order.price)
        .bind(&order.stop_price)
        .bind(&order.amount)
        .bind(&order.filled)
        .bind(&order.remaining)
        .bind(&order.status)
        .bind(&order.time_in_force)
        .bind(order.post_only)
        .bind(order.reduce_only)
        .fetch_one(&self.pool)
        .await?;
        Ok(o)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbOrder>> {
        let order = sqlx::query_as::<_, DbOrder>("SELECT * FROM rx_orders WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(order)
    }

    pub async fn find_open_by_market(&self, market_id: &str) -> DbResult<Vec<DbOrder>> {
        let orders = sqlx::query_as::<_, DbOrder>(
            "SELECT * FROM rx_orders WHERE market_id = $1 AND status IN ('new', 'partially_filled') ORDER BY created_at"
        )
        .bind(market_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }

    pub async fn find_open_by_user(&self, user_id: Uuid, market_id: Option<&str>) -> DbResult<Vec<DbOrder>> {
        let orders = if let Some(m) = market_id {
            sqlx::query_as::<_, DbOrder>(
                "SELECT * FROM rx_orders WHERE user_id = $1 AND market_id = $2 AND status IN ('new', 'partially_filled') ORDER BY created_at DESC"
            )
            .bind(user_id)
            .bind(m)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbOrder>(
                "SELECT * FROM rx_orders WHERE user_id = $1 AND status IN ('new', 'partially_filled') ORDER BY created_at DESC"
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(orders)
    }

    pub async fn find_by_user(&self, user_id: Uuid, market_id: Option<&str>, limit: i64, offset: i64) -> DbResult<Vec<DbOrder>> {
        let orders = if let Some(m) = market_id {
            sqlx::query_as::<_, DbOrder>(
                "SELECT * FROM rx_orders WHERE user_id = $1 AND market_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(user_id)
            .bind(m)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbOrder>(
                "SELECT * FROM rx_orders WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(orders)
    }

    pub async fn update_fill(&self, id: Uuid, filled: Decimal, remaining: Decimal, quote_filled: Decimal, fee_total: Decimal, status: &str) -> DbResult<()> {
        sqlx::query(
            "UPDATE rx_orders SET filled = $2, remaining = $3, quote_filled = $4, fee_total = $5, status = $6 WHERE id = $1"
        )
        .bind(id)
        .bind(filled)
        .bind(remaining)
        .bind(quote_filled)
        .bind(fee_total)
        .bind(status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn cancel(&self, id: Uuid) -> DbResult<DbOrder> {
        let order = sqlx::query_as::<_, DbOrder>(
            "UPDATE rx_orders SET status = 'cancelled' WHERE id = $1 AND status IN ('new', 'partially_filled') RETURNING *"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Order not found or already complete".to_string()))?;
        Ok(order)
    }

    pub async fn cancel_all_by_user(&self, user_id: Uuid, market_id: Option<&str>) -> DbResult<u64> {
        let result = if let Some(m) = market_id {
            sqlx::query(
                "UPDATE rx_orders SET status = 'cancelled' WHERE user_id = $1 AND market_id = $2 AND status IN ('new', 'partially_filled')"
            )
            .bind(user_id)
            .bind(m)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                "UPDATE rx_orders SET status = 'cancelled' WHERE user_id = $1 AND status IN ('new', 'partially_filled')"
            )
            .bind(user_id)
            .execute(&self.pool)
            .await?
        };
        Ok(result.rows_affected())
    }
}
