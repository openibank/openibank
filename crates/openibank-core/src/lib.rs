//! OpeniBank Core - Canonical types and wallet primitives for AI-agent banking
//!
//! This crate implements the foundational economic primitives for OpeniBank:
//! - AssetObject: unified representation of crypto and non-crypto assets
//! - BudgetPolicy: bounded spending authority for agents
//! - SpendPermit: agent-native "currency of authority"
//! - PaymentIntent: proposed payment before authorization
//! - CommitmentReceipt: proof-carrying evidence of authorized actions
//! - EvidenceBundle: inputs, attestations, and signatures for audit
//!
//! # Architectural Invariants
//!
//! 1. Resonator is the only economic actor
//! 2. No direct settlement without commitment
//! 3. SpendPermits are mandatory
//! 4. All money-impacting actions emit verifiable receipts
//! 5. Authority is always bounded
//! 6. Fail closed

pub mod types;
pub mod wallet;
pub mod commitment;
pub mod crypto;
pub mod error;

pub use types::*;
pub use wallet::*;
pub use commitment::*;
pub use crypto::*;
pub use error::*;
