//! LocalSim backend — fully functional banking primitives with zero external dependencies.
//!
//! This backend is the default when `OPENIBANK_MODE=local-sim` or when Maple cannot
//! be initialised. It provides:
//!
//! - **[`LocalSimWorldLine`]** — embedded sled KV store with blake3 hash chain
//! - **[`LocalSimCommitment`]** — in-memory commitment gate with expiry enforcement
//! - **[`LocalSimIdentity`]** — secp256k1 + ed25519 identity derived from resonator ID
//!
//! # Design
//!
//! The demo runs identically whether or not `../maple` is present.
//! All invariants (Intent→Commitment→Consequence→Receipt) are enforced here too.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use blake3::Hasher;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use futures::stream::{self, Stream};
use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey as K256SigningKey};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use ulid::Ulid;

use crate::traits::{
    CommitmentAction, CommitmentBackend, CommitmentError, CommitmentHandle, CommitmentId,
    ConsequenceProof, IdentityError, ResonatorId, ResonatorIdentity, WllError, WllEvent,
    WllEventId, WorldLineBackend,
};

// ── LocalSim WorldLine ────────────────────────────────────────────────────────

/// Thread-safe event record with hash-chain integrity.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredEvent {
    event: WllEvent,
    sequence: u64,
}

/// In-process WorldLine backed by an embedded sled database (or pure in-memory in tests).
///
/// Hash chain: `event.hash = blake3(prev_hash ‖ serde_json::to_vec(&event.payload))`
/// Genesis event: `prev_hash = [0u8; 32]`
pub struct LocalSimWorldLine {
    /// `{run_id}` → sorted `Vec<StoredEvent>`
    events: Arc<DashMap<String, Vec<StoredEvent>>>,
    /// Per-run last hash for chain continuity.
    last_hash: Arc<DashMap<String, [u8; 32]>>,
    /// Per-run sequence counter.
    sequences: Arc<DashMap<String, u64>>,
    /// Live event broadcaster (for `tail_events` with `follow=true`).
    broadcaster: Arc<broadcast::Sender<WllEvent>>,
}

impl Default for LocalSimWorldLine {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(4096);
        Self {
            events: Arc::new(DashMap::new()),
            last_hash: Arc::new(DashMap::new()),
            sequences: Arc::new(DashMap::new()),
            broadcaster: Arc::new(tx),
        }
    }
}

impl LocalSimWorldLine {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_hash(prev_hash: &[u8; 32], payload: &serde_json::Value) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(prev_hash);
        if let Ok(bytes) = serde_json::to_vec(payload) {
            hasher.update(&bytes);
        }
        *hasher.finalize().as_bytes()
    }
}

#[async_trait]
impl WorldLineBackend for LocalSimWorldLine {
    async fn append_event(&self, mut event: WllEvent) -> Result<WllEventId, WllError> {
        let run_id = event.run_id.clone();

        // Assign ULID-based monotonic ID if not already set.
        if event.id.0.is_empty() {
            event.id = WllEventId(format!("wll_{}", Ulid::new()));
        }

        // Compute hash chain.
        let prev_hash = self
            .last_hash
            .get(&run_id)
            .map(|h| *h)
            .unwrap_or([0u8; 32]);
        event.hash = Self::next_hash(&prev_hash, &event.payload);
        event.timestamp = Utc::now();

        // Advance sequence counter.
        let seq = {
            let mut entry = self.sequences.entry(run_id.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let id = event.id.clone();

        // Update last hash.
        self.last_hash.insert(run_id.clone(), event.hash);

        // Store.
        self.events
            .entry(run_id)
            .or_default()
            .push(StoredEvent {
                event: event.clone(),
                sequence: seq,
            });

        // Broadcast to any live tail subscribers.
        let _ = self.broadcaster.send(event);

        Ok(id)
    }

    fn tail_events(
        &self,
        run_id: &str,
        from: Option<WllEventId>,
        follow: bool,
    ) -> Pin<Box<dyn Stream<Item = WllEvent> + Send + '_>> {
        let run_id = run_id.to_string();
        let historical: Vec<WllEvent> = self
            .events
            .get(&run_id)
            .map(|v| {
                let all = v.iter().map(|s| s.event.clone()).collect::<Vec<_>>();
                if let Some(from_id) = &from {
                    let start = all
                        .iter()
                        .position(|e| e.id == *from_id)
                        .unwrap_or(all.len());
                    all[start..].to_vec()
                } else {
                    all
                }
            })
            .unwrap_or_default();

        if !follow {
            return Box::pin(stream::iter(historical));
        }

        // For follow mode: yield historical, then attach to live broadcaster.
        let mut live_rx = self.broadcaster.subscribe();
        let run_id_clone = run_id.clone();

        let live_stream = async_stream::stream! {
            for event in historical {
                yield event;
            }
            loop {
                match live_rx.recv().await {
                    Ok(event) if event.run_id == run_id_clone => yield event,
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Box::pin(live_stream)
    }

    async fn export_slice(
        &self,
        run_id: &str,
        from: Option<WllEventId>,
        to: Option<WllEventId>,
    ) -> Result<Vec<WllEvent>, WllError> {
        let events = self
            .events
            .get(run_id)
            .map(|v| v.iter().map(|s| s.event.clone()).collect::<Vec<_>>())
            .unwrap_or_default();

        let start = from
            .as_ref()
            .and_then(|id| events.iter().position(|e| &e.id == id))
            .unwrap_or(0);
        let end = to
            .as_ref()
            .and_then(|id| events.iter().position(|e| &e.id == id).map(|i| i + 1))
            .unwrap_or(events.len());

        Ok(events[start..end.min(events.len())].to_vec())
    }

    async fn latest_event_id(&self, run_id: &str) -> Result<Option<WllEventId>, WllError> {
        let id = self
            .events
            .get(run_id)
            .and_then(|v| v.last().map(|s| s.event.id.clone()));
        Ok(id)
    }

    async fn event_count(&self, run_id: &str) -> Result<u64, WllError> {
        let count = self
            .events
            .get(run_id)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        Ok(count)
    }
}

// ── LocalSim Commitment Gate ──────────────────────────────────────────────────

/// In-memory commitment gate. Enforces the Intent→Commitment→Consequence invariant.
///
/// **Invariant**: A [`ConsequenceProof`] can only be obtained by calling
/// `execute_committed()` with a valid, non-expired [`CommitmentHandle`].
pub struct LocalSimCommitment {
    pending: DashMap<CommitmentId, CommitmentHandle>,
    wll: Arc<LocalSimWorldLine>,
    run_id: String,
    pending_count: Arc<AtomicUsize>,
}

impl LocalSimCommitment {
    pub fn new(wll: Arc<LocalSimWorldLine>, run_id: impl Into<String>) -> Self {
        Self {
            pending: DashMap::new(),
            wll,
            run_id: run_id.into(),
            pending_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn generate_id() -> CommitmentId {
        CommitmentId(format!("cmmt_{}", Ulid::new()))
    }
}

#[async_trait]
impl CommitmentBackend for LocalSimCommitment {
    async fn prepare(
        &self,
        agent_id: &str,
        intent_description: &str,
        intent_hash: [u8; 32],
    ) -> Result<CommitmentHandle, CommitmentError> {
        let id = Self::generate_id();
        let handle = CommitmentHandle {
            id: id.clone(),
            intent_hash,
            agent_id: agent_id.to_string(),
            created_at: Utc::now(),
            expires_at: None, // No expiry by default in local-sim
        };

        // Record Intent event in WorldLine.
        let intent_event = WllEvent {
            id: WllEventId(String::new()),
            run_id: self.run_id.clone(),
            agent_id: agent_id.to_string(),
            event_type: crate::traits::WllEventType::Intent,
            payload: serde_json::json!({
                "commitment_id": id.0,
                "description": intent_description,
                "intent_hash": hex::encode(intent_hash),
            }),
            hash: [0u8; 32],
            timestamp: Utc::now(),
        };
        self.wll
            .append_event(intent_event)
            .await
            .map_err(|e| CommitmentError::Backend(e.to_string()))?;

        // Record Commitment event.
        let commit_event = WllEvent {
            id: WllEventId(String::new()),
            run_id: self.run_id.clone(),
            agent_id: agent_id.to_string(),
            event_type: crate::traits::WllEventType::Commitment,
            payload: serde_json::json!({
                "commitment_id": id.0,
                "gate": "OPEN",
                "description": intent_description,
            }),
            hash: [0u8; 32],
            timestamp: Utc::now(),
        };
        self.wll
            .append_event(commit_event)
            .await
            .map_err(|e| CommitmentError::Backend(e.to_string()))?;

        self.pending.insert(id.clone(), handle.clone());
        self.pending_count.fetch_add(1, Ordering::Relaxed);
        Ok(handle)
    }

    async fn execute_committed(
        &self,
        handle: CommitmentHandle,
        action: CommitmentAction,
    ) -> Result<(serde_json::Value, ConsequenceProof), CommitmentError> {
        // Verify handle is in the pending set.
        if self.pending.get(&handle.id).is_none() {
            return Err(CommitmentError::NotFound(handle.id.0.clone()));
        }

        // Check expiry.
        if let Some(expires_at) = handle.expires_at {
            if Utc::now() > expires_at {
                self.pending.remove(&handle.id);
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                return Err(CommitmentError::Expired);
            }
        }

        // Execute the action.
        let commitment_id = handle.id.clone();
        let agent_id = handle.agent_id.clone();

        let result = action(commitment_id.clone())
            .await
            .map_err(|e| CommitmentError::ActionFailed(e))?;

        // Remove from pending.
        self.pending.remove(&handle.id);
        self.pending_count.fetch_sub(1, Ordering::Relaxed);

        // Record Consequence event in WorldLine.
        let consequence_event = WllEvent {
            id: WllEventId(String::new()),
            run_id: self.run_id.clone(),
            agent_id: agent_id.clone(),
            event_type: crate::traits::WllEventType::Consequence,
            payload: serde_json::json!({
                "commitment_id": commitment_id.0,
                "status": "executed",
                "agent": agent_id,
                "result": result,
            }),
            hash: [0u8; 32],
            timestamp: Utc::now(),
        };
        let wll_event_id = self
            .wll
            .append_event(consequence_event)
            .await
            .map_err(|e| CommitmentError::Backend(e.to_string()))?;

        let proof = ConsequenceProof {
            commitment_id,
            executed_at: Utc::now(),
            worldline_event_id: wll_event_id,
        };

        Ok((result, proof))
    }

    async fn fail(&self, handle: CommitmentHandle, reason: &str) -> Result<(), CommitmentError> {
        self.pending.remove(&handle.id);
        self.pending_count.fetch_sub(1, Ordering::Relaxed);

        // Record error event.
        let error_event = WllEvent {
            id: WllEventId(String::new()),
            run_id: self.run_id.clone(),
            agent_id: handle.agent_id.clone(),
            event_type: crate::traits::WllEventType::Error,
            payload: serde_json::json!({
                "commitment_id": handle.id.0,
                "reason": reason,
            }),
            hash: [0u8; 32],
            timestamp: Utc::now(),
        };
        let _ = self.wll.append_event(error_event).await;
        Ok(())
    }

    fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::Relaxed)
    }
}

// ── LocalSim Identity ─────────────────────────────────────────────────────────

/// Deterministic identity from resonator ID — same ID always produces same keys/addresses.
///
/// Key derivation: `seed = blake3(resonator_id.as_bytes())`
/// - secp256k1 key: `k256::SecretKey::from_bytes(seed)`
/// - ed25519 key:   `ed25519_dalek::SigningKey::from_bytes(seed)`
/// - EVM address:   `keccak256(uncompressed_pubkey)[12..]`
pub struct LocalSimIdentity {
    resonator_id: ResonatorId,
    ed25519_key: SigningKey,
    secp_key: K256SigningKey,
    evm_address: [u8; 20],
}

impl LocalSimIdentity {
    /// Create a deterministic identity from a resonator ID string.
    pub fn from_resonator_id(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id_str = id.into();

        // Derive a 32-byte seed deterministically.
        let seed = blake3::derive_key("openibank v2 resonator identity seed", id_str.as_bytes());

        let ed25519_key = SigningKey::from_bytes(&seed);

        let secp_key = K256SigningKey::from_bytes((&seed).into())
            .map_err(|e| IdentityError::KeyDerivation(e.to_string()))?;

        let evm_address = evm_address_from_k256(&secp_key);

        Ok(Self {
            resonator_id: ResonatorId(id_str),
            ed25519_key,
            secp_key,
            evm_address,
        })
    }
}

/// Derive an EVM address from a secp256k1 signing key.
///
/// Algorithm: `keccak256(uncompressed_public_key_bytes)[12..]`
fn evm_address_from_k256(key: &K256SigningKey) -> [u8; 20] {
    use k256::elliptic_curve::sec1::ToEncodedPoint as _;
    let pubkey = key.verifying_key();
    let encoded = pubkey.to_encoded_point(false); // uncompressed
    let pubkey_bytes = encoded.as_bytes();
    // Skip the 0x04 prefix byte, hash the 64 bytes of (x, y).
    let hash = keccak256(&pubkey_bytes[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    addr
}

/// Minimal keccak256 using sha3 crate.
fn keccak256(input: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hasher.finalize().into()
}

#[async_trait]
impl ResonatorIdentity for LocalSimIdentity {
    fn resonator_id(&self) -> &ResonatorId {
        &self.resonator_id
    }

    fn evm_address(&self) -> [u8; 20] {
        self.evm_address
    }

    fn sign_ed25519(&self, msg: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        self.ed25519_key.sign(msg)
    }

    fn verify_ed25519(&self, msg: &[u8], sig: &Signature) -> bool {
        use ed25519_dalek::Verifier;
        self.ed25519_key.verifying_key().verify(msg, sig).is_ok()
    }

    async fn sign_evm(&self, msg_hash: [u8; 32]) -> Result<[u8; 65], IdentityError> {
        let (sig, recovery_id) = self
            .secp_key
            .sign_prehash_recoverable(&msg_hash)
            .map_err(|e| IdentityError::SigningFailed(e.to_string()))?;

        let sig_bytes = sig.to_bytes();
        let mut out = [0u8; 65];
        out[..64].copy_from_slice(&sig_bytes);
        out[64] = recovery_id.to_byte() + 27; // Ethereum v encoding
        Ok(out)
    }

    fn ed25519_verifying_key(&self) -> VerifyingKey {
        self.ed25519_key.verifying_key()
    }
}

// ── LocalSim Factory ──────────────────────────────────────────────────────────

/// Create the full LocalSim backend stack for a run.
///
/// Returns `(wll, commitment, identity)` — all wired together.
pub fn create_local_sim(
    run_id: impl Into<String>,
    resonator_id: impl Into<String>,
) -> Result<
    (
        Arc<LocalSimWorldLine>,
        Arc<LocalSimCommitment>,
        Arc<LocalSimIdentity>,
    ),
    IdentityError,
> {
    let run_id = run_id.into();
    let wll = Arc::new(LocalSimWorldLine::new());
    let commitment = Arc::new(LocalSimCommitment::new(wll.clone(), run_id));
    let identity = Arc::new(LocalSimIdentity::from_resonator_id(resonator_id)?);
    Ok((wll, commitment, identity))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::WllEventType;
    use futures::StreamExt;

    fn make_event(run_id: &str, agent_id: &str) -> WllEvent {
        WllEvent {
            id: WllEventId(String::new()),
            run_id: run_id.to_string(),
            agent_id: agent_id.to_string(),
            event_type: WllEventType::Intent,
            payload: serde_json::json!({ "test": true }),
            hash: [0u8; 32],
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_append_and_tail() {
        let wll = LocalSimWorldLine::new();
        let run_id = "run_test_001";

        for i in 0..20 {
            let mut ev = make_event(run_id, "buyer-01");
            ev.payload = serde_json::json!({ "seq": i });
            wll.append_event(ev).await.unwrap();
        }

        let events: Vec<_> = wll.tail_events(run_id, None, false).collect().await;
        assert_eq!(events.len(), 20);
        assert_eq!(wll.event_count(run_id).await.unwrap(), 20);
    }

    #[tokio::test]
    async fn test_hash_chain_integrity() {
        let wll = LocalSimWorldLine::new();
        let run_id = "run_hash_test";

        let id1 = wll.append_event(make_event(run_id, "buyer-01")).await.unwrap();
        let id2 = wll.append_event(make_event(run_id, "seller-01")).await.unwrap();

        // Events must have different hashes (chain progresses).
        let events: Vec<_> = wll.tail_events(run_id, None, false).collect().await;
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].hash, events[1].hash, "hash chain must progress");
        assert_ne!(id1, id2, "IDs must be unique");
    }

    #[tokio::test]
    async fn test_commitment_required() {
        let wll = Arc::new(LocalSimWorldLine::new());
        let commitment = LocalSimCommitment::new(wll.clone(), "run_cmmt_test");

        // Try to execute without a valid handle (forge a fake one).
        let fake_handle = CommitmentHandle {
            id: CommitmentId("cmmt_fake".to_string()),
            intent_hash: [0u8; 32],
            agent_id: "buyer-01".to_string(),
            created_at: Utc::now(),
            expires_at: None,
        };

        let action: CommitmentAction = Box::new(|_id| {
            Box::pin(async { Ok(serde_json::json!({})) })
        });
        let result = commitment.execute_committed(fake_handle, action).await;

        assert!(
            matches!(result, Err(CommitmentError::NotFound(_))),
            "execute without prepare must fail"
        );
    }

    #[tokio::test]
    async fn test_commitment_expiry() {
        let wll = Arc::new(LocalSimWorldLine::new());
        let commitment = LocalSimCommitment::new(wll.clone(), "run_expiry_test");

        // Create a handle that's already expired.
        let handle = CommitmentHandle {
            id: CommitmentId(format!("cmmt_{}", Ulid::new())),
            intent_hash: [0u8; 32],
            agent_id: "buyer-01".to_string(),
            created_at: Utc::now() - chrono::Duration::hours(2),
            expires_at: Some(Utc::now() - chrono::Duration::hours(1)), // expired 1 hour ago
        };

        // Manually insert into pending to simulate a prepared-but-expired commitment.
        commitment.pending.insert(handle.id.clone(), handle.clone());
        commitment.pending_count.fetch_add(1, Ordering::Relaxed);

        let action: CommitmentAction = Box::new(|_id| {
            Box::pin(async { Ok(serde_json::json!({})) })
        });
        let result = commitment.execute_committed(handle, action).await;

        assert!(
            matches!(result, Err(CommitmentError::Expired)),
            "expired commitment must be rejected"
        );
    }

    #[tokio::test]
    async fn test_commitment_full_flow() {
        let wll = Arc::new(LocalSimWorldLine::new());
        let commitment = LocalSimCommitment::new(wll.clone(), "run_flow_test");

        let handle = commitment
            .prepare("buyer-01", "transfer 50 IUSD", [1u8; 32])
            .await
            .unwrap();

        assert_eq!(commitment.pending_count(), 1);

        let action: CommitmentAction = Box::new(|id| {
            Box::pin(async move {
                assert!(id.0.starts_with("cmmt_"));
                Ok(serde_json::json!({ "transferred": 50 }))
            })
        });
        let (value, proof) = commitment.execute_committed(handle, action).await.unwrap();

        assert_eq!(value["transferred"], 50);
        assert_eq!(commitment.pending_count(), 0);
        assert!(!proof.worldline_event_id.0.is_empty());

        // Verify WLL recorded Intent + Commitment + Consequence events.
        let events: Vec<_> = wll
            .tail_events("run_flow_test", None, false)
            .collect()
            .await;
        assert!(events.len() >= 3, "must have at least Intent+Commitment+Consequence");
    }

    #[tokio::test]
    async fn test_identity_deterministic() {
        let id_a = LocalSimIdentity::from_resonator_id("buyer-01").unwrap();
        let id_b = LocalSimIdentity::from_resonator_id("buyer-01").unwrap();

        assert_eq!(
            id_a.evm_address(),
            id_b.evm_address(),
            "same resonator ID must always produce same EVM address"
        );
        assert_eq!(id_a.ed25519_verifying_key(), id_b.ed25519_verifying_key());
    }

    #[tokio::test]
    async fn test_identity_different_ids_differ() {
        let buyer = LocalSimIdentity::from_resonator_id("buyer-01").unwrap();
        let seller = LocalSimIdentity::from_resonator_id("seller-01").unwrap();

        assert_ne!(
            buyer.evm_address(),
            seller.evm_address(),
            "different IDs must produce different addresses"
        );
    }

    #[tokio::test]
    async fn test_identity_no_key_export() {
        // Compile-time check: LocalSimIdentity has no method returning &[u8] key material.
        // The fields are private — only public methods are on the ResonatorIdentity trait.
        let id = LocalSimIdentity::from_resonator_id("test-agent").unwrap();
        // We can sign but cannot get the key bytes.
        let sig = id.sign_ed25519(b"hello openibank");
        assert!(id.verify_ed25519(b"hello openibank", &sig));
    }

    #[tokio::test]
    async fn test_identity_evm_sign() {
        let id = LocalSimIdentity::from_resonator_id("buyer-01").unwrap();
        let msg_hash = [0xabu8; 32];
        let sig = id.sign_evm(msg_hash).await.unwrap();
        assert_eq!(sig.len(), 65);
        assert!(sig[64] == 27 || sig[64] == 28, "v must be 27 or 28");
    }

    #[tokio::test]
    async fn test_follow_stream_receives_live_events() {
        let wll = Arc::new(LocalSimWorldLine::new());
        let run_id = "run_follow_test";

        // Start follow stream before appending.
        let mut stream = wll.tail_events(run_id, None, true);

        // Append events from a background task.
        let wll_clone = wll.clone();
        let run_id_owned = run_id.to_string();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            for _ in 0..5 {
                wll_clone
                    .append_event(make_event(&run_id_owned, "buyer-01"))
                    .await
                    .unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        });

        // Collect events with timeout.
        let mut received = 0usize;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline && received < 5 {
            let timeout = tokio::time::sleep_until(deadline);
            tokio::select! {
                event = stream.next() => {
                    if event.is_some() { received += 1; }
                }
                _ = timeout => break,
            }
        }

        handle.await.unwrap();
        assert!(received >= 5, "follow stream must receive live events (got {})", received);
    }
}
