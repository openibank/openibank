//! In-proc message bus for AgentKernel (pluggable later)

use tokio::sync::mpsc;

use crate::policy::KernelAction;
use crate::propose::{KernelProposal, ProposalRequest};

#[derive(Debug, Clone)]
pub enum KernelMessage {
    Proposal(ProposalRequest),
    Action(KernelAction),
}

#[derive(Debug, Clone)]
pub enum KernelResponse {
    ProposalResult(Result<KernelProposal, String>),
    ActionResult(Result<(), String>),
}

pub struct InProcBus {
    sender: mpsc::Sender<KernelMessage>,
    receiver: mpsc::Receiver<KernelMessage>,
}

impl InProcBus {
    pub fn new(buffer: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer);
        Self { sender, receiver }
    }

    pub fn sender(&self) -> mpsc::Sender<KernelMessage> {
        self.sender.clone()
    }

    pub async fn recv(&mut self) -> Option<KernelMessage> {
        self.receiver.recv().await
    }
}
