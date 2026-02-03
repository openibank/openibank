//! Receipt commands - Verify and inspect receipts

use colored::*;
use openibank_core::{CommitmentReceipt, IssuerReceipt};
use std::fs;

/// Verify a receipt
pub async fn verify_receipt(receipt_input: &str) -> anyhow::Result<()> {
    println!("{}", "Verifying Receipt...".bright_white().bold());
    println!();

    let json = load_receipt_json(receipt_input)?;

    // Try as IssuerReceipt first
    if let Ok(issuer_receipt) = serde_json::from_str::<IssuerReceipt>(&json) {
        println!("  Type: {}", "IssuerReceipt".bright_cyan());
        println!("  ID: {}", issuer_receipt.receipt_id.bright_yellow());
        println!("  Operation: {}", format!("{:?}", issuer_receipt.operation).bright_cyan());
        println!("  Amount: {}", format!("{}", issuer_receipt.amount).bright_cyan());
        println!("  Target: {}", issuer_receipt.target.0.bright_cyan());
        println!();

        match issuer_receipt.verify() {
            Ok(()) => {
                println!("  {} {}", "✓".bright_green(), "Signature is VALID".bright_green().bold());
                println!();
                println!("  This receipt cryptographically proves:");
                println!("    • The issuer signed this transaction");
                println!("    • The amount and target have not been tampered with");
                println!("    • The reserve attestation hash was captured at signing time");
            }
            Err(e) => {
                println!("  {} {}", "✗".bright_red(), "Signature is INVALID".bright_red().bold());
                println!("  Error: {}", e.to_string().bright_red());
            }
        }
        return Ok(());
    }

    // Try as CommitmentReceipt
    if let Ok(commitment_receipt) = serde_json::from_str::<CommitmentReceipt>(&json) {
        println!("  Type: {}", "CommitmentReceipt".bright_cyan());
        println!("  ID: {}", commitment_receipt.commitment_id.0.bright_yellow());
        println!("  Actor: {}", commitment_receipt.actor.0.bright_cyan());
        println!("  Intent Hash: {}...", &commitment_receipt.intent_hash[..16].bright_yellow());
        println!();

        match commitment_receipt.verify() {
            Ok(()) => {
                println!("  {} {}", "✓".bright_green(), "Signature is VALID".bright_green().bold());
                println!();
                println!("  This receipt cryptographically proves:");
                println!("    • The commitment was authorized by the signer");
                println!("    • The intent hash matches what was signed");
                println!("    • The policy snapshot was captured at commitment time");
            }
            Err(e) => {
                println!("  {} {}", "✗".bright_red(), "Signature is INVALID".bright_red().bold());
                println!("  Error: {}", e.to_string().bright_red());
            }
        }
        return Ok(());
    }

    anyhow::bail!("Could not parse receipt as IssuerReceipt or CommitmentReceipt");
}

/// Inspect receipt details
pub async fn inspect_receipt(receipt_input: &str) -> anyhow::Result<()> {
    println!("{}", "Receipt Details".bright_white().bold());
    println!("{}", "─".repeat(60));

    let json = load_receipt_json(receipt_input)?;

    // Try as IssuerReceipt first
    if let Ok(issuer_receipt) = serde_json::from_str::<IssuerReceipt>(&json) {
        println!();
        println!("  {}", "Type: IssuerReceipt".bright_white());
        println!();
        println!("  {}", "Core Fields:".bright_white());
        println!("    Receipt ID:     {}", issuer_receipt.receipt_id.bright_yellow());
        println!("    Operation:      {}", format!("{:?}", issuer_receipt.operation).bright_cyan());
        println!("    Asset:          {}", issuer_receipt.asset.0.bright_cyan());
        println!("    Amount:         {}", format!("{}", issuer_receipt.amount).bright_cyan());
        println!("    Target:         {}", issuer_receipt.target.0.bright_cyan());
        println!("    Issued At:      {}", issuer_receipt.issued_at.to_string().bright_cyan());
        println!();
        println!("  {}", "Evidence Hashes:".bright_white());
        println!("    Reserve Hash:   {}", issuer_receipt.reserve_attestation_hash.bright_yellow());
        println!("    Policy Hash:    {}", issuer_receipt.policy_snapshot_hash.bright_yellow());
        println!();
        println!("  {}", "Cryptographic Proof:".bright_white());
        println!("    Signature:      {}...", &issuer_receipt.signature[..32].bright_yellow());
        println!("    Signer Key:     {}...", &issuer_receipt.signer_public_key[..32].bright_yellow());
        println!();

        // Verify
        let verification = issuer_receipt.verify();
        println!("  {}", "Verification:".bright_white());
        println!("    Status:         {}",
            if verification.is_ok() {
                "VALID ✓".bright_green()
            } else {
                "INVALID ✗".bright_red()
            }
        );

        return Ok(());
    }

    // Try as CommitmentReceipt
    if let Ok(commitment_receipt) = serde_json::from_str::<CommitmentReceipt>(&json) {
        println!();
        println!("  {}", "Type: CommitmentReceipt".bright_white());
        println!();
        println!("  {}", "Core Fields:".bright_white());
        println!("    Commitment ID:  {}", commitment_receipt.commitment_id.0.bright_yellow());
        println!("    Actor:          {}", commitment_receipt.actor.0.bright_cyan());
        println!("    Committed At:   {}", commitment_receipt.committed_at.to_string().bright_cyan());
        println!();
        println!("  {}", "Evidence Hashes:".bright_white());
        println!("    Intent Hash:    {}", commitment_receipt.intent_hash.bright_yellow());
        println!("    Policy Hash:    {}", commitment_receipt.policy_snapshot_hash.bright_yellow());
        println!("    Evidence Hash:  {}", commitment_receipt.evidence_hash.bright_yellow());
        println!();
        println!("  {}", "Consequence Reference:".bright_white());
        println!("    Type:           {}", commitment_receipt.consequence_ref.consequence_type.bright_cyan());
        println!("    Reference ID:   {}", commitment_receipt.consequence_ref.reference_id.bright_cyan());
        println!();
        println!("  {}", "Cryptographic Proof:".bright_white());
        println!("    Signature:      {}...", &commitment_receipt.signature[..32].bright_yellow());
        println!("    Signer Key:     {}...", &commitment_receipt.signer_public_key[..32].bright_yellow());
        println!();

        // Verify
        let verification = commitment_receipt.verify();
        println!("  {}", "Verification:".bright_white());
        println!("    Status:         {}",
            if verification.is_ok() {
                "VALID ✓".bright_green()
            } else {
                "INVALID ✗".bright_red()
            }
        );

        return Ok(());
    }

    anyhow::bail!("Could not parse receipt");
}

/// Diff two receipts
pub async fn diff_receipts(receipt1_input: &str, receipt2_input: &str) -> anyhow::Result<()> {
    println!("{}", "Comparing Receipts".bright_white().bold());
    println!("{}", "─".repeat(60));

    let json1 = load_receipt_json(receipt1_input)?;
    let json2 = load_receipt_json(receipt2_input)?;

    let receipt1: serde_json::Value = serde_json::from_str(&json1)?;
    let receipt2: serde_json::Value = serde_json::from_str(&json2)?;

    println!();
    println!("  Comparing field by field:");
    println!();

    compare_json_values(&receipt1, &receipt2, "");

    Ok(())
}

fn compare_json_values(v1: &serde_json::Value, v2: &serde_json::Value, path: &str) {
    match (v1, v2) {
        (serde_json::Value::Object(o1), serde_json::Value::Object(o2)) => {
            // Get all keys from both objects
            let mut all_keys: Vec<&String> = o1.keys().chain(o2.keys()).collect();
            all_keys.sort();
            all_keys.dedup();

            for key in all_keys {
                let new_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", path, key)
                };

                match (o1.get(key), o2.get(key)) {
                    (Some(val1), Some(val2)) => {
                        compare_json_values(val1, val2, &new_path);
                    }
                    (Some(val1), None) => {
                        println!("    {} {} (only in first)", "−".bright_red(), new_path.bright_red());
                    }
                    (None, Some(val2)) => {
                        println!("    {} {} (only in second)", "+".bright_green(), new_path.bright_green());
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        _ => {
            if v1 == v2 {
                println!("    {} {}: {}", "=".bright_black(), path.bright_black(), format!("{}", v1).bright_black());
            } else {
                println!("    {} {}", "≠".bright_yellow(), path.bright_yellow());
                println!("        Receipt 1: {}", format!("{}", v1).bright_red());
                println!("        Receipt 2: {}", format!("{}", v2).bright_green());
            }
        }
    }
}

fn load_receipt_json(input: &str) -> anyhow::Result<String> {
    // Check if it's a file path
    if std::path::Path::new(input).exists() {
        Ok(fs::read_to_string(input)?)
    } else if input.starts_with('{') {
        // Looks like inline JSON
        Ok(input.to_string())
    } else {
        anyhow::bail!("Input is neither a valid file path nor JSON: {}", input)
    }
}
