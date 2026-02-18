//! Stable adapter traits — the ONLY surface through which OpeniBank touches Maple.
//!
//! All other OpeniBank crates depend ONLY on these traits, never on Maple directly.
//! This shields the rest of the system from Maple API changes and enables the
//! [`LocalSimBackend`](crate::local_sim) fallback for zero-dependency demo mode.
//!
//! # The Three Pillars
//!
//! 1. **[`WorldLineBackend`]** — append-only event ledger with hash-chain integrity
//! 2. **[`CommitmentBackend`]** — enforces Intent → Commitment → Consequence invariant
//! 3. **[`ResonatorIdentity`]** — EVM + ed25519 identity, vault never exports keys

use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── WorldLine ─────────────────────────────────────────────────────────────────

/// A single event in the WorldLine append-only ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WllEvent {
    pub id: WllEventId,
    pub run_id: String,
    pub agent_id: String,
    pub event_type: WllEventType,
    /// Domain-specific payload (serialized as JSON for portability).
    pub payload: serde_json::Value,
    /// blake3(prev_hash ‖ cbor(payload)) — zero bytes for genesis event.
    pub hash: [u8; 32],
    pub timestamp: DateTime<Utc>,
}

/// Opaque, monotonically increasing event identifier (ULID format).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WllEventId(pub String);

impl std::fmt::Display for WllEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The semantic stage of a WorldLine event, aligned with Maple's resonance stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WllEventType {
    /// Agent declared intent to act.
    Intent,
    /// Commitment gate approved — consequence may now execute.
    Commitment,
    /// Effect executed (balance changed, permit issued, etc.).
    Consequence,
    /// Cryptographically signed receipt issued.
    Receipt,
    /// A new agent was registered in the system.
    AgentRegistered,
    /// A spend permit was issued.
    PermitIssued,
    /// A spend permit was revoked.
    PermitRevoked,
    /// An error occurred in the pipeline.
    Error,
}

/// Append-only, hash-chained event ledger.
///
/// Implemented by both [`MapleNativeWorldLine`](crate::maple_backend) (wraps
/// Maple's EventFabric) and [`LocalSimWorldLine`](crate::local_sim) (embedded sled WAL).
#[async_trait]
pub trait WorldLineBackend: Send + Sync + 'static {
    /// Append a new event. Returns the assigned event ID and final hash.
    async fn append_event(&self, event: WllEvent) -> Result<WllEventId, WllError>;

    /// Stream all events for `run_id` starting from `from` (inclusive).
    ///
    /// If `follow = true`, the stream stays open and yields new events as they arrive.
    fn tail_events(
        &self,
        run_id: &str,
        from: Option<WllEventId>,
        follow: bool,
    ) -> Pin<Box<dyn Stream<Item = WllEvent> + Send + '_>>;

    /// Export a contiguous slice of events for a run (used for bundle export).
    async fn export_slice(
        &self,
        run_id: &str,
        from: Option<WllEventId>,
        to: Option<WllEventId>,
    ) -> Result<Vec<WllEvent>, WllError>;

    /// Return the latest event ID for a run (useful for hash-chain anchoring).
    async fn latest_event_id(&self, run_id: &str) -> Result<Option<WllEventId>, WllError>;

    /// Total event count for a run (for dashboard display).
    async fn event_count(&self, run_id: &str) -> Result<u64, WllError>;
}

// ── Commitment Gate ───────────────────────────────────────────────────────────

/// An approved, unfulfilled commitment. Opaque handle — you cannot forge one.
///
/// The `CommitmentBackend::execute()` contract guarantees:
/// **It is physically impossible to get a `ConsequenceProof` without a valid `CommitmentHandle`.**
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentHandle {
    pub id: CommitmentId,
    /// blake3 of the serialized intent payload.
    pub intent_hash: [u8; 32],
    pub agent_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Opaque commitment identifier (ULID format with "cmmt_" prefix).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitmentId(pub String);

impl std::fmt::Display for CommitmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Proof that a consequence was executed under a valid commitment.
///
/// Contains the WorldLine event ID that permanently records the execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceProof {
    pub commitment_id: CommitmentId,
    pub executed_at: DateTime<Utc>,
    /// The WorldLine event that recorded this consequence.
    pub worldline_event_id: WllEventId,
}

/// Type alias for an action to execute inside the commitment gate.
///
/// The action receives the commitment ID and returns a JSON value representing
/// the outcome (for WorldLine recording). The actual typed result is returned
/// separately by the concrete implementation.
pub type CommitmentAction =
    Box<dyn FnOnce(CommitmentId) -> futures::future::BoxFuture<'static, Result<serde_json::Value, String>> + Send>;

/// Enforces the Intent → Commitment → Consequence invariant.
///
/// `execute_committed()` is the ONLY way to obtain a [`ConsequenceProof`].
/// Without a valid [`CommitmentHandle`], execution is impossible.
///
/// This trait is dyn-compatible: the action is passed as a boxed closure.
#[async_trait]
pub trait CommitmentBackend: Send + Sync + 'static {
    /// Record an intent and return a handle. No execution happens yet.
    async fn prepare(
        &self,
        agent_id: &str,
        intent_description: &str,
        intent_hash: [u8; 32],
    ) -> Result<CommitmentHandle, CommitmentError>;

    /// Execute the committed action. Called only if the handle is valid and not expired.
    ///
    /// Returns a [`ConsequenceProof`] containing the WorldLine event ID that permanently
    /// records this consequence. Embed this proof into the resulting Receipt.
    async fn execute_committed(
        &self,
        handle: CommitmentHandle,
        action: CommitmentAction,
    ) -> Result<(serde_json::Value, ConsequenceProof), CommitmentError>;

    /// Mark a commitment as failed (for error paths). Idempotent.
    async fn fail(&self, handle: CommitmentHandle, reason: &str) -> Result<(), CommitmentError>;

    /// Count of currently pending (not yet executed or failed) commitments.
    fn pending_count(&self) -> usize;
}

// ── Resonator Identity ────────────────────────────────────────────────────────

/// Opaque resonator identifier (maps to Maple's `ResonatorId`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorId(pub String);

impl std::fmt::Display for ResonatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Cryptographic identity for a Resonator/Agent.
///
/// # Key Security Invariant
///
/// This trait has **no method that returns raw key bytes**.
/// The vault never exports private keys. Period.
#[async_trait]
pub trait ResonatorIdentity: Send + Sync + 'static {
    /// The stable identifier for this resonator.
    fn resonator_id(&self) -> &ResonatorId;

    /// The EVM address (20 bytes) derived deterministically from the secp256k1 keypair.
    ///
    /// Computed as: `keccak256(uncompressed_secp256k1_pubkey)[12..]`
    fn evm_address(&self) -> [u8; 20];

    /// Format the EVM address as a checksummed hex string: `"0xAbCd..."`.
    fn evm_address_hex(&self) -> String {
        let addr = self.evm_address();
        format!("0x{}", hex::encode(addr))
    }

    /// Sign `msg` with the ed25519 key (for receipts, permits, attestations).
    fn sign_ed25519(&self, msg: &[u8]) -> ed25519_dalek::Signature;

    /// Verify an ed25519 signature produced by this identity.
    fn verify_ed25519(&self, msg: &[u8], sig: &ed25519_dalek::Signature) -> bool;

    /// Sign `msg_hash` with the secp256k1 key (for on-chain EVM transactions).
    ///
    /// Returns 65 bytes: `r (32) ‖ s (32) ‖ v (1)` (Ethereum compact format).
    async fn sign_evm(&self, msg_hash: [u8; 32]) -> Result<[u8; 65], IdentityError>;

    /// Returns the ed25519 verifying key (public key — safe to share).
    fn ed25519_verifying_key(&self) -> ed25519_dalek::VerifyingKey;
}

// ── Error Types ───────────────────────────────────────────────────────────────

/// Errors from the WorldLine backend.
#[derive(Debug, Error)]
pub enum WllError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("hash chain broken at event {0}")]
    HashChainBroken(String),
    #[error("event not found: {0}")]
    NotFound(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Errors from the commitment gate backend.
#[derive(Debug, Error)]
pub enum CommitmentError {
    #[error("commitment not found: {0}")]
    NotFound(String),
    #[error("commitment expired")]
    Expired,
    #[error("consequence without commitment — invariant violated")]
    NoCommitment,
    #[error("action failed: {0:?}")]
    ActionFailed(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("lock error")]
    LockError,
}

/// Errors from the identity/vault layer.
#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("signing failed: {0}")]
    SigningFailed(String),
    #[error("wallet not connected")]
    NotConnected,
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),
}
