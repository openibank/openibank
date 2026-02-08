//! Withdrawal repository

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{DbResult, DbError, DbWithdrawal};

pub struct WithdrawalRepo {
    pool: PgPool,
}

impl WithdrawalRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, withdrawal: &DbWithdrawal) -> DbResult<DbWithdrawal> {
        let w = sqlx::query_as::<_, DbWithdrawal>(
            r#"
            INSERT INTO withdrawals (id, user_id, wallet_id, currency, network, amount, fee, to_address, memo, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(withdrawal.id)
        .bind(withdrawal.user_id)
        .bind(withdrawal.wallet_id)
        .bind(&withdrawal.currency)
        .bind(&withdrawal.network)
        .bind(&withdrawal.amount)
        .bind(&withdrawal.fee)
        .bind(&withdrawal.to_address)
        .bind(&withdrawal.memo)
        .bind(&withdrawal.status)
        .fetch_one(&self.pool)
        .await?;
        Ok(w)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbWithdrawal>> {
        let withdrawal = sqlx::query_as::<_, DbWithdrawal>("SELECT * FROM withdrawals WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(withdrawal)
    }

    pub async fn list_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> DbResult<Vec<DbWithdrawal>> {
        let withdrawals = sqlx::query_as::<_, DbWithdrawal>(
            "SELECT * FROM withdrawals WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(withdrawals)
    }

    pub async fn list_pending(&self) -> DbResult<Vec<DbWithdrawal>> {
        let withdrawals = sqlx::query_as::<_, DbWithdrawal>(
            "SELECT * FROM withdrawals WHERE status IN ('pending', 'awaiting_approval') ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(withdrawals)
    }

    pub async fn list_by_status(&self, status: &str, limit: i64) -> DbResult<Vec<DbWithdrawal>> {
        let withdrawals = sqlx::query_as::<_, DbWithdrawal>(
            "SELECT * FROM withdrawals WHERE status = $1 ORDER BY created_at LIMIT $2"
        )
        .bind(status)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(withdrawals)
    }

    pub async fn approve(&self, id: Uuid, approver_id: Uuid) -> DbResult<DbWithdrawal> {
        let withdrawal = sqlx::query_as::<_, DbWithdrawal>(
            "UPDATE withdrawals SET status = 'processing', approved_by = $2, approved_at = NOW() WHERE id = $1 AND status IN ('pending', 'awaiting_approval') RETURNING *"
        )
        .bind(id)
        .bind(approver_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Withdrawal not found or not pending".to_string()))?;
        Ok(withdrawal)
    }

    pub async fn process(&self, id: Uuid, tx_hash: &str) -> DbResult<DbWithdrawal> {
        let withdrawal = sqlx::query_as::<_, DbWithdrawal>(
            "UPDATE withdrawals SET tx_hash = $2 WHERE id = $1 AND status = 'processing' RETURNING *"
        )
        .bind(id)
        .bind(tx_hash)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Withdrawal not found or not processing".to_string()))?;
        Ok(withdrawal)
    }

    pub async fn complete(&self, id: Uuid, receipt_id: Uuid) -> DbResult<DbWithdrawal> {
        let withdrawal = sqlx::query_as::<_, DbWithdrawal>(
            "UPDATE withdrawals SET status = 'completed', completed_at = NOW(), receipt_id = $2 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(receipt_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Withdrawal not found".to_string()))?;
        Ok(withdrawal)
    }

    pub async fn fail(&self, id: Uuid, reason: &str) -> DbResult<DbWithdrawal> {
        let withdrawal = sqlx::query_as::<_, DbWithdrawal>(
            "UPDATE withdrawals SET status = 'failed', failure_reason = $2 WHERE id = $1 AND status IN ('pending', 'awaiting_approval', 'processing') RETURNING *"
        )
        .bind(id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Withdrawal not found or already completed".to_string()))?;
        Ok(withdrawal)
    }

    pub async fn cancel(&self, id: Uuid, user_id: Uuid) -> DbResult<DbWithdrawal> {
        let withdrawal = sqlx::query_as::<_, DbWithdrawal>(
            "UPDATE withdrawals SET status = 'cancelled' WHERE id = $1 AND user_id = $2 AND status IN ('pending', 'awaiting_approval') RETURNING *"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Withdrawal not found or cannot be cancelled".to_string()))?;
        Ok(withdrawal)
    }

    pub async fn get_daily_total(&self, user_id: Uuid, currency: &str) -> DbResult<Decimal> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(amount), 0) as total
            FROM withdrawals
            WHERE user_id = $1 AND currency = $2
              AND status NOT IN ('cancelled', 'failed')
              AND created_at > NOW() - INTERVAL '24 hours'
            "#
        )
        .bind(user_id)
        .bind(currency)
        .fetch_one(&self.pool)
        .await?;

        let total: Decimal = row.try_get("total")?;
        Ok(total)
    }
}
