# Tutorial 9: Multi-Agent Coordination

> **Duration**: 60 minutes
> **Level**: Advanced
> **Prerequisites**: Tutorials 1-8, Understanding of distributed systems and game theory

---

## Overview

Multi-agent coordination is at the heart of OpeniBank's vision. In this tutorial, you'll learn to:

- Design multi-agent systems with Maple Resonators
- Implement agent communication protocols
- Build collaborative trading strategies
- Handle agent conflicts and disputes
- Create market-making agent networks

---

## Understanding Multi-Agent Systems

```
┌─────────────────────────────────────────────────────────────────┐
│                Multi-Agent Coordination Layer                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌────────────┐     ┌────────────┐     ┌────────────┐         │
│   │  Buyer A   │     │  Seller B  │     │ Arbiter C  │         │
│   │ Resonator  │     │ Resonator  │     │ Resonator  │         │
│   └─────┬──────┘     └─────┬──────┘     └─────┬──────┘         │
│         │                  │                  │                 │
│   ┌─────▼──────────────────▼──────────────────▼─────┐          │
│   │              Resonance Coupling Layer            │          │
│   │                                                  │          │
│   │   • Message Routing    • State Synchronization  │          │
│   │   • Commitment Tracking • Conflict Resolution   │          │
│   └──────────────────────────────────────────────────┘          │
│                           │                                      │
│   ┌───────────────────────▼──────────────────────────┐          │
│   │              Authority & Accountability          │          │
│   │                     Service (AAS)                │          │
│   │                                                  │          │
│   │   • Identity    • Capability Grants   • Audit   │          │
│   └──────────────────────────────────────────────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step 1: Agent Communication with Resonators

### Creating Coupled Agents

```rust
use openibank_maple::{Resonator, ResonanceChannel, Message};
use openibank_agents::{BuyerAgent, SellerAgent, ArbiterAgent};

async fn create_coupled_agents() -> Result<(BuyerAgent, SellerAgent, ArbiterAgent), Box<dyn std::error::Error>> {
    // Create resonators (agent identities)
    let buyer_resonator = Resonator::new("buyer_alice");
    let seller_resonator = Resonator::new("seller_datacorp");
    let arbiter_resonator = Resonator::new("arbiter_veritas");

    // Create agents with resonators
    let buyer = BuyerAgent::with_resonator(buyer_resonator.clone());
    let seller = SellerAgent::with_resonator(seller_resonator.clone());
    let arbiter = ArbiterAgent::with_resonator(arbiter_resonator.clone());

    // Establish resonance coupling for the trade
    let coupling = ResonanceCoupling::new()
        .add_resonator(buyer_resonator, Role::Buyer)
        .add_resonator(seller_resonator, Role::Seller)
        .add_resonator(arbiter_resonator, Role::Arbiter)
        .build()?;

    println!("Coupling established: {}", coupling.id());

    Ok((buyer, seller, arbiter))
}
```

### Message Passing

```rust
use openibank_maple::{Message, MessageType};

async fn agent_communication(
    buyer: &BuyerAgent,
    seller: &SellerAgent,
) -> Result<(), Box<dyn std::error::Error>> {
    // Buyer sends intent to purchase
    let intent = Message::new(MessageType::Intent)
        .from(buyer.resonator_id())
        .to(seller.resonator_id())
        .payload(json!({
            "action": "purchase",
            "service": "Data Analysis",
            "max_price": 10000,
        }));

    buyer.send(intent).await?;

    // Seller receives and responds with offer
    let offer = seller.receive().await?;

    if offer.message_type == MessageType::Offer {
        let offer_details: OfferDetails = serde_json::from_value(offer.payload)?;

        println!("Received offer: {} IUSD for {}", offer_details.price, offer_details.service);

        // Buyer evaluates and accepts
        if offer_details.price <= 10000 {
            let acceptance = Message::new(MessageType::Accept)
                .from(buyer.resonator_id())
                .to(seller.resonator_id())
                .reference(offer.id)
                .payload(json!({
                    "accepted_price": offer_details.price,
                }));

            buyer.send(acceptance).await?;
        }
    }

    Ok(())
}
```

---

## Step 2: Commitment Framework (RCF)

### Creating Commitments

```rust
use openibank_maple::rcf::{Commitment, CommitmentState, CommitmentBuilder};

async fn create_trade_commitment(
    buyer: &BuyerAgent,
    seller: &SellerAgent,
    arbiter: &ArbiterAgent,
    amount: Decimal,
    service: &str,
) -> Result<Commitment, Box<dyn std::error::Error>> {
    // Build commitment
    let commitment = CommitmentBuilder::new()
        .commitment_type("trade")
        .parties([
            (buyer.resonator_id(), Role::Buyer),
            (seller.resonator_id(), Role::Seller),
            (arbiter.resonator_id(), Role::Arbiter),
        ])
        .terms(json!({
            "service": service,
            "price": amount.to_string(),
            "delivery_deadline": Utc::now() + Duration::hours(24),
            "dispute_window": Duration::hours(48),
        }))
        .escrow(EscrowConfig {
            amount,
            release_conditions: vec![
                "service_delivered".to_string(),
                "quality_verified".to_string(),
            ],
        })
        .build()?;

    // All parties sign
    let mut signed = commitment;
    signed.sign(buyer.private_key())?;
    signed.sign(seller.private_key())?;
    signed.sign(arbiter.private_key())?;

    // Submit to AAS
    let aas = AasClient::new("http://localhost:8080/aas");
    aas.submit_commitment(&signed).await?;

    println!("Commitment {} submitted", signed.id());

    Ok(signed)
}
```

### Commitment Lifecycle

```rust
async fn manage_commitment_lifecycle(
    commitment: &Commitment,
    aas: &AasClient,
) -> Result<(), Box<dyn std::error::Error>> {
    // Monitor commitment state
    let mut state_stream = aas.subscribe_commitment(commitment.id()).await?;

    while let Some(event) = state_stream.next().await {
        match event.new_state {
            CommitmentState::Pending => {
                println!("Commitment pending signatures");
            }
            CommitmentState::Active => {
                println!("Commitment is now active");
            }
            CommitmentState::InProgress => {
                println!("Work in progress");
            }
            CommitmentState::Delivered => {
                println!("Service delivered, awaiting verification");
            }
            CommitmentState::Completed => {
                println!("Commitment fulfilled successfully!");
                break;
            }
            CommitmentState::Disputed => {
                println!("Dispute raised: {:?}", event.details);
                // Arbiter will handle
            }
            CommitmentState::Resolved => {
                println!("Dispute resolved: {:?}", event.resolution);
            }
            CommitmentState::Cancelled => {
                println!("Commitment cancelled");
                break;
            }
        }
    }

    Ok(())
}
```

---

## Step 3: Collaborative Trading Strategies

### Agent Swarm Trading

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct TradingSwarm {
    agents: Vec<Arc<TradingAgent>>,
    coordinator: SwarmCoordinator,
    shared_state: Arc<RwLock<SwarmState>>,
}

impl TradingSwarm {
    async fn run_collaborative_strategy(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Each agent analyzes different aspects
        let analysis_tasks: Vec<_> = self.agents.iter().map(|agent| {
            let agent = agent.clone();
            let state = self.shared_state.clone();

            tokio::spawn(async move {
                let analysis = agent.analyze_market().await?;
                state.write().await.update_analysis(agent.id(), analysis);
                Ok::<_, Box<dyn std::error::Error>>(())
            })
        }).collect();

        // Wait for all analyses
        futures::future::try_join_all(analysis_tasks).await?;

        // Coordinator makes decision based on consensus
        let consensus = self.coordinator.build_consensus(&self.shared_state.read().await).await?;

        if consensus.confidence > 0.7 {
            println!("High confidence signal: {:?}", consensus.signal);

            // Distribute order across agents
            let order_size = consensus.recommended_size;
            let agent_count = self.agents.len();
            let size_per_agent = order_size / Decimal::from(agent_count);

            for agent in &self.agents {
                agent.execute_order(
                    &consensus.symbol,
                    consensus.signal.direction,
                    size_per_agent,
                ).await?;
            }
        }

        Ok(())
    }
}

struct SwarmCoordinator;

impl SwarmCoordinator {
    async fn build_consensus(&self, state: &SwarmState) -> Result<ConsensusResult, Box<dyn std::error::Error>> {
        let analyses: Vec<_> = state.analyses.values().collect();

        // Voting on direction
        let mut bullish_votes = 0;
        let mut bearish_votes = 0;
        let mut neutral_votes = 0;

        for analysis in &analyses {
            match analysis.signal {
                Signal::Bullish => bullish_votes += 1,
                Signal::Bearish => bearish_votes += 1,
                Signal::Neutral => neutral_votes += 1,
            }
        }

        let total = analyses.len();
        let confidence = (bullish_votes.max(bearish_votes) as f64) / (total as f64);

        let signal = if bullish_votes > bearish_votes && bullish_votes > neutral_votes {
            ConsensusSignal { direction: OrderSide::Buy }
        } else if bearish_votes > bullish_votes && bearish_votes > neutral_votes {
            ConsensusSignal { direction: OrderSide::Sell }
        } else {
            return Ok(ConsensusResult {
                signal: ConsensusSignal { direction: OrderSide::Buy },
                confidence: 0.0,
                symbol: String::new(),
                recommended_size: dec!(0),
            });
        };

        Ok(ConsensusResult {
            signal,
            confidence,
            symbol: state.target_symbol.clone(),
            recommended_size: state.calculate_position_size(confidence),
        })
    }
}
```

### Market Making Network

```rust
struct MarketMakingNetwork {
    makers: Vec<MarketMaker>,
    inventory_manager: InventoryManager,
    price_oracle: PriceOracle,
}

impl MarketMakingNetwork {
    async fn coordinate_quotes(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get reference price
        let mid_price = self.price_oracle.get_mid_price().await?;

        // Calculate network inventory
        let total_inventory = self.inventory_manager.get_total_inventory().await?;

        // Each maker provides quotes at different levels
        for (i, maker) in self.makers.iter().enumerate() {
            let level = i as u32 + 1;

            // Spread increases with level
            let spread = dec!(0.001) * Decimal::from(level);

            // Size decreases with level
            let size = dec!(1.0) / Decimal::from(level);

            // Adjust for network inventory
            let inventory_skew = total_inventory.skew();

            let bid = mid_price * (dec!(1) - spread - inventory_skew * dec!(0.0005));
            let ask = mid_price * (dec!(1) + spread - inventory_skew * dec!(0.0005));

            maker.update_quotes(bid, ask, size).await?;
        }

        Ok(())
    }

    async fn rebalance_inventory(&self) -> Result<(), Box<dyn std::error::Error>> {
        let inventories = self.inventory_manager.get_all_inventories().await?;

        // Find makers with excess and deficit
        let mut excess_makers: Vec<_> = inventories.iter()
            .filter(|(_, inv)| inv.imbalance() > dec!(0.1))
            .collect();

        let mut deficit_makers: Vec<_> = inventories.iter()
            .filter(|(_, inv)| inv.imbalance() < dec!(-0.1))
            .collect();

        // Match excess with deficit
        while !excess_makers.is_empty() && !deficit_makers.is_empty() {
            let (excess_id, excess_inv) = excess_makers.pop().unwrap();
            let (deficit_id, deficit_inv) = deficit_makers.pop().unwrap();

            let transfer_amount = excess_inv.excess().min(deficit_inv.deficit());

            // Internal transfer (no fees)
            self.inventory_manager.internal_transfer(
                excess_id,
                deficit_id,
                transfer_amount,
            ).await?;

            println!("Rebalanced {} from {} to {}", transfer_amount, excess_id, deficit_id);
        }

        Ok(())
    }
}
```

---

## Step 4: Conflict Resolution

### Dispute Handling

```rust
use openibank_agents::ArbiterAgent;

impl ArbiterAgent {
    async fn handle_dispute(&self, dispute: Dispute) -> Result<Resolution, Box<dyn std::error::Error>> {
        println!("Handling dispute: {}", dispute.id);

        // Gather evidence from both parties
        let buyer_evidence = self.request_evidence(&dispute.buyer_id).await?;
        let seller_evidence = self.request_evidence(&dispute.seller_id).await?;

        // Analyze with LLM
        let analysis = self.llm.analyze_dispute(
            &dispute,
            &buyer_evidence,
            &seller_evidence,
        ).await?;

        // Make decision
        let resolution = match analysis.recommendation {
            Recommendation::RefundBuyer => {
                Resolution {
                    dispute_id: dispute.id,
                    outcome: Outcome::RefundFull,
                    rationale: analysis.rationale,
                    escrow_action: EscrowAction::ReleaseToBuyer,
                }
            }
            Recommendation::PaySeller => {
                Resolution {
                    dispute_id: dispute.id,
                    outcome: Outcome::PaySeller,
                    rationale: analysis.rationale,
                    escrow_action: EscrowAction::ReleaseToSeller,
                }
            }
            Recommendation::Split { buyer_pct, seller_pct } => {
                Resolution {
                    dispute_id: dispute.id,
                    outcome: Outcome::PartialRefund { buyer_pct, seller_pct },
                    rationale: analysis.rationale,
                    escrow_action: EscrowAction::Split { buyer_pct, seller_pct },
                }
            }
            Recommendation::NeedsHumanReview => {
                self.escalate_to_human(&dispute).await?;
                return Ok(Resolution::pending_human_review(dispute.id));
            }
        };

        // Sign and submit resolution
        let signed = resolution.sign(self.private_key())?;
        self.submit_resolution(signed.clone()).await?;

        // Execute escrow action
        self.execute_escrow_action(&signed.escrow_action, &dispute.escrow_id).await?;

        Ok(resolution)
    }

    async fn analyze_dispute(
        &self,
        dispute: &Dispute,
        buyer_evidence: &Evidence,
        seller_evidence: &Evidence,
    ) -> Result<DisputeAnalysis, Box<dyn std::error::Error>> {
        let prompt = format!(r#"
Analyze this trade dispute and recommend a resolution:

Dispute Details:
- ID: {}
- Amount: {} IUSD
- Service: {}
- Buyer claim: {}
- Seller claim: {}

Buyer Evidence:
{}

Seller Evidence:
{}

Consider:
1. Was the service delivered as specified?
2. Was the quality acceptable?
3. Did either party violate terms?
4. What is the fair resolution?

Respond with JSON:
{{
  "recommendation": "refund_buyer" | "pay_seller" | "split",
  "buyer_share": 0-100,
  "seller_share": 0-100,
  "rationale": "explanation",
  "confidence": 0.0-1.0
}}
"#,
            dispute.id,
            dispute.amount,
            dispute.service,
            dispute.buyer_claim,
            dispute.seller_claim,
            serde_json::to_string_pretty(buyer_evidence)?,
            serde_json::to_string_pretty(seller_evidence)?,
        );

        let response = self.llm.complete(&prompt).await?;
        let analysis: DisputeAnalysis = serde_json::from_str(&response)?;

        Ok(analysis)
    }
}
```

### Consensus Mechanisms

```rust
struct MultiArbiterConsensus {
    arbiters: Vec<ArbiterAgent>,
    quorum_size: usize,
    timeout: Duration,
}

impl MultiArbiterConsensus {
    async fn resolve_dispute(&self, dispute: Dispute) -> Result<Resolution, Box<dyn std::error::Error>> {
        // Request resolution from all arbiters
        let resolution_futures: Vec<_> = self.arbiters.iter().map(|arbiter| {
            let dispute = dispute.clone();
            async move {
                tokio::time::timeout(
                    self.timeout,
                    arbiter.propose_resolution(&dispute),
                ).await.ok().flatten()
            }
        }).collect();

        let resolutions: Vec<_> = futures::future::join_all(resolution_futures)
            .await
            .into_iter()
            .flatten()
            .collect();

        // Check quorum
        if resolutions.len() < self.quorum_size {
            return Err("Failed to reach quorum".into());
        }

        // Count votes for each outcome
        let mut votes: HashMap<Outcome, Vec<&Resolution>> = HashMap::new();
        for resolution in &resolutions {
            votes.entry(resolution.outcome.clone())
                .or_default()
                .push(resolution);
        }

        // Find majority
        let (winning_outcome, winning_votes) = votes.into_iter()
            .max_by_key(|(_, v)| v.len())
            .ok_or("No votes")?;

        if winning_votes.len() >= self.quorum_size {
            // Consensus reached
            let final_resolution = Resolution {
                outcome: winning_outcome,
                rationale: format!(
                    "Consensus reached: {} of {} arbiters agreed",
                    winning_votes.len(),
                    self.arbiters.len()
                ),
                signatures: winning_votes.iter()
                    .map(|r| r.signature.clone())
                    .collect(),
                ..winning_votes[0].clone()
            };

            Ok(final_resolution)
        } else {
            // Escalate to higher authority
            Err("Failed to reach consensus".into())
        }
    }
}
```

---

## Step 5: Agent Ecosystem Patterns

### Hub-and-Spoke Model

```rust
struct HubAgent {
    id: ResonatorId,
    spoke_agents: HashMap<ResonatorId, SpokeAgent>,
    task_queue: TaskQueue,
}

impl HubAgent {
    async fn distribute_work(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(task) = self.task_queue.pop().await {
            // Find best spoke for task
            let best_spoke = self.find_best_spoke(&task).await?;

            // Assign task
            let assignment = TaskAssignment {
                task_id: task.id.clone(),
                spoke_id: best_spoke.id.clone(),
                deadline: task.deadline,
            };

            best_spoke.assign(task.clone()).await?;

            println!("Assigned {} to {}", task.id, best_spoke.id);
        }

        Ok(())
    }

    async fn find_best_spoke(&self, task: &Task) -> Result<&SpokeAgent, Box<dyn std::error::Error>> {
        let mut best_score = 0.0;
        let mut best_spoke = None;

        for spoke in self.spoke_agents.values() {
            let score = spoke.score_for_task(task).await?;
            if score > best_score {
                best_score = score;
                best_spoke = Some(spoke);
            }
        }

        best_spoke.ok_or("No suitable spoke found".into())
    }

    async fn aggregate_results(&self) -> Result<AggregatedResult, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for spoke in self.spoke_agents.values() {
            let spoke_results = spoke.get_results().await?;
            results.extend(spoke_results);
        }

        Ok(AggregatedResult {
            total_tasks: results.len(),
            successful: results.iter().filter(|r| r.success).count(),
            failed: results.iter().filter(|r| !r.success).count(),
            total_value: results.iter().map(|r| r.value).sum(),
        })
    }
}
```

### Peer-to-Peer Network

```rust
struct P2PAgentNetwork {
    local_agent: P2PAgent,
    peers: HashMap<ResonatorId, PeerConnection>,
    gossip: GossipProtocol,
}

impl P2PAgentNetwork {
    async fn broadcast_opportunity(&self, opportunity: Opportunity) -> Result<(), Box<dyn std::error::Error>> {
        // Gossip to all peers
        for peer in self.peers.values() {
            peer.send(Message::Opportunity(opportunity.clone())).await?;
        }

        Ok(())
    }

    async fn negotiate_with_peer(&self, peer_id: &ResonatorId, offer: Offer) -> Result<Agreement, Box<dyn std::error::Error>> {
        let peer = self.peers.get(peer_id)
            .ok_or("Peer not found")?;

        // Send offer
        peer.send(Message::Offer(offer.clone())).await?;

        // Wait for response
        let response = peer.receive().await?;

        match response {
            Message::Accept(acceptance) => {
                // Create agreement
                let agreement = Agreement {
                    offer: offer.clone(),
                    acceptance,
                    parties: vec![self.local_agent.id.clone(), peer_id.clone()],
                };

                // Sign and exchange signatures
                let signed = agreement.sign(self.local_agent.private_key())?;
                peer.send(Message::Signed(signed.clone())).await?;

                let peer_signature = match peer.receive().await? {
                    Message::Signed(s) => s,
                    _ => return Err("Expected signature".into()),
                };

                let final_agreement = signed.add_signature(peer_signature)?;

                Ok(final_agreement)
            }
            Message::Counter(counter_offer) => {
                // Evaluate counter-offer
                if self.local_agent.evaluate_offer(&counter_offer).await? {
                    self.negotiate_with_peer(peer_id, counter_offer).await
                } else {
                    Err("Could not reach agreement".into())
                }
            }
            Message::Reject(reason) => {
                Err(format!("Offer rejected: {}", reason).into())
            }
            _ => Err("Unexpected response".into()),
        }
    }
}
```

---

## Step 6: Complete Multi-Agent Trading System

```rust
use openibank_maple::*;
use openibank_agents::*;

struct MultiAgentTradingSystem {
    buyers: Vec<BuyerAgent>,
    sellers: Vec<SellerAgent>,
    arbiters: Vec<ArbiterAgent>,
    market_makers: Vec<MarketMaker>,
    coordinator: SystemCoordinator,
    aas: AasClient,
}

impl MultiAgentTradingSystem {
    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting multi-agent trading system...");

        // Initialize all agents
        for buyer in &mut self.buyers {
            buyer.register_with_aas(&self.aas).await?;
        }
        for seller in &mut self.sellers {
            seller.register_with_aas(&self.aas).await?;
        }
        for arbiter in &mut self.arbiters {
            arbiter.register_with_aas(&self.aas).await?;
        }

        // Start market makers
        for mm in &self.market_makers {
            tokio::spawn({
                let mm = mm.clone();
                async move {
                    mm.run_market_making_loop().await
                }
            });
        }

        // Main trading loop
        loop {
            // Match buyers with sellers
            let matches = self.coordinator.find_matches(&self.buyers, &self.sellers).await?;

            for (buyer, seller, service) in matches {
                // Create trade commitment
                let arbiter = self.coordinator.assign_arbiter(&self.arbiters).await?;

                let commitment = self.create_trade(buyer, seller, arbiter, &service).await?;

                // Spawn trade execution
                tokio::spawn(async move {
                    if let Err(e) = execute_trade(commitment).await {
                        eprintln!("Trade failed: {}", e);
                    }
                });
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn create_trade(
        &self,
        buyer: &BuyerAgent,
        seller: &SellerAgent,
        arbiter: &ArbiterAgent,
        service: &Service,
    ) -> Result<Commitment, Box<dyn std::error::Error>> {
        // Establish coupling
        let coupling = ResonanceCoupling::new()
            .add(buyer.resonator(), Role::Buyer)
            .add(seller.resonator(), Role::Seller)
            .add(arbiter.resonator(), Role::Arbiter)
            .build()?;

        // Create commitment
        let commitment = CommitmentBuilder::new()
            .coupling(coupling)
            .service(service.clone())
            .price(service.price)
            .build()?;

        // All parties sign
        let signed = commitment
            .sign(buyer.key())?
            .sign(seller.key())?
            .sign(arbiter.key())?;

        // Submit to AAS
        self.aas.submit(&signed).await?;

        Ok(signed)
    }
}

async fn execute_trade(commitment: Commitment) -> Result<(), Box<dyn std::error::Error>> {
    // Lock escrow
    let escrow = Escrow::from_commitment(&commitment)?;
    escrow.lock().await?;

    // Seller delivers
    let delivery = commitment.seller().deliver(&commitment).await?;

    // Buyer verifies
    if commitment.buyer().verify_delivery(&delivery).await? {
        // Release escrow to seller
        escrow.release_to_seller().await?;
        commitment.complete().await?;
    } else {
        // Raise dispute
        let dispute = Dispute::new(&commitment, "Delivery not satisfactory");
        commitment.arbiter().handle(&dispute).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut system = MultiAgentTradingSystem::new().await?;
    system.run().await
}
```

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `COUPLING_FAILED` | Resonator mismatch | Verify all resonators are registered |
| `COMMITMENT_REJECTED` | Invalid signatures | Check key pairs and signing order |
| `CONSENSUS_TIMEOUT` | Network issues | Increase timeout, check connectivity |
| `DEADLOCK` | Circular dependencies | Review agent dependencies |

---

## Best Practices

1. **Clear role separation** - Each agent type has distinct responsibilities
2. **Fail-safe defaults** - Always have fallback behaviors
3. **Audit everything** - All inter-agent communication should be logged
4. **Test isolation** - Test agents individually before integration
5. **Monitor resource usage** - Multi-agent systems can be resource-intensive

---

## Next Steps

- [Tutorial 10: Security & Compliance](./10-security.md)
- [Maple AI Framework Documentation](https://maple.ai/docs)
- [AAS API Reference](../api/README.md#aas)
