//! OpeniBank Settlement - Atomic settlement execution and finality
//!
//! The settlement engine ensures atomic execution, multi-channel support,
//! receipt generation, and finality tracking.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub use openibank_types::{SettlementChannel, SettlementLeg, SettlementLegStatus};

use openibank_types::{
    Amount, BatchId, Currency, InstitutionId, OpeniBankError, ReceiptId, Result, TemporalAnchor,
};

/// Settlement batch containing multiple legs
#[derive(Debug, Clone)]
pub struct SettlementBatch {
    pub id: BatchId,
    pub legs: Vec<SettlementLeg>,
    pub state: SettlementBatchState,
    pub created_at: DateTime<Utc>,
    pub prepared_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub currency: Currency,
    pub total_volume: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementBatchState {
    Created,
    Prepared,
    Executing,
    Executed,
    Finalized,
    Failed,
}

#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub batch_id: BatchId,
    pub success: bool,
    pub executed_legs: usize,
    pub failed_legs: usize,
    pub total_volume: Amount,
    pub receipts: Vec<ReceiptId>,
    pub executed_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FinalityConfirmation {
    pub leg_id: uuid::Uuid,
    pub confirmed: bool,
    pub confirmation_time: DateTime<Utc>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
}

#[async_trait::async_trait]
pub trait SettlementChannelExecutor: Send + Sync {
    async fn execute(&self, leg: &SettlementLeg) -> Result<ReceiptId>;
    async fn check_finality(&self, receipt_id: &ReceiptId) -> Result<FinalityConfirmation>;
    async fn rollback(&self, receipt_id: &ReceiptId) -> Result<()>;
}

pub struct InMemoryChannel {
    balances: Arc<RwLock<HashMap<InstitutionId, Amount>>>,
    settlements: Arc<RwLock<HashMap<ReceiptId, SettlementLeg>>>,
}

impl InMemoryChannel {
    pub fn new() -> Self {
        Self {
            balances: Arc::new(RwLock::new(HashMap::new())),
            settlements: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set_balance(&self, institution: InstitutionId, balance: Amount) {
        self.balances.write().await.insert(institution, balance);
    }

    pub async fn get_balance(&self, institution: &InstitutionId) -> Amount {
        self.balances
            .read()
            .await
            .get(institution)
            .cloned()
            .unwrap_or_else(|| Amount::zero(Currency::iusd()))
    }
}

impl Default for InMemoryChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SettlementChannelExecutor for InMemoryChannel {
    async fn execute(&self, leg: &SettlementLeg) -> Result<ReceiptId> {
        let mut balances = self.balances.write().await;

        let from_balance = balances
            .entry(leg.from.clone())
            .or_insert_with(|| Amount::zero(leg.amount.currency))
            .clone();

        if from_balance.value < leg.amount.value {
            return Err(OpeniBankError::InsufficientFunds {
                wallet_id: leg.from.to_string(),
                requested: leg.amount.to_human(),
                available: from_balance.to_human(),
            });
        }

        balances.get_mut(&leg.from).unwrap().value -= leg.amount.value;
        balances
            .entry(leg.to.clone())
            .or_insert_with(|| Amount::zero(leg.amount.currency))
            .value += leg.amount.value;

        let receipt_id = ReceiptId::new();
        self.settlements.write().await.insert(receipt_id.clone(), leg.clone());

        info!("Settlement executed: {} from {} to {}", leg.amount, leg.from, leg.to);
        Ok(receipt_id)
    }

    async fn check_finality(&self, receipt_id: &ReceiptId) -> Result<FinalityConfirmation> {
        let settlements = self.settlements.read().await;
        if settlements.contains_key(receipt_id) {
            Ok(FinalityConfirmation {
                leg_id: uuid::Uuid::nil(),
                confirmed: true,
                confirmation_time: Utc::now(),
                block_number: None,
                tx_hash: None,
            })
        } else {
            Err(OpeniBankError::ReceiptNotFound {
                receipt_id: receipt_id.to_string(),
            })
        }
    }

    async fn rollback(&self, receipt_id: &ReceiptId) -> Result<()> {
        let mut settlements = self.settlements.write().await;
        if let Some(leg) = settlements.remove(receipt_id) {
            let mut balances = self.balances.write().await;
            balances.get_mut(&leg.to).unwrap().value -= leg.amount.value;
            balances.get_mut(&leg.from).unwrap().value += leg.amount.value;
            warn!("Settlement rolled back: {}", receipt_id);
            Ok(())
        } else {
            Err(OpeniBankError::ReceiptNotFound {
                receipt_id: receipt_id.to_string(),
            })
        }
    }
}

pub struct SettlementExecutor {
    batches: Arc<RwLock<HashMap<BatchId, SettlementBatch>>>,
    channels: Arc<RwLock<HashMap<SettlementChannel, Arc<dyn SettlementChannelExecutor>>>>,
}

impl SettlementExecutor {
    pub fn new() -> Self {
        let mut channels: HashMap<SettlementChannel, Arc<dyn SettlementChannelExecutor>> = HashMap::new();
        channels.insert(SettlementChannel::Internal, Arc::new(InMemoryChannel::new()));
        Self {
            batches: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(channels)),
        }
    }

    pub async fn register_channel(&self, channel: SettlementChannel, executor: Arc<dyn SettlementChannelExecutor>) {
        self.channels.write().await.insert(channel, executor);
    }

    pub async fn create_batch(&self, legs: Vec<SettlementLeg>, currency: Currency) -> Result<BatchId> {
        if legs.is_empty() {
            return Err(OpeniBankError::BatchNotFound {
                batch_id: "empty".to_string(),
            });
        }

        let total_volume = legs.iter().fold(Amount::zero(currency), |acc, leg| {
            Amount::new(acc.value + leg.amount.value, currency, acc.decimals)
        });

        let batch = SettlementBatch {
            id: BatchId::new(),
            legs,
            state: SettlementBatchState::Created,
            created_at: Utc::now(),
            prepared_at: None,
            executed_at: None,
            finalized_at: None,
            currency,
            total_volume,
        };

        let batch_id = batch.id.clone();
        self.batches.write().await.insert(batch_id.clone(), batch);
        Ok(batch_id)
    }

    pub async fn prepare(&self, batch_id: &BatchId) -> Result<()> {
        let mut batches = self.batches.write().await;
        let batch = batches.get_mut(batch_id).ok_or_else(|| OpeniBankError::BatchNotFound {
            batch_id: batch_id.to_string(),
        })?;

        if batch.state != SettlementBatchState::Created {
            return Err(OpeniBankError::BatchNotFound {
                batch_id: format!("Cannot prepare batch in state {:?}", batch.state),
            });
        }

        batch.state = SettlementBatchState::Prepared;
        batch.prepared_at = Some(Utc::now());
        info!("Settlement batch {} prepared with {} legs", batch_id, batch.legs.len());
        Ok(())
    }

    pub async fn execute(&self, batch_id: &BatchId) -> Result<SettlementResult> {
        {
            let mut batches = self.batches.write().await;
            let batch = batches.get_mut(batch_id).ok_or_else(|| OpeniBankError::BatchNotFound {
                batch_id: batch_id.to_string(),
            })?;

            if batch.state != SettlementBatchState::Prepared {
                return Err(OpeniBankError::BatchNotFound {
                    batch_id: format!("Cannot execute batch in state {:?}", batch.state),
                });
            }
            batch.state = SettlementBatchState::Executing;
        }

        let mut receipts = Vec::new();
        let mut executed_legs = 0;
        let failed_legs = 0;
        let mut error_message: Option<String> = None;

        let legs: Vec<SettlementLeg> = {
            let batches = self.batches.read().await;
            batches.get(batch_id).unwrap().legs.clone()
        };

        let channels = self.channels.read().await;

        for mut leg in legs {
            let channel = channels.get(&leg.channel).ok_or_else(|| OpeniBankError::BatchNotFound {
                batch_id: format!("Channel {:?} not found", leg.channel),
            })?;

            match channel.execute(&leg).await {
                Ok(receipt_id) => {
                    leg.receipt_id = Some(receipt_id.clone());
                    leg.status = SettlementLegStatus::Completed;
                    leg.executed_at = Some(TemporalAnchor::now());
                    receipts.push(receipt_id);
                    executed_legs += 1;
                }
                Err(e) => {
                    leg.status = SettlementLegStatus::Failed;
                    error_message = Some(e.to_string());

                    for receipt_id in &receipts {
                        let _ = channel.rollback(receipt_id).await;
                    }

                    let mut batches = self.batches.write().await;
                    if let Some(batch) = batches.get_mut(batch_id) {
                        batch.state = SettlementBatchState::Failed;
                    }

                    return Ok(SettlementResult {
                        batch_id: batch_id.clone(),
                        success: false,
                        executed_legs: 0,
                        failed_legs: 1,
                        total_volume: Amount::zero(Currency::iusd()),
                        receipts: vec![],
                        executed_at: Utc::now(),
                        finalized_at: None,
                        error: error_message,
                    });
                }
            }
        }

        let total_volume = {
            let mut batches = self.batches.write().await;
            let batch = batches.get_mut(batch_id).unwrap();
            batch.state = SettlementBatchState::Executed;
            batch.executed_at = Some(Utc::now());
            batch.total_volume
        };

        info!("Settlement batch {} executed: {} legs, {} volume", batch_id, executed_legs, total_volume);

        Ok(SettlementResult {
            batch_id: batch_id.clone(),
            success: true,
            executed_legs,
            failed_legs,
            total_volume,
            receipts,
            executed_at: Utc::now(),
            finalized_at: None,
            error: None,
        })
    }

    pub async fn confirm_finality(&self, batch_id: &BatchId) -> Result<Vec<FinalityConfirmation>> {
        let batch = {
            let batches = self.batches.read().await;
            batches.get(batch_id).cloned().ok_or_else(|| OpeniBankError::BatchNotFound {
                batch_id: batch_id.to_string(),
            })?
        };

        if batch.state != SettlementBatchState::Executed {
            return Err(OpeniBankError::BatchNotFound {
                batch_id: format!("Cannot confirm finality for batch in state {:?}", batch.state),
            });
        }

        let mut confirmations = Vec::new();
        let channels = self.channels.read().await;

        for leg in &batch.legs {
            if let Some(ref receipt_id) = leg.receipt_id {
                if let Some(channel) = channels.get(&leg.channel) {
                    let confirmation = channel.check_finality(receipt_id).await?;
                    confirmations.push(confirmation);
                }
            }
        }

        if confirmations.iter().all(|c| c.confirmed) {
            let mut batches = self.batches.write().await;
            if let Some(batch) = batches.get_mut(batch_id) {
                batch.state = SettlementBatchState::Finalized;
                batch.finalized_at = Some(Utc::now());
            }
            info!("Settlement batch {} finalized", batch_id);
        }

        Ok(confirmations)
    }

    pub async fn get_batch(&self, batch_id: &BatchId) -> Result<SettlementBatch> {
        self.batches.read().await.get(batch_id).cloned().ok_or_else(|| OpeniBankError::BatchNotFound {
            batch_id: batch_id.to_string(),
        })
    }

    pub async fn execute_batch_atomic(&self, legs: Vec<SettlementLeg>, currency: Currency) -> Result<SettlementResult> {
        let batch_id = self.create_batch(legs, currency).await?;
        self.prepare(&batch_id).await?;
        let result = self.execute(&batch_id).await?;
        if result.success {
            self.confirm_finality(&batch_id).await?;
        }
        Ok(result)
    }
}

impl Default for SettlementExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openibank_types::{Amount, Currency, InstitutionId, SettlementChannel, SettlementLeg, SettlementLegStatus};

    fn iusd(v: i128) -> Amount { Amount::new(v, Currency::iusd(), 18) }

    fn make_leg(from: InstitutionId, to: InstitutionId, amount: Amount) -> SettlementLeg {
        SettlementLeg {
            id: uuid::Uuid::new_v4(),
            from,
            to,
            amount,
            status: SettlementLegStatus::Pending,
            channel: SettlementChannel::Internal,
            receipt_id: None,
            executed_at: None,
        }
    }

    #[tokio::test]
    async fn single_leg_executes_successfully() {
        let executor = SettlementExecutor::new();
        let channel = InMemoryChannel::new();
        let inst_a = InstitutionId::new();
        let inst_b = InstitutionId::new();

        channel.set_balance(inst_a.clone(), iusd(1_000_000)).await;
        executor.register_channel(
            SettlementChannel::Internal,
            std::sync::Arc::new(channel),
        ).await;

        let legs = vec![make_leg(inst_a, inst_b, iusd(500_000))];
        let result = executor.execute_batch_atomic(legs, Currency::iusd()).await.unwrap();
        assert!(result.success);
        assert_eq!(result.executed_legs, 1);
        assert_eq!(result.failed_legs, 0);
    }

    #[tokio::test]
    async fn insufficient_funds_fails_atomically() {
        let executor = SettlementExecutor::new();
        let channel = InMemoryChannel::new();
        let inst_a = InstitutionId::new();
        let inst_b = InstitutionId::new();

        // Only 100 but trying to settle 1000
        channel.set_balance(inst_a.clone(), iusd(100)).await;
        executor.register_channel(
            SettlementChannel::Internal,
            std::sync::Arc::new(channel),
        ).await;

        let legs = vec![make_leg(inst_a, inst_b, iusd(1_000))];
        let result = executor.execute_batch_atomic(legs, Currency::iusd()).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn multi_leg_batch_executes_in_order() {
        let executor = SettlementExecutor::new();
        let channel = InMemoryChannel::new();
        let a = InstitutionId::new();
        let b = InstitutionId::new();
        let c = InstitutionId::new();

        channel.set_balance(a.clone(), iusd(1_000)).await;
        channel.set_balance(b.clone(), iusd(500)).await;
        executor.register_channel(
            SettlementChannel::Internal,
            std::sync::Arc::new(channel),
        ).await;

        let legs = vec![
            make_leg(a.clone(), b.clone(), iusd(300)),
            make_leg(b.clone(), c.clone(), iusd(200)),
        ];
        let result = executor.execute_batch_atomic(legs, Currency::iusd()).await.unwrap();
        assert!(result.success);
        assert_eq!(result.executed_legs, 2);
    }

    #[tokio::test]
    async fn batch_state_lifecycle() {
        let executor = SettlementExecutor::new();
        let channel = InMemoryChannel::new();
        let a = InstitutionId::new();
        let b = InstitutionId::new();
        channel.set_balance(a.clone(), iusd(1_000)).await;
        executor.register_channel(
            SettlementChannel::Internal,
            std::sync::Arc::new(channel),
        ).await;

        let legs = vec![make_leg(a, b, iusd(100))];
        let batch_id = executor.create_batch(legs, Currency::iusd()).await.unwrap();

        let batch = executor.get_batch(&batch_id).await.unwrap();
        assert_eq!(batch.state, SettlementBatchState::Created);

        executor.prepare(&batch_id).await.unwrap();
        let batch = executor.get_batch(&batch_id).await.unwrap();
        assert_eq!(batch.state, SettlementBatchState::Prepared);

        let result = executor.execute(&batch_id).await.unwrap();
        assert!(result.success);
        let batch = executor.get_batch(&batch_id).await.unwrap();
        assert_eq!(batch.state, SettlementBatchState::Executed);

        executor.confirm_finality(&batch_id).await.unwrap();
        let batch = executor.get_batch(&batch_id).await.unwrap();
        assert_eq!(batch.state, SettlementBatchState::Finalized);
    }

    #[tokio::test]
    async fn empty_batch_is_rejected() {
        let executor = SettlementExecutor::new();
        let result = executor.create_batch(vec![], Currency::iusd()).await;
        assert!(result.is_err());
    }
}
