//! Receipt repository for immutable transaction records (cryptographic proofs)

use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbResult, DbReceipt};

pub struct ReceiptRepo {
    pool: PgPool,
}

impl ReceiptRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, receipt: &DbReceipt) -> DbResult<DbReceipt> {
        let r = sqlx::query_as::<_, DbReceipt>(
            r#"
            INSERT INTO receipts (id, receipt_type, commitment_id, user_id, payload, payload_hash,
                signature, signer_public_key, chain_proof)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(receipt.id)
        .bind(&receipt.receipt_type)
        .bind(receipt.commitment_id)
        .bind(receipt.user_id)
        .bind(&receipt.payload)
        .bind(&receipt.payload_hash)
        .bind(&receipt.signature)
        .bind(&receipt.signer_public_key)
        .bind(&receipt.chain_proof)
        .fetch_one(&self.pool)
        .await?;
        Ok(r)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbReceipt>> {
        let receipt = sqlx::query_as::<_, DbReceipt>("SELECT * FROM receipts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(receipt)
    }

    pub async fn list_by_user(
        &self,
        user_id: Uuid,
        receipt_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<DbReceipt>> {
        let receipts = if let Some(rt) = receipt_type {
            sqlx::query_as::<_, DbReceipt>(
                "SELECT * FROM receipts WHERE user_id = $1 AND receipt_type = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(user_id)
            .bind(rt)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbReceipt>(
                "SELECT * FROM receipts WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(receipts)
    }

    pub async fn find_by_commitment(&self, commitment_id: Uuid) -> DbResult<Option<DbReceipt>> {
        let receipt = sqlx::query_as::<_, DbReceipt>(
            "SELECT * FROM receipts WHERE commitment_id = $1"
        )
        .bind(commitment_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(receipt)
    }

    pub async fn list_by_type(&self, receipt_type: &str, limit: i64, offset: i64) -> DbResult<Vec<DbReceipt>> {
        let receipts = sqlx::query_as::<_, DbReceipt>(
            "SELECT * FROM receipts WHERE receipt_type = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(receipt_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(receipts)
    }
}
