//! Wallet and balance repository

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbResult, DbError, DbWallet, DbBalance, DbBalanceChange};

/// Wallet repository for managing balances
pub struct WalletRepo {
    pool: PgPool,
}

impl WalletRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new wallet
    pub async fn create(
        &self,
        user_id: Uuid,
        wallet_type: &str,
        agent_id: Option<Uuid>,
    ) -> DbResult<DbWallet> {
        let wallet = sqlx::query_as::<_, DbWallet>(
            r#"
            INSERT INTO wallets (user_id, wallet_type, agent_id)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, wallet_type, agent_id, created_at
            "#
        )
        .bind(user_id)
        .bind(wallet_type)
        .bind(agent_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(wallet)
    }

    /// Find wallet by ID
    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbWallet>> {
        let wallet = sqlx::query_as::<_, DbWallet>(
            "SELECT id, user_id, wallet_type, agent_id, created_at FROM wallets WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(wallet)
    }

    /// Find user's spot wallet
    pub async fn find_spot_wallet(&self, user_id: Uuid) -> DbResult<Option<DbWallet>> {
        let wallet = sqlx::query_as::<_, DbWallet>(
            r#"
            SELECT id, user_id, wallet_type, agent_id, created_at
            FROM wallets
            WHERE user_id = $1 AND wallet_type = 'spot' AND agent_id IS NULL
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(wallet)
    }

    /// Find user's wallet by type
    pub async fn find_by_type(
        &self,
        user_id: Uuid,
        wallet_type: &str,
    ) -> DbResult<Option<DbWallet>> {
        let wallet = sqlx::query_as::<_, DbWallet>(
            r#"
            SELECT id, user_id, wallet_type, agent_id, created_at
            FROM wallets
            WHERE user_id = $1 AND wallet_type = $2 AND agent_id IS NULL
            "#
        )
        .bind(user_id)
        .bind(wallet_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(wallet)
    }

    /// List all wallets for user
    pub async fn list_by_user(&self, user_id: Uuid) -> DbResult<Vec<DbWallet>> {
        let wallets = sqlx::query_as::<_, DbWallet>(
            "SELECT id, user_id, wallet_type, agent_id, created_at FROM wallets WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(wallets)
    }

    // =========================================================================
    // Balance Operations
    // =========================================================================

    /// Get balance for a currency in a wallet
    pub async fn get_balance(&self, wallet_id: Uuid, currency: &str) -> DbResult<DbBalance> {
        let balance = sqlx::query_as::<_, DbBalance>(
            r#"
            SELECT wallet_id, currency, available, locked, updated_at
            FROM balances
            WHERE wallet_id = $1 AND currency = $2
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .fetch_optional(&self.pool)
        .await?;

        Ok(balance.unwrap_or_else(|| DbBalance {
            wallet_id,
            currency: currency.to_string(),
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            updated_at: chrono::Utc::now(),
        }))
    }

    /// Get all balances for a wallet
    pub async fn get_all_balances(&self, wallet_id: Uuid) -> DbResult<Vec<DbBalance>> {
        let balances = sqlx::query_as::<_, DbBalance>(
            "SELECT wallet_id, currency, available, locked, updated_at FROM balances WHERE wallet_id = $1"
        )
        .bind(wallet_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(balances)
    }

    /// Credit balance (deposit, trade receive, etc.)
    /// Uses UPSERT to handle first-time balance creation
    pub async fn credit(
        &self,
        wallet_id: Uuid,
        currency: &str,
        amount: Decimal,
        change_type: &str,
        reference_type: Option<&str>,
        reference_id: Option<Uuid>,
        receipt_id: Option<Uuid>,
    ) -> DbResult<DbBalance> {
        if amount <= Decimal::ZERO {
            return Err(DbError::InvalidInput("Credit amount must be positive".to_string()));
        }

        let mut tx = self.pool.begin().await?;

        // Get current balance (or 0 if not exists)
        let current = sqlx::query_as::<_, DbBalance>(
            "SELECT wallet_id, currency, available, locked, updated_at FROM balances WHERE wallet_id = $1 AND currency = $2 FOR UPDATE"
        )
        .bind(wallet_id)
        .bind(currency)
        .fetch_optional(&mut *tx)
        .await?;

        let balance_before = current.as_ref().map(|b| b.available).unwrap_or(Decimal::ZERO);
        let balance_after = balance_before + amount;

        // Upsert balance
        let balance = sqlx::query_as::<_, DbBalance>(
            r#"
            INSERT INTO balances (wallet_id, currency, available, locked)
            VALUES ($1, $2, $3, 0)
            ON CONFLICT (wallet_id, currency)
            DO UPDATE SET available = balances.available + $3, updated_at = NOW()
            RETURNING wallet_id, currency, available, locked, updated_at
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await?;

        // Record balance change
        sqlx::query(
            r#"
            INSERT INTO balance_changes
                (wallet_id, currency, change_type, amount, balance_before, balance_after, reference_type, reference_id, receipt_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(change_type)
        .bind(amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(reference_type)
        .bind(reference_id)
        .bind(receipt_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(balance)
    }

    /// Debit balance (withdrawal, trade spend, etc.)
    pub async fn debit(
        &self,
        wallet_id: Uuid,
        currency: &str,
        amount: Decimal,
        change_type: &str,
        reference_type: Option<&str>,
        reference_id: Option<Uuid>,
        receipt_id: Option<Uuid>,
    ) -> DbResult<DbBalance> {
        if amount <= Decimal::ZERO {
            return Err(DbError::InvalidInput("Debit amount must be positive".to_string()));
        }

        let mut tx = self.pool.begin().await?;

        // Get current balance with lock
        let current = sqlx::query_as::<_, DbBalance>(
            "SELECT wallet_id, currency, available, locked, updated_at FROM balances WHERE wallet_id = $1 AND currency = $2 FOR UPDATE"
        )
        .bind(wallet_id)
        .bind(currency)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| DbError::InsufficientBalance(format!("No {} balance", currency)))?;

        if current.available < amount {
            return Err(DbError::InsufficientBalance(format!(
                "Insufficient {}: have {}, need {}",
                currency, current.available, amount
            )));
        }

        let balance_before = current.available;
        let balance_after = balance_before - amount;

        // Update balance
        let balance = sqlx::query_as::<_, DbBalance>(
            r#"
            UPDATE balances
            SET available = available - $3, updated_at = NOW()
            WHERE wallet_id = $1 AND currency = $2
            RETURNING wallet_id, currency, available, locked, updated_at
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await?;

        // Record balance change (negative amount)
        sqlx::query(
            r#"
            INSERT INTO balance_changes
                (wallet_id, currency, change_type, amount, balance_before, balance_after, reference_type, reference_id, receipt_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(change_type)
        .bind(-amount)  // Negative for debit
        .bind(balance_before)
        .bind(balance_after)
        .bind(reference_type)
        .bind(reference_id)
        .bind(receipt_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(balance)
    }

    /// Lock balance (for pending orders)
    pub async fn lock(
        &self,
        wallet_id: Uuid,
        currency: &str,
        amount: Decimal,
    ) -> DbResult<DbBalance> {
        if amount <= Decimal::ZERO {
            return Err(DbError::InvalidInput("Lock amount must be positive".to_string()));
        }

        let mut tx = self.pool.begin().await?;

        // Get current balance with lock
        let current = sqlx::query_as::<_, DbBalance>(
            "SELECT wallet_id, currency, available, locked, updated_at FROM balances WHERE wallet_id = $1 AND currency = $2 FOR UPDATE"
        )
        .bind(wallet_id)
        .bind(currency)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| DbError::InsufficientBalance(format!("No {} balance", currency)))?;

        if current.available < amount {
            return Err(DbError::InsufficientBalance(format!(
                "Insufficient {} to lock: have {}, need {}",
                currency, current.available, amount
            )));
        }

        // Move from available to locked
        let balance = sqlx::query_as::<_, DbBalance>(
            r#"
            UPDATE balances
            SET available = available - $3, locked = locked + $3, updated_at = NOW()
            WHERE wallet_id = $1 AND currency = $2
            RETURNING wallet_id, currency, available, locked, updated_at
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(balance)
    }

    /// Unlock balance (when order cancelled)
    pub async fn unlock(
        &self,
        wallet_id: Uuid,
        currency: &str,
        amount: Decimal,
    ) -> DbResult<DbBalance> {
        if amount <= Decimal::ZERO {
            return Err(DbError::InvalidInput("Unlock amount must be positive".to_string()));
        }

        let balance = sqlx::query_as::<_, DbBalance>(
            r#"
            UPDATE balances
            SET available = available + $3, locked = locked - $3, updated_at = NOW()
            WHERE wallet_id = $1 AND currency = $2 AND locked >= $3
            RETURNING wallet_id, currency, available, locked, updated_at
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(amount)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::InvalidInput("Not enough locked balance".to_string()))?;

        Ok(balance)
    }

    /// Consume locked balance (when order fills)
    pub async fn consume_locked(
        &self,
        wallet_id: Uuid,
        currency: &str,
        amount: Decimal,
        change_type: &str,
        reference_type: Option<&str>,
        reference_id: Option<Uuid>,
        receipt_id: Option<Uuid>,
    ) -> DbResult<DbBalance> {
        if amount <= Decimal::ZERO {
            return Err(DbError::InvalidInput("Consume amount must be positive".to_string()));
        }

        let mut tx = self.pool.begin().await?;

        // Get current balance
        let current = sqlx::query_as::<_, DbBalance>(
            "SELECT wallet_id, currency, available, locked, updated_at FROM balances WHERE wallet_id = $1 AND currency = $2 FOR UPDATE"
        )
        .bind(wallet_id)
        .bind(currency)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| DbError::InsufficientBalance(format!("No {} balance", currency)))?;

        if current.locked < amount {
            return Err(DbError::InsufficientBalance(format!(
                "Insufficient locked {}: have {}, need {}",
                currency, current.locked, amount
            )));
        }

        let balance_before = current.available + current.locked;
        let balance_after = balance_before - amount;

        // Reduce locked balance
        let balance = sqlx::query_as::<_, DbBalance>(
            r#"
            UPDATE balances
            SET locked = locked - $3, updated_at = NOW()
            WHERE wallet_id = $1 AND currency = $2
            RETURNING wallet_id, currency, available, locked, updated_at
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await?;

        // Record balance change
        sqlx::query(
            r#"
            INSERT INTO balance_changes
                (wallet_id, currency, change_type, amount, balance_before, balance_after, reference_type, reference_id, receipt_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(wallet_id)
        .bind(currency)
        .bind(change_type)
        .bind(-amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(reference_type)
        .bind(reference_id)
        .bind(receipt_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(balance)
    }

    /// Get balance change history
    pub async fn get_balance_history(
        &self,
        wallet_id: Uuid,
        currency: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<DbBalanceChange>> {
        let changes = if let Some(curr) = currency {
            sqlx::query_as::<_, DbBalanceChange>(
                r#"
                SELECT id, wallet_id, currency, change_type, amount, balance_before, balance_after,
                       reference_type, reference_id, receipt_id, created_at
                FROM balance_changes
                WHERE wallet_id = $1 AND currency = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#
            )
            .bind(wallet_id)
            .bind(curr)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbBalanceChange>(
                r#"
                SELECT id, wallet_id, currency, change_type, amount, balance_before, balance_after,
                       reference_type, reference_id, receipt_id, created_at
                FROM balance_changes
                WHERE wallet_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#
            )
            .bind(wallet_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(changes)
    }
}
