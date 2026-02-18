# Maple API Inventory

Generated: 2026-02-17
Maple commit: 7c39d16

## Discovered Crates

| Crate Name | Path | Relevant APIs |
|---|---|---|
| resonator-types | crates/resonator/types | Resonator, ResonatorId, ResonatorProfile, AttentionBudget, CouplingGraph |
| resonator-commitment | crates/resonator/commitment | ContractEngine (trait), ContractLifecycleManager, StoredContract, ContractStatus, InMemoryContractEngine |
| resonator-intent | crates/resonator/intent | IntentStabilizationEngine, IntentCandidate, StabilizedIntent, IntentHistory |
| resonator-consequence | crates/resonator/consequence | ConsequenceStore (trait), ConsequenceTracker, RecordedConsequence, ConsequenceReceipt |
| resonator-identity | crates/resonator/identity | stub only — see LocalSimIdentity |
| rcf-types | crates/rcf/types | CapabilityRef, EffectDomain, IdentityRef, ScopeConstraint, TemporalValidity, ResourceLimits |
| rcf-commitment | crates/rcf/commitment | RcfCommitment, CommitmentId, CommitmentBuilder, Reversibility, AuditLevel |
| maple-runtime | crates/maple-runtime | MapleRuntime, ResonatorHandle, CommitmentGateway, ibank_runtime_config() |
| worldline-core | crates/worldline/core | WorldlineId, IdentityMaterial, IdentityManager, EffectDomain, CommitmentScope |
| worldline-runtime | crates/worldline/runtime | EventFabric, CommitmentGate, KernelEvent, ResonanceStage, EventPayload |
| worldline-ledger | crates/worldline/ledger | Ledger primitives for WorldLine |
| aas-types | crates/aas/types | AgentId, CommitmentOutcome, PolicyDecisionCard |
| aas-service | crates/aas/service | AasError |
| aas-identity | crates/aas/identity | Identity registration |
| aas-capability | crates/aas/capability | Capability grants |

## WorldLine / WAL

**STATUS: FOUND — using MapleNative WorldLine via worldline-runtime**

- `worldline_runtime::fabric::EventFabric` — append events to WorldLine
  - `emit(worldline_id, stage, payload, parents) -> KernelEvent`
  - `subscribe(filter, options) -> EventStream`
  - `FabricConfig { data_dir: Option<PathBuf> }` — disk or memory WAL
- `worldline_runtime::gate::CommitmentGate` — enforce commitment boundary
  - Multi-stage pipeline: Declaration → Identity Binding → Capability Check → Policy → Risk → Co-Signature → Final Decision
  - `submit(declaration) -> AdjudicationResult`
  - `record_outcome(commitment_id, outcome)`
- `worldline_core::identity::IdentityManager` — deterministic WorldlineId per agent
  - `create_worldline(IdentityMaterial::GenesisHash([u8;32])) -> WorldlineId`

## Commitment Gate

**STATUS: FOUND — using MapleNative CommitmentGate**

- `worldline_runtime::gate::CommitmentGate` with configurable stages
- Stages: `DeclarationStage`, `IdentityBindingStage`, `CapabilityCheckStage`, `PolicyEvaluationStage`, `RiskAssessmentStage`, `CoSignatureStage`, `FinalDecisionStage`
- `AdjudicationResult::Approved { .. }` is the only path to consequence execution
- Enforces invariant: **No consequence without commitment approval**

## Resonator Identity

**STATUS: resonator-identity is a stub — LocalSimIdentity used for key operations**

- `worldline_core` provides `WorldlineId` (identity trajectory)
- Key signing (ed25519, secp256k1) is implemented via `LocalSimIdentity` using `ed25519-dalek` and `k256`
- EVM address derivation: `keccak256(secp256k1_uncompressed_pubkey)[12..]`

## Backend Selection

```
OPENIBANK_MODE=local-sim    → LocalSimBackend (embedded sled WAL, no Maple runtime needed)
OPENIBANK_MODE=maple-native → MapleNativeBackend (full WorldLine + CommitmentGate)
auto (default)              → MapleNativeBackend, falls back to LocalSimBackend on init error
```

## OpeniBank Adapter Traits (openibank-maple/src/traits.rs)

These stable traits isolate all other OpeniBank crates from Maple API changes:

- `WorldLineBackend` — append_event, tail_events, export_slice, latest_event_id
- `CommitmentBackend` — prepare, execute, fail
- `ResonatorIdentity` — resonator_id, evm_address, sign_ed25519, verify_ed25519, sign_evm
