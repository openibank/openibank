use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use ed25519_dalek::SigningKey;
use openibank_domain::{Amount, DomainError, Receipt, SpendPermit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::{broadcast, Mutex, RwLock};
use worldline_core::identity::IdentityManager;
use worldline_core::types::{
    CapabilityId, CommitmentScope, ConfidenceProfile, EffectDomain, EventId, IdentityMaterial,
    Reversibility, TemporalAnchor, TemporalBounds, WorldlineId,
};
use worldline_runtime::fabric::{
    EventFabric, EventPayload, FabricConfig, KernelEvent, ResonanceStage,
};
use worldline_runtime::gate::{
    AdjudicationResult, CommitmentDeclaration, CommitmentGate, CommitmentOutcome, DeclarationStage,
    FinalDecisionStage, GateConfig, IdentityBindingStage, PolicyEvaluationStage,
    RiskAssessmentStage,
};
use worldline_runtime::gate::{
    CapabilityCheckStage, CoSignatureStage, MockCapabilityProvider, MockPolicyProvider,
};

#[derive(Debug, Error)]
pub enum MapleAdapterError {
    #[error("fabric error: {0}")]
    Fabric(#[from] worldline_runtime::fabric::FabricError),
    #[error("gate error: {0}")]
    Gate(#[from] worldline_runtime::gate::GateError),
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("unknown agent: {0}")]
    UnknownAgent(String),
    #[error("commitment denied: {0}")]
    CommitmentDenied(String),
    #[error("receipt worldline pointer not found in WorldLine log")]
    WorldlinePointerNotFound,
    #[error("receipt worldline hash mismatch")]
    WorldlineHashMismatch,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ActionKind {
    Mint,
    Permit,
    Escrow,
    Settle,
}

impl ActionKind {
    fn as_str(self) -> &'static str {
        match self {
            ActionKind::Mint => "mint_iusd",
            ActionKind::Permit => "issue_permit",
            ActionKind::Escrow => "create_escrow",
            ActionKind::Settle => "settle",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldlineEventRecord {
    pub event_id: String,
    pub worldline_id: String,
    pub agent_id: String,
    pub stage: String,
    pub event_type: String,
    pub hash: String,
    pub timestamp_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub agent_id: String,
    pub worldline_id: String,
    pub balance: Amount,
    pub permits_count: usize,
    pub last_worldline_event: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub mode: String,
    pub worldline_id: String,
    pub maple_version: String,
}

#[async_trait]
pub trait WorldLineWriter {
    async fn execute_action(
        &self,
        action: ActionKind,
        from: &str,
        to: &str,
        amount: Amount,
    ) -> Result<Receipt, MapleAdapterError>;
}

pub trait WorldLineReader {
    fn subscribe_worldline(&self) -> broadcast::Receiver<WorldlineEventRecord>;
}

#[async_trait]
pub trait CommitmentGatePort {
    async fn verify_receipt(&self, receipt: &Receipt) -> Result<(), MapleAdapterError>;
}

#[derive(Clone)]
pub struct MapleWorldlineRuntime {
    run_id: String,
    root_worldline: WorldlineId,
    fabric: Arc<EventFabric>,
    gate: Arc<Mutex<CommitmentGate>>,
    signing_key: SigningKey,
    worldline_to_agent: Arc<HashMap<WorldlineId, String>>,
    agent_to_worldline: Arc<HashMap<String, WorldlineId>>,
    balances: Arc<RwLock<HashMap<String, Amount>>>,
    permits: Arc<RwLock<HashMap<String, usize>>>,
    receipts: Arc<RwLock<Vec<Receipt>>>,
    raw_events: Arc<RwLock<Vec<KernelEvent>>>,
    event_records: Arc<RwLock<Vec<WorldlineEventRecord>>>,
    last_event_by_worldline: Arc<Mutex<HashMap<WorldlineId, EventId>>>,
    event_sender: broadcast::Sender<WorldlineEventRecord>,
    data_root: PathBuf,
}

impl MapleWorldlineRuntime {
    pub async fn new(run_id: impl Into<String>, seed: u64) -> Result<Self, MapleAdapterError> {
        let run_id = run_id.into();
        let data_root = default_data_root();
        let run_dir = data_root.join("runs").join(&run_id);
        let fabric_dir = run_dir.join("fabric");
        let mut fabric_config = FabricConfig::default();
        let force_memory = cfg!(test)
            || std::env::var("OPENIBANK_WAL_IN_MEMORY")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
                .unwrap_or(false);
        if !force_memory && std::fs::create_dir_all(&fabric_dir).is_ok() {
            fabric_config.data_dir = Some(fabric_dir);
        }

        let fabric = match EventFabric::init(fabric_config).await {
            Ok(fabric) => Arc::new(fabric),
            Err(first_err) => {
                let memory_fabric = EventFabric::init(FabricConfig::default())
                    .await
                    .map_err(|_| first_err)?;
                Arc::new(memory_fabric)
            }
        };

        let mut identity_manager = IdentityManager::new();
        let mut agent_to_worldline = HashMap::new();
        let mut worldline_to_agent = HashMap::new();
        let mut balances = HashMap::new();
        let mut permits = HashMap::new();

        for agent in ["issuer-01", "buyer-01", "seller-01", "auditor-01"] {
            let wid = identity_manager.create_worldline(worldline_material(&run_id, agent))?;
            agent_to_worldline.insert(agent.to_string(), wid.clone());
            worldline_to_agent.insert(wid, agent.to_string());
            balances.insert(agent.to_string(), 0);
            permits.insert(agent.to_string(), 0);
        }

        let root_worldline =
            identity_manager.create_worldline(worldline_material(&run_id, "root"))?;
        let identity_manager = Arc::new(std::sync::RwLock::new(identity_manager));

        let capability_provider = Arc::new(MockCapabilityProvider::new());
        for wid in agent_to_worldline.values() {
            capability_provider.grant(wid.clone(), "CAP-FIN", EffectDomain::Financial);
        }
        capability_provider.grant(root_worldline.clone(), "CAP-FIN", EffectDomain::Financial);

        let policy_provider = Arc::new(MockPolicyProvider::approve_all());
        let mut gate = CommitmentGate::new(fabric.clone(), GateConfig::default());
        gate.add_stage(Box::new(DeclarationStage::new(true, 0.6)));
        gate.add_stage(Box::new(IdentityBindingStage::new(identity_manager)));
        gate.add_stage(Box::new(CapabilityCheckStage::new(capability_provider)));
        gate.add_stage(Box::new(PolicyEvaluationStage::new(policy_provider)));
        gate.add_stage(Box::new(RiskAssessmentStage::new(Default::default())));
        gate.add_stage(Box::new(CoSignatureStage::new()));
        gate.add_stage(Box::new(FinalDecisionStage::new()));

        let (event_sender, _) = broadcast::channel(1024);
        let raw_events = Arc::new(RwLock::new(Vec::new()));
        let event_records = Arc::new(RwLock::new(Vec::new()));
        let receipts = Arc::new(RwLock::new(Vec::new()));
        let last_event_by_worldline = Arc::new(Mutex::new(HashMap::new()));

        let runtime = Self {
            run_id: run_id.clone(),
            root_worldline,
            fabric: fabric.clone(),
            gate: Arc::new(Mutex::new(gate)),
            signing_key: signing_key_from_seed(&run_id, seed),
            worldline_to_agent: Arc::new(worldline_to_agent),
            agent_to_worldline: Arc::new(agent_to_worldline),
            balances: Arc::new(RwLock::new(balances)),
            permits: Arc::new(RwLock::new(permits)),
            receipts,
            raw_events: raw_events.clone(),
            event_records: event_records.clone(),
            last_event_by_worldline,
            event_sender: event_sender.clone(),
            data_root,
        };

        runtime
            .spawn_worldline_listener(raw_events, event_records, event_sender)
            .await;
        runtime.emit_genesis_events().await?;
        runtime.persist_snapshot().await?;

        Ok(runtime)
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn worldline_id(&self) -> String {
        worldline_ref(&self.root_worldline)
    }

    pub fn maple_version(&self) -> &'static str {
        "0.1.2"
    }

    pub async fn list_agents(&self) -> Vec<AgentSnapshot> {
        let balances = self.balances.read().await;
        let permits = self.permits.read().await;
        let records = self.event_records.read().await;

        let mut snapshots = Vec::new();
        for (agent, wid) in self.agent_to_worldline.iter() {
            let last = records
                .iter()
                .rev()
                .find(|record| record.agent_id == *agent)
                .map(|record| record.event_id.clone());
            snapshots.push(AgentSnapshot {
                agent_id: agent.clone(),
                worldline_id: worldline_ref(wid),
                balance: *balances.get(agent).unwrap_or(&0),
                permits_count: *permits.get(agent).unwrap_or(&0),
                last_worldline_event: last,
            });
        }
        snapshots.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        snapshots
    }

    pub async fn latest_receipts(&self, limit: usize) -> Vec<Receipt> {
        let receipts = self.receipts.read().await;
        receipts.iter().rev().take(limit).cloned().collect()
    }

    pub async fn tail_worldline(&self, limit: usize) -> Vec<WorldlineEventRecord> {
        let records = self.event_records.read().await;
        records.iter().rev().take(limit).cloned().collect()
    }

    pub async fn export_bundle(
        &self,
        run_id: &str,
        out_dir: &Path,
    ) -> Result<PathBuf, MapleAdapterError> {
        if run_id != self.run_id {
            return Err(MapleAdapterError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unknown run_id: {}", run_id),
            )));
        }
        self.persist_snapshot().await?;
        export_persisted_run(run_id, out_dir, &self.data_root)
    }

    pub fn export_persisted(run_id: &str, out_dir: &Path) -> Result<PathBuf, MapleAdapterError> {
        export_persisted_run(run_id, out_dir, &default_data_root())
    }

    async fn spawn_worldline_listener(
        &self,
        raw_events: Arc<RwLock<Vec<KernelEvent>>>,
        event_records: Arc<RwLock<Vec<WorldlineEventRecord>>>,
        event_sender: broadcast::Sender<WorldlineEventRecord>,
    ) {
        let fabric = self.fabric.clone();
        let worldline_to_agent = self.worldline_to_agent.clone();
        let mut rx = fabric.subscribe(None, None).await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let already_recorded = raw_events
                    .read()
                    .await
                    .iter()
                    .any(|existing| existing.id == event.id);
                if already_recorded {
                    continue;
                }

                let record = to_record(&event, &worldline_to_agent);

                raw_events.write().await.push(event.clone());
                event_records.write().await.push(record.clone());
                let _ = event_sender.send(record);
            }
        });
    }

    async fn emit_genesis_events(&self) -> Result<(), MapleAdapterError> {
        for (agent, wid) in self.agent_to_worldline.iter() {
            let event = self
                .fabric
                .emit(
                    wid.clone(),
                    ResonanceStage::System,
                    EventPayload::WorldlineCreated {
                        profile: format!("openibank-agent:{}", agent),
                    },
                    vec![],
                )
                .await?;
            self.record_event(event.clone()).await;
            self.last_event_by_worldline
                .lock()
                .await
                .insert(wid.clone(), event.id);
        }

        let root_event = self
            .fabric
            .emit(
                self.root_worldline.clone(),
                ResonanceStage::System,
                EventPayload::WorldlineCreated {
                    profile: "openibank-root".to_string(),
                },
                vec![],
            )
            .await?;
        self.record_event(root_event.clone()).await;
        self.last_event_by_worldline
            .lock()
            .await
            .insert(self.root_worldline.clone(), root_event.id);

        Ok(())
    }

    async fn persist_snapshot(&self) -> Result<(), MapleAdapterError> {
        let run_dir = self.data_root.join("runs").join(&self.run_id);
        let receipts_dir = run_dir.join("receipts");
        std::fs::create_dir_all(&receipts_dir)?;

        let metadata = RunMetadata {
            run_id: self.run_id.clone(),
            mode: "local-sim".to_string(),
            worldline_id: worldline_ref(&self.root_worldline),
            maple_version: self.maple_version().to_string(),
        };
        std::fs::write(
            run_dir.join("metadata.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )?;

        let receipts = self.receipts.read().await;
        std::fs::write(
            run_dir.join("receipts.json"),
            serde_json::to_vec_pretty(&*receipts)?,
        )?;

        for receipt in receipts.iter() {
            let filename = format!("{}.json", receipt.tx_id);
            openibank_domain::save_receipt(&receipts_dir.join(filename), receipt)?;
        }

        let raw_events = self.raw_events.read().await;
        let exported: Vec<ExportedWorldlineEvent> = raw_events
            .iter()
            .map(ExportedWorldlineEvent::from_kernel)
            .collect();
        std::fs::write(
            run_dir.join("worldline_slice.json"),
            serde_json::to_vec_pretty(&exported)?,
        )?;

        Ok(())
    }

    async fn record_event(&self, event: KernelEvent) {
        let already_recorded = self
            .raw_events
            .read()
            .await
            .iter()
            .any(|existing| existing.id == event.id);
        if already_recorded {
            return;
        }

        let record = to_record(&event, &self.worldline_to_agent);
        self.raw_events.write().await.push(event);
        self.event_records.write().await.push(record.clone());
        let _ = self.event_sender.send(record);
    }
}

#[async_trait]
impl WorldLineWriter for MapleWorldlineRuntime {
    async fn execute_action(
        &self,
        action: ActionKind,
        from: &str,
        to: &str,
        amount: Amount,
    ) -> Result<Receipt, MapleAdapterError> {
        let from_wid = self
            .agent_to_worldline
            .get(from)
            .cloned()
            .ok_or_else(|| MapleAdapterError::UnknownAgent(from.to_string()))?;
        let to_wid = self
            .agent_to_worldline
            .get(to)
            .cloned()
            .ok_or_else(|| MapleAdapterError::UnknownAgent(to.to_string()))?;

        let parent = self
            .last_event_by_worldline
            .lock()
            .await
            .get(&from_wid)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();

        let intent = self
            .fabric
            .emit(
                from_wid.clone(),
                ResonanceStage::Intent,
                EventPayload::IntentStabilized {
                    direction: action.as_str().to_string(),
                    confidence: 0.92,
                    conditions: vec![
                        format!("from={}", from),
                        format!("to={}", to),
                        format!("amount={}", amount),
                    ],
                },
                parent,
            )
            .await?;
        self.record_event(intent.clone()).await;
        self.last_event_by_worldline
            .lock()
            .await
            .insert(from_wid.clone(), intent.id.clone());

        let declaration = CommitmentDeclaration::builder(
            from_wid.clone(),
            CommitmentScope {
                effect_domain: EffectDomain::Financial,
                targets: vec![to_wid.clone()],
                constraints: vec![format!("max_amount={}", amount)],
            },
        )
        .derived_from_intent(intent.id.clone())
        .confidence(ConfidenceProfile::new(0.92, 0.9, 0.91, 0.88))
        .temporal_bounds(TemporalBounds {
            starts: TemporalAnchor::now(0),
            expires: None,
            review_at: None,
        })
        .reversibility(Reversibility::Conditional {
            conditions: vec!["receipt_verification".to_string()],
        })
        .capability(CapabilityId("CAP-FIN".to_string()))
        .affected_party(to_wid.clone())
        .evidence(format!("openibank-action={}", action.as_str()))
        .build();

        let commitment_id = declaration.id.clone();

        let adjudication = self.gate.lock().await.submit(declaration).await?;
        if !matches!(adjudication, AdjudicationResult::Approved { .. }) {
            return Err(MapleAdapterError::CommitmentDenied(format!(
                "action {} denied by commitment gate",
                action.as_str()
            )));
        }

        {
            let mut balances = self.balances.write().await;
            *balances.entry(from.to_string()).or_default() -= amount;
            *balances.entry(to.to_string()).or_default() += amount;
        }
        {
            let mut permits = self.permits.write().await;
            *permits.entry(from.to_string()).or_default() += 1;
        }

        let consequence = self
            .fabric
            .emit(
                from_wid.clone(),
                ResonanceStage::Consequence,
                EventPayload::ConsequenceObserved {
                    commitment_id: commitment_id.clone(),
                    state_changes: serde_json::json!({
                        "balances": self.balances.read().await.clone(),
                        "action": action.as_str(),
                    }),
                },
                vec![intent.id.clone()],
            )
            .await?;
        self.record_event(consequence.clone()).await;
        self.last_event_by_worldline
            .lock()
            .await
            .insert(from_wid.clone(), consequence.id.clone());

        self.gate
            .lock()
            .await
            .record_outcome(&commitment_id, CommitmentOutcome::Fulfilled)
            .await?;

        let permit = SpendPermit::new(
            openibank_domain::AgentId::new(from),
            openibank_domain::AgentId::new(to),
            amount,
        );

        let receipt = Receipt::new_unsigned(
            openibank_domain::AgentId::new(from),
            openibank_domain::AgentId::new(to),
            amount,
            permit.permit_id,
            format!("{}", commitment_id),
            worldline_ref(&from_wid),
            format!("{}", consequence.id),
            hex::encode(consequence.integrity_hash.0),
            "AI agents need banks too. Maple WorldLine verified.",
        )
        .sign(&self.signing_key)?;

        let receipt_event = self
            .fabric
            .emit(
                from_wid.clone(),
                ResonanceStage::Consequence,
                EventPayload::Custom {
                    type_name: "openibank.receipt".to_string(),
                    data: serde_json::to_value(&receipt)?,
                },
                vec![consequence.id.clone()],
            )
            .await?;
        self.record_event(receipt_event.clone()).await;
        self.last_event_by_worldline
            .lock()
            .await
            .insert(from_wid.clone(), receipt_event.id.clone());

        let mut final_receipt = receipt;
        final_receipt.worldline_event_id = format!("{}", receipt_event.id);
        final_receipt.worldline_event_hash = hex::encode(receipt_event.integrity_hash.0);
        final_receipt = final_receipt.sign(&self.signing_key)?;

        let verification_event = self
            .fabric
            .emit(
                from_wid.clone(),
                ResonanceStage::Governance,
                EventPayload::Custom {
                    type_name: "openibank.receipt.verified".to_string(),
                    data: serde_json::json!({
                        "tx_id": final_receipt.tx_id,
                        "worldline_pointer": final_receipt.worldline_pointer(),
                    }),
                },
                vec![receipt_event.id.clone()],
            )
            .await?;
        self.record_event(verification_event).await;

        self.receipts.write().await.push(final_receipt.clone());
        self.persist_snapshot().await?;
        Ok(final_receipt)
    }
}

impl WorldLineReader for MapleWorldlineRuntime {
    fn subscribe_worldline(&self) -> broadcast::Receiver<WorldlineEventRecord> {
        self.event_sender.subscribe()
    }
}

#[async_trait]
impl CommitmentGatePort for MapleWorldlineRuntime {
    async fn verify_receipt(&self, receipt: &Receipt) -> Result<(), MapleAdapterError> {
        receipt.verify()?;
        let mut matched: Option<KernelEvent> = None;
        for _ in 0..20 {
            let events = self.raw_events.read().await;
            matched = events
                .iter()
                .find(|event| format!("{}", event.id) == receipt.worldline_event_id)
                .cloned();
            drop(events);
            if matched.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let event = matched.ok_or(MapleAdapterError::WorldlinePointerNotFound)?;

        let expected_hash = hex::encode(event.integrity_hash.0);
        if expected_hash != receipt.worldline_event_hash {
            return Err(MapleAdapterError::WorldlineHashMismatch);
        }
        if worldline_ref(&event.worldline_id) != receipt.worldline_id {
            return Err(MapleAdapterError::WorldlinePointerNotFound);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportedWorldlineEvent {
    pub event_id: String,
    pub worldline_id: String,
    pub stage: String,
    pub event_type: String,
    pub hash: String,
    pub payload: serde_json::Value,
    pub timestamp_ms: i64,
}

impl ExportedWorldlineEvent {
    fn from_kernel(event: &KernelEvent) -> Self {
        Self {
            event_id: format!("{}", event.id),
            worldline_id: worldline_ref(&event.worldline_id),
            stage: format!("{:?}", event.stage),
            event_type: event_type_label(&event.payload),
            hash: hex::encode(event.integrity_hash.0),
            payload: serde_json::to_value(&event.payload).unwrap_or_else(|_| serde_json::json!({})),
            timestamp_ms: event.timestamp.physical as i64,
        }
    }
}

fn event_type_label(payload: &EventPayload) -> String {
    match payload {
        EventPayload::IntentStabilized { .. } => "Intent Proposed".to_string(),
        EventPayload::CommitmentDeclared { .. } => "Commitment Recorded".to_string(),
        EventPayload::CommitmentApproved { .. } => "Commitment Approved".to_string(),
        EventPayload::CommitmentDenied { .. } => "Commitment Denied".to_string(),
        EventPayload::ConsequenceObserved { .. } => "Consequence Executed".to_string(),
        EventPayload::CommitmentFulfilled { .. } => "Consequence Executed".to_string(),
        EventPayload::Custom { type_name, .. } if type_name == "openibank.receipt.verified" => {
            "Receipt Verified".to_string()
        }
        EventPayload::Custom { type_name, .. } if type_name == "openibank.receipt" => {
            "Receipt Emitted".to_string()
        }
        EventPayload::Custom { type_name, .. } => type_name.clone(),
        _ => format!("{:?}", payload),
    }
}

fn to_record(
    event: &KernelEvent,
    worldline_to_agent: &HashMap<WorldlineId, String>,
) -> WorldlineEventRecord {
    WorldlineEventRecord {
        event_id: format!("{}", event.id),
        worldline_id: worldline_ref(&event.worldline_id),
        agent_id: worldline_to_agent
            .get(&event.worldline_id)
            .cloned()
            .unwrap_or_else(|| "system".to_string()),
        stage: format!("{:?}", event.stage),
        event_type: event_type_label(&event.payload),
        hash: hex::encode(event.integrity_hash.0),
        timestamp_ms: event.timestamp.physical as i64,
    }
}

fn worldline_material(run_id: &str, label: &str) -> IdentityMaterial {
    let mut hasher = Sha256::new();
    hasher.update(run_id.as_bytes());
    hasher.update(b":");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest[..32]);
    IdentityMaterial::GenesisHash(bytes)
}

fn signing_key_from_seed(run_id: &str, seed: u64) -> SigningKey {
    let mut hasher = Sha256::new();
    hasher.update(run_id.as_bytes());
    hasher.update(seed.to_le_bytes());
    let digest = hasher.finalize();
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&digest[..32]);
    SigningKey::from_bytes(&key_bytes)
}

fn worldline_ref(worldline: &WorldlineId) -> String {
    format!("wl:{}", hex::encode(worldline.identity_hash()))
}

fn default_data_root() -> PathBuf {
    if let Ok(path) = std::env::var("OPENIBANK_DATA_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(home) = std::env::var("HOME") {
        let candidate = PathBuf::from(home).join(".openibank");
        if std::fs::create_dir_all(&candidate).is_ok() {
            let probe = candidate.join(".write_probe");
            let writable = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&probe)
                .is_ok();
            let _ = std::fs::remove_file(&probe);
            if writable {
                return candidate;
            }
        }
    }
    std::env::temp_dir().join("openibank")
}

fn export_persisted_run(
    run_id: &str,
    out_dir: &Path,
    data_root: &Path,
) -> Result<PathBuf, MapleAdapterError> {
    let source_dir = data_root.join("runs").join(run_id);
    if !source_dir.exists() {
        return Err(MapleAdapterError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("run not found: {}", run_id),
        )));
    }

    let target_dir = out_dir.join(run_id);
    std::fs::create_dir_all(&target_dir)?;
    copy_dir_all(&source_dir, &target_dir)?;
    Ok(target_dir)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), MapleAdapterError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

impl From<worldline_core::identity::IdentityError> for MapleAdapterError {
    fn from(error: worldline_core::identity::IdentityError) -> Self {
        MapleAdapterError::Io(std::io::Error::other(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn adapter_runs_intent_commitment_consequence_receipt_flow() {
        let run_id = format!("test-run-{}", uuid::Uuid::new_v4());
        let runtime = MapleWorldlineRuntime::new(run_id, 7)
            .await
            .expect("runtime");

        let receipt = runtime
            .execute_action(ActionKind::Settle, "buyer-01", "seller-01", 500)
            .await
            .expect("action");
        runtime.verify_receipt(&receipt).await.expect("verify");

        let mut has_intent = false;
        let mut has_commitment = false;
        let mut has_consequence = false;
        let mut has_receipt = false;

        for _ in 0..30 {
            let tail = runtime.tail_worldline(50).await;
            has_intent = tail.iter().any(|e| e.event_type == "Intent Proposed");
            has_commitment = tail.iter().any(|e| e.event_type == "Commitment Recorded");
            has_consequence = tail.iter().any(|e| e.event_type == "Consequence Executed");
            has_receipt = tail.iter().any(|e| e.event_type == "Receipt Verified");
            if has_intent && has_commitment && has_consequence && has_receipt {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert!(has_intent && has_commitment && has_consequence && has_receipt);
    }
}
