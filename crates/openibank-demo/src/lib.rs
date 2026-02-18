use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use openibank_domain::card::{write_cards, CardFormat};
use openibank_maple::{
    ActionKind, CommitmentGatePort, MapleAdapterError, MapleWorldlineRuntime, WorldLineWriter,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DemoError {
    #[error("maple adapter error: {0}")]
    Maple(#[from] MapleAdapterError),
    #[error("domain error: {0}")]
    Domain(#[from] openibank_domain::DomainError),
    #[error("demo is already running")]
    AlreadyRunning,
    #[error("demo is not running")]
    NotRunning,
}

#[derive(Clone)]
pub struct DemoEngine {
    runtime: MapleWorldlineRuntime,
    seed: u64,
    running: Arc<AtomicBool>,
    task: Arc<Mutex<Option<JoinHandle<()>>>>,
    stop_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl DemoEngine {
    pub async fn new(seed: u64) -> Result<Self, DemoError> {
        let run_id = format!(
            "run_{}_{}_{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            seed,
            &Uuid::new_v4().to_string()[..8]
        );
        let runtime = MapleWorldlineRuntime::new(run_id, seed).await?;
        Ok(Self {
            runtime,
            seed,
            running: Arc::new(AtomicBool::new(false)),
            task: Arc::new(Mutex::new(None)),
            stop_tx: Arc::new(Mutex::new(None)),
        })
    }

    pub fn runtime(&self) -> MapleWorldlineRuntime {
        self.runtime.clone()
    }

    pub fn run_id(&self) -> &str {
        self.runtime.run_id()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub async fn start(&self) -> Result<(), DemoError> {
        if self.is_running() {
            return Err(DemoError::AlreadyRunning);
        }

        let (stop_tx, mut stop_rx) = oneshot::channel();
        *self.stop_tx.lock().await = Some(stop_tx);
        self.running.store(true, Ordering::Relaxed);

        let runtime = self.runtime.clone();
        let seed = self.seed;
        let running = self.running.clone();
        let task = tokio::spawn(async move {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut step = 0usize;
            let actions = [
                ActionKind::Mint,
                ActionKind::Permit,
                ActionKind::Escrow,
                ActionKind::Settle,
            ];

            loop {
                tokio::select! {
                    _ = &mut stop_rx => break,
                    _ = tokio::time::sleep(Duration::from_millis(1200)) => {
                        let action = actions[step % actions.len()];
                        let amount = rng.gen_range(500..=3_000) as i64;
                        let (from, to) = match action {
                            ActionKind::Mint => ("issuer-01", "buyer-01"),
                            ActionKind::Permit => ("buyer-01", "seller-01"),
                            ActionKind::Escrow => ("buyer-01", "seller-01"),
                            ActionKind::Settle => ("buyer-01", "seller-01"),
                        };

                        let _ = runtime.execute_action(action, from, to, amount).await;
                        step += 1;
                    }
                }
            }

            running.store(false, Ordering::Relaxed);
        });

        *self.task.lock().await = Some(task);
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), DemoError> {
        if !self.is_running() {
            return Err(DemoError::NotRunning);
        }

        if let Some(stop_tx) = self.stop_tx.lock().await.take() {
            let _ = stop_tx.send(());
        }
        if let Some(task) = self.task.lock().await.take() {
            let _ = task.await;
        }

        self.running.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub async fn generate_latest_cards(&self, out_dir: &Path) -> Result<Vec<PathBuf>, DemoError> {
        let latest = self.runtime.latest_receipts(1).await;
        if let Some(receipt) = latest.first() {
            Ok(write_cards(receipt, out_dir, CardFormat::Both)?)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn verify_latest_receipt(&self) -> Result<bool, DemoError> {
        let latest = self.runtime.latest_receipts(1).await;
        if let Some(receipt) = latest.first() {
            self.runtime.verify_receipt(receipt).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn export_bundle(&self, out_dir: &Path) -> Result<PathBuf, DemoError> {
        Ok(self.runtime.export_bundle(self.run_id(), out_dir).await?)
    }
}
