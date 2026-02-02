//! OpeniBank Agents - Reference Agent Implementations
//!
//! This crate provides openclaw-style reference agents that demonstrate
//! the viral agent commerce flow:
//!
//! - **BuyerAgent**: Has a budget, issues SpendPermits, pays via escrow
//! - **SellerAgent**: Publishes services, issues invoices, delivers
//! - **ArbiterAgent**: Validates delivery, resolves disputes
//!
//! # Key Principle
//!
//! **LLMs may PROPOSE intents, NEVER EXECUTE money.**
//!
//! All agents follow deterministic behavior when no LLM is available.
//! LLMs can only suggest actions that are then validated and executed
//! through the commitment gate.

pub mod brain;
pub mod buyer;
pub mod seller;
pub mod arbiter;

pub use brain::*;
pub use buyer::{BuyerAgent, BuyerError, ServiceOffer};
pub use seller::{DeliveryProof, SellerAgent, SellerError, Service};
pub use arbiter::{ArbiterAgent, ArbiterError, DecisionResult, DisputeCase};
