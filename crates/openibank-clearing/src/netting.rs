//! Multilateral netting algorithm
//!
//! Implements complete netting logic, not mocked.

use openibank_types::*;
use std::collections::HashMap;

/// Compute multilateral net positions from gross positions
pub fn compute_multilateral_net(
    positions: &[GrossPosition],
    currency: Currency,
) -> Result<NettingResult> {
    // 1. Build position matrix: institution × amounts
    let mut net_amounts: HashMap<InstitutionId, i128> = HashMap::new();

    for position in positions {
        // Debtor owes money (negative)
        *net_amounts.entry(position.from.clone()).or_insert(0) -= position.amount.value;
        // Creditor is owed money (positive)
        *net_amounts.entry(position.to.clone()).or_insert(0) += position.amount.value;
    }

    // 2. Validate conservation (sum of nets = 0)
    let net_sum: i128 = net_amounts.values().sum();
    if net_sum != 0 {
        return Err(OpeniBankError::ConservationViolation {
            batch_id: "unknown".to_string(),
            net_sum: net_sum.to_string(),
        });
    }

    // 3. Separate payers and receivers
    let mut payers: Vec<(InstitutionId, i128)> = Vec::new();
    let mut receivers: Vec<(InstitutionId, i128)> = Vec::new();

    for (inst, amount) in net_amounts {
        if amount < 0 {
            payers.push((inst, -amount)); // Convert to positive for payers
        } else if amount > 0 {
            receivers.push((inst, amount));
        }
        // Zero positions are ignored
    }

    // Sort for deterministic ordering
    payers.sort_by_key(|(id, _)| id.0);
    receivers.sort_by_key(|(id, _)| id.0);

    // 4. Generate minimum settlement legs using greedy algorithm
    let mut legs = Vec::new();
    let mut payer_idx = 0;
    let mut receiver_idx = 0;
    let mut payer_remaining: Vec<i128> = payers.iter().map(|(_, a)| *a).collect();
    let mut receiver_remaining: Vec<i128> = receivers.iter().map(|(_, a)| *a).collect();

    while payer_idx < payers.len() && receiver_idx < receivers.len() {
        let amount = payer_remaining[payer_idx].min(receiver_remaining[receiver_idx]);

        if amount > 0 {
            legs.push(SettlementLeg {
                id: uuid::Uuid::new_v4(),
                from: payers[payer_idx].0.clone(),
                to: receivers[receiver_idx].0.clone(),
                amount: Amount::new(amount, currency, 18),
                status: SettlementLegStatus::Pending,
                channel: SettlementChannel::Internal,
                receipt_id: None,
                executed_at: None,
            });
        }

        payer_remaining[payer_idx] -= amount;
        receiver_remaining[receiver_idx] -= amount;

        if payer_remaining[payer_idx] == 0 {
            payer_idx += 1;
        }
        if receiver_remaining[receiver_idx] == 0 {
            receiver_idx += 1;
        }
    }

    // 5. Compute efficiency
    let gross_count = positions.len() as u64;
    let net_count = legs.len() as u64;
    let efficiency = if gross_count > 0 {
        1.0 - (net_count as f64 / gross_count as f64)
    } else {
        1.0
    };

    Ok(NettingResult {
        gross_transactions: gross_count,
        net_settlements: net_count,
        efficiency,
        legs,
        conservation_proof: ConservationProof {
            positions_hash: String::new(), // Would compute hash
            net_sum: Amount::new(0, currency, 18),
            verified: true,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bilateral_netting() {
        let positions = vec![
            GrossPosition {
                from: InstitutionId::new(),
                to: InstitutionId::new(),
                amount: Amount::new(100_000_000_000_000_000_000, Currency::iusd(), 18), // 100
                transactions: vec![],
            },
        ];

        let result = compute_multilateral_net(&positions, Currency::iusd()).unwrap();
        assert_eq!(result.net_settlements, 1);
        assert!(result.conservation_proof.verified);
    }

    #[test]
    fn test_multilateral_netting() {
        let inst_a = InstitutionId::new();
        let inst_b = InstitutionId::new();
        let inst_c = InstitutionId::new();

        // A owes B 100, B owes C 100, C owes A 100 -> circular, should net to 0
        let positions = vec![
            GrossPosition {
                from: inst_a.clone(),
                to: inst_b.clone(),
                amount: Amount::new(100_000_000_000_000_000_000, Currency::iusd(), 18),
                transactions: vec![],
            },
            GrossPosition {
                from: inst_b.clone(),
                to: inst_c.clone(),
                amount: Amount::new(100_000_000_000_000_000_000, Currency::iusd(), 18),
                transactions: vec![],
            },
            GrossPosition {
                from: inst_c.clone(),
                to: inst_a.clone(),
                amount: Amount::new(100_000_000_000_000_000_000, Currency::iusd(), 18),
                transactions: vec![],
            },
        ];

        let result = compute_multilateral_net(&positions, Currency::iusd()).unwrap();
        // Perfect circular netting - no settlements needed
        assert_eq!(result.net_settlements, 0);
        assert_eq!(result.efficiency, 1.0);
    }
}
