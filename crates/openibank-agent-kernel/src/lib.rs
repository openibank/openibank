//! OpeniBank AgentKernel - policy, proposal, gating, and trace runtime
//!
//! The kernel owns the agent event loop, validates proposals, enforces
//! capability/contract gates, and emits audit-ready traces.

pub mod gate;
pub mod io;
pub mod kernel;
pub mod policy;
pub mod propose;
pub mod trace;

pub use gate::{
    CapabilitySet, CapabilityStatus, CommitmentContext, CommitmentGate, Contract, ContractSet,
    GateDecision, GateError,
};
pub use io::{InProcBus, KernelMessage, KernelResponse};
pub use kernel::{AgentKernel, KernelConfig, KernelError, KernelMode};
pub use policy::{DeterministicPolicy, KernelAction, KernelIntent, KernelPolicy, PolicyDecision};
pub use propose::{KernelProposal, KernelProposer, ProposalRequest, ProposeError};
pub use trace::{KernelStage, KernelTrace, KernelTraceEvent};
