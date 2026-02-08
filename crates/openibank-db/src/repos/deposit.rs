//! Deposit repository

use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbResult, DbError, DbDeposit, DbDepositAddress};

pub struct DepositRepo {
    pool: PgPool,
}

impl DepositRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Deposit Address Operations
    // =========================================================================

    pub async fn create_address(&self, address: &DbDepositAddress) -> DbResult<DbDepositAddress> {
        let addr = sqlx::query_as::<_, DbDepositAddress>(
            r#"
            INSERT INTO deposit_addresses (user_id, currency, network, address, memo)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(address.user_id)
        .bind(&address.currency)
        .bind(&address.network)
        .bind(&address.address)
        .bind(&address.memo)
        .fetch_one(&self.pool)
        .await?;
        Ok(addr)
    }

    pub async fn find_address(&self, user_id: Uuid, currency: &str, network: &str) -> DbResult<Option<DbDepositAddress>> {
        let addr = sqlx::query_as::<_, DbDepositAddress>(
            "SELECT * FROM deposit_addresses WHERE user_id = $1 AND currency = $2 AND network = $3"
        )
        .bind(user_id)
        .bind(currency)
        .bind(network)
        .fetch_optional(&self.pool)
        .await?;
        Ok(addr)
    }

    pub async fn find_by_address(&self, address: &str, network: &str) -> DbResult<Option<DbDepositAddress>> {
        let addr = sqlx::query_as::<_, DbDepositAddress>(
            "SELECT * FROM deposit_addresses WHERE address = $1 AND network = $2"
        )
        .bind(address)
        .bind(network)
        .fetch_optional(&self.pool)
        .await?;
        Ok(addr)
    }

    pub async fn list_user_addresses(&self, user_id: Uuid) -> DbResult<Vec<DbDepositAddress>> {
        let addresses = sqlx::query_as::<_, DbDepositAddress>(
            "SELECT * FROM deposit_addresses WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(addresses)
    }

    // =========================================================================
    // Deposit Operations
    // =========================================================================

    pub async fn create(&self, deposit: &DbDeposit) -> DbResult<DbDeposit> {
        let d = sqlx::query_as::<_, DbDeposit>(
            r#"
            INSERT INTO deposits (id, user_id, wallet_id, currency, network, amount, tx_hash, from_address,
                confirmations, required_confirmations, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#
        )
        .bind(deposit.id)
        .bind(deposit.user_id)
        .bind(deposit.wallet_id)
        .bind(&deposit.currency)
        .bind(&deposit.network)
        .bind(&deposit.amount)
        .bind(&deposit.tx_hash)
        .bind(&deposit.from_address)
        .bind(deposit.confirmations)
        .bind(deposit.required_confirmations)
        .bind(&deposit.status)
        .fetch_one(&self.pool)
        .await?;
        Ok(d)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbDeposit>> {
        let deposit = sqlx::query_as::<_, DbDeposit>("SELECT * FROM deposits WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(deposit)
    }

    pub async fn find_by_tx_hash(&self, tx_hash: &str, network: &str) -> DbResult<Option<DbDeposit>> {
        let deposit = sqlx::query_as::<_, DbDeposit>(
            "SELECT * FROM deposits WHERE tx_hash = $1 AND network = $2"
        )
        .bind(tx_hash)
        .bind(network)
        .fetch_optional(&self.pool)
        .await?;
        Ok(deposit)
    }

    pub async fn list_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> DbResult<Vec<DbDeposit>> {
        let deposits = sqlx::query_as::<_, DbDeposit>(
            "SELECT * FROM deposits WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(deposits)
    }

    pub async fn list_pending(&self) -> DbResult<Vec<DbDeposit>> {
        let deposits = sqlx::query_as::<_, DbDeposit>(
            "SELECT * FROM deposits WHERE status IN ('pending', 'confirming') ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(deposits)
    }

    pub async fn update_confirmations(&self, id: Uuid, confirmations: i32) -> DbResult<()> {
        sqlx::query("UPDATE deposits SET confirmations = $2 WHERE id = $1")
            .bind(id)
            .bind(confirmations)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> DbResult<()> {
        if status == "completed" {
            sqlx::query("UPDATE deposits SET status = $2, credited_at = NOW() WHERE id = $1")
                .bind(id)
                .bind(status)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("UPDATE deposits SET status = $2 WHERE id = $1")
                .bind(id)
                .bind(status)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn complete(&self, id: Uuid, receipt_id: Uuid) -> DbResult<DbDeposit> {
        let deposit = sqlx::query_as::<_, DbDeposit>(
            "UPDATE deposits SET status = 'completed', credited_at = NOW(), receipt_id = $2 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(receipt_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("Deposit not found".to_string()))?;
        Ok(deposit)
    }
}
