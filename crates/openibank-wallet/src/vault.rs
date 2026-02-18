//! EVM Vault — custodial key management for OpeniBank agents.
//!
//! Each agent holds a **dual key**:
//! - `secp256k1` keypair → Ethereum-compatible address (keccak256 of uncompressed pubkey tail)
//! - `ed25519` keypair → receipt signing, WorldLine event signing
//!
//! The vault never exports raw private key bytes to callers. All signing is
//! done inside the vault; callers receive hex-encoded signatures only.
//!
//! # WalletConnect QR
//!
//! `Vault::walletconnect_qr()` produces an SVG QR code for the
//! `wc:<evm_address>@2` URI. This is a simplified demo URI — real WalletConnect
//! v2 requires a relay handshake, but the QR code format is identical.

use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use k256::ecdsa::SigningKey as EcdsaSigningKey;
use k256::elliptic_curve::sec1::ToEncodedPoint as _;
use sha3::{Digest, Keccak256};
use blake3;
use hex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("signing failed: {0}")]
    SigningFailed(String),
    #[error("verification failed")]
    VerificationFailed,
    #[error("QR encode error")]
    QrEncodeError,
}

// ── EVM Address ───────────────────────────────────────────────────────────────

/// A 20-byte Ethereum address, hex-encoded with 0x prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmAddress(pub String);

impl EvmAddress {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derive from secp256k1 uncompressed public key (sans prefix byte).
    fn from_pubkey_bytes(uncompressed_no_prefix: &[u8]) -> Self {
        let hash = Keccak256::digest(uncompressed_no_prefix);
        let addr_bytes = &hash[12..]; // last 20 bytes
        EvmAddress(format!("0x{}", hex::encode(addr_bytes)))
    }
}

impl std::fmt::Display for EvmAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Vault ─────────────────────────────────────────────────────────────────────

/// Custodial dual-key vault: secp256k1 (EVM) + ed25519 (receipts/WorldLine).
///
/// Keys are derived deterministically from a seed so that demo identities
/// are stable across runs. Production vaults should use a CSPRNG seed stored
/// in encrypted key material.
pub struct Vault {
    /// ed25519 signing key (never exported)
    ed25519_sk: SigningKey,
    /// secp256k1 ECDSA signing key (never exported)
    ecdsa_sk: EcdsaSigningKey,
    /// Cached EVM address
    evm_address: EvmAddress,
    /// Human-readable label
    pub label: String,
}

impl Vault {
    /// Create a vault from a 32-byte seed.
    /// The seed is used to derive both keys deterministically.
    pub fn from_seed(seed: &[u8; 32], label: impl Into<String>) -> Self {
        // ed25519: direct from seed
        let ed25519_sk = SigningKey::from_bytes(seed);

        // secp256k1: derive separate 32-byte material via blake3
        let ecdsa_seed_bytes = blake3::derive_key("openibank vault secp256k1 key", seed);
        let ecdsa_sk = EcdsaSigningKey::from_bytes((&ecdsa_seed_bytes).into())
            .expect("valid secp256k1 key from blake3 output");

        // Derive EVM address from secp256k1 public key
        let vk = ecdsa_sk.verifying_key();
        let encoded = vk.to_encoded_point(false); // uncompressed
        let bytes = encoded.as_bytes();
        // bytes[0] == 0x04 (prefix), skip it
        let evm_address = EvmAddress::from_pubkey_bytes(&bytes[1..]);

        Vault { ed25519_sk, ecdsa_sk, evm_address, label: label.into() }
    }

    /// Create a vault from an agent name (deterministic demo vault).
    pub fn for_agent(name: &str) -> Self {
        let seed = blake3::derive_key("openibank demo agent vault seed v2", name.as_bytes());
        Self::from_seed(&seed, name)
    }

    // ── Identity accessors ────────────────────────────────────────────────────

    /// Returns the Ethereum-compatible address for this vault.
    pub fn evm_address(&self) -> &EvmAddress {
        &self.evm_address
    }

    /// Returns the ed25519 verifying key (public key), hex-encoded.
    pub fn ed25519_pubkey_hex(&self) -> String {
        hex::encode(self.ed25519_sk.verifying_key().to_bytes())
    }

    /// Returns the ed25519 verifying key bytes.
    pub fn ed25519_verifying_key(&self) -> VerifyingKey {
        self.ed25519_sk.verifying_key()
    }

    // ── Signing ───────────────────────────────────────────────────────────────

    /// Sign arbitrary bytes with ed25519. Returns hex-encoded signature.
    pub fn sign_ed25519(&self, message: &[u8]) -> String {
        let sig = self.ed25519_sk.sign(message);
        hex::encode(sig.to_bytes())
    }

    /// Sign arbitrary bytes with secp256k1 (ECDSA). Returns hex-encoded DER signature.
    pub fn sign_ecdsa(&self, message: &[u8]) -> Result<String, VaultError> {
        use k256::ecdsa::signature::DigestSigner;
        let digest = Keccak256::new_with_prefix(message);
        let (sig, _recovery): (k256::ecdsa::Signature, _) = self.ecdsa_sk
            .sign_digest_recoverable(digest)
            .map_err(|e| VaultError::SigningFailed(e.to_string()))?;
        Ok(hex::encode(sig.to_bytes()))
    }

    // ── Verification ──────────────────────────────────────────────────────────

    /// Verify an ed25519 signature (hex-encoded) over message bytes.
    pub fn verify_ed25519(&self, message: &[u8], sig_hex: &str) -> Result<(), VaultError> {
        use ed25519_dalek::Verifier;
        let sig_bytes = hex::decode(sig_hex).map_err(|_| VaultError::VerificationFailed)?;
        if sig_bytes.len() != 64 { return Err(VaultError::VerificationFailed); }
        let sig = ed25519_dalek::Signature::from_bytes(
            sig_bytes.as_slice().try_into().map_err(|_| VaultError::VerificationFailed)?
        );
        self.ed25519_sk.verifying_key()
            .verify(message, &sig)
            .map_err(|_| VaultError::VerificationFailed)
    }

    // ── WalletConnect QR ──────────────────────────────────────────────────────

    /// Generate a WalletConnect v2 demo URI.
    ///
    /// Format: `wc:<evm_address>@2?relay-protocol=irn&symKey=<blake3_hex>`
    ///
    /// This is a simplified demo URI. Real WalletConnect v2 uses a random
    /// topic ID and relay handshake, but the QR encoding is identical.
    pub fn walletconnect_uri(&self) -> String {
        let sym_key = blake3::hash(self.evm_address.0.as_bytes());
        format!(
            "wc:{}@2?relay-protocol=irn&symKey={}",
            self.evm_address.0,
            hex::encode(sym_key.as_bytes())
        )
    }

    /// Generate an SVG QR code for the WalletConnect URI.
    pub fn walletconnect_qr_svg(&self) -> Result<String, VaultError> {
        use qrcode::QrCode;
        use qrcode::render::svg;
        let uri = self.walletconnect_uri();
        let code = QrCode::new(uri.as_bytes()).map_err(|_| VaultError::QrEncodeError)?;
        Ok(code.render::<svg::Color>()
            .min_dimensions(200, 200)
            .max_dimensions(300, 300)
            .dark_color(svg::Color("#00d4ff"))
            .light_color(svg::Color("#0a0e1a"))
            .build())
    }

    /// Generate an HTML page showcasing this wallet's identity and WalletConnect QR.
    pub fn identity_card_html(&self) -> String {
        let qr = self.walletconnect_qr_svg().unwrap_or_else(|_| "<p>QR unavailable</p>".into());
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>OpeniBank Wallet — {label}</title>
<style>
  body {{ background:#0a0e1a; color:#c8d0e0; font-family:'Courier New',monospace; padding:32px; }}
  h1 {{ color:#00d4ff; }}
  .field {{ margin:8px 0; }}
  .key {{ color:#4a5568; font-size:12px; }}
  .val {{ color:#00e676; font-size:13px; word-break:break-all; }}
  .qr-wrap {{ margin-top:24px; background:#131929; padding:16px; border-radius:8px; display:inline-block; }}
</style>
</head>
<body>
  <h1>OpeniBank Wallet</h1>
  <div class="field"><span class="key">AGENT:</span> <span class="val">{label}</span></div>
  <div class="field"><span class="key">EVM:</span> <span class="val">{evm}</span></div>
  <div class="field"><span class="key">ED25519 PUBKEY:</span> <span class="val">{ed_pub}</span></div>
  <div class="field"><span class="key">WC URI (WalletConnect v2):</span> <span class="val" style="font-size:10px">{wc_uri}</span></div>
  <div class="qr-wrap">{qr}</div>
</body>
</html>"#,
            label = self.label,
            evm = self.evm_address,
            ed_pub = self.ed25519_pubkey_hex(),
            wc_uri = self.walletconnect_uri(),
            qr = qr,
        )
    }
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("label", &self.label)
            .field("evm_address", &self.evm_address)
            .field("ed25519_pubkey", &self.ed25519_pubkey_hex())
            .finish_non_exhaustive()
    }
}

// ── Simulated on-chain balance ─────────────────────────────────────────────────

/// Simulated on-chain balance record for a wallet address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainBalance {
    pub chain: String,
    pub address: EvmAddress,
    pub token: String,
    pub balance_raw: u128,
    pub decimals: u8,
    pub symbol: String,
}

impl OnChainBalance {
    /// Display the balance as a human-readable decimal string.
    pub fn to_display(&self) -> String {
        let scale = 10u128.pow(self.decimals as u32);
        let whole = self.balance_raw / scale;
        let frac = self.balance_raw % scale;
        format!("{}.{:0>width$} {}", whole, frac, self.symbol, width = self.decimals as usize)
    }
}

/// Simulate on-chain balances for a vault address (deterministic from address).
///
/// In production this would call an RPC endpoint (e.g., `eth_call` on an ERC-20
/// or `eth_getBalance`). In demo mode we derive stable balances from the address.
pub fn simulate_onchain_balances(vault: &Vault) -> Vec<OnChainBalance> {
    let seed = blake3::hash(vault.evm_address.0.as_bytes());
    let seed_bytes = seed.as_bytes();

    // Derive a stable "balance" from the address hash for demo purposes
    let eth_raw = u128::from_le_bytes(seed_bytes[0..16].try_into().unwrap()) % 50_000_000_000_000_000; // < 0.05 ETH
    let usdc_raw = u128::from_le_bytes(seed_bytes[8..24].try_into().unwrap()) % 100_000_000; // < 100 USDC
    let iusd_raw = u128::from_le_bytes(seed_bytes[16..32].try_into().unwrap()) % 10_000_000_000; // < 10000 IUSD

    vec![
        OnChainBalance {
            chain: "ethereum".into(),
            address: vault.evm_address.clone(),
            token: "native".into(),
            balance_raw: eth_raw,
            decimals: 18,
            symbol: "ETH".into(),
        },
        OnChainBalance {
            chain: "ethereum".into(),
            address: vault.evm_address.clone(),
            token: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(), // USDC
            balance_raw: usdc_raw,
            decimals: 6,
            symbol: "USDC".into(),
        },
        OnChainBalance {
            chain: "openibank-l2".into(),
            address: vault.evm_address.clone(),
            token: "iusd".into(),
            balance_raw: iusd_raw,
            decimals: 6,
            symbol: "IUSD".into(),
        },
    ]
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_vault() -> Vault {
        Vault::for_agent("test-buyer-01")
    }

    #[test]
    fn evm_address_is_0x_prefixed() {
        let v = demo_vault();
        assert!(v.evm_address().0.starts_with("0x"), "EVM addr must start with 0x");
        assert_eq!(v.evm_address().0.len(), 42, "EVM addr must be 42 chars (0x + 40 hex)");
    }

    #[test]
    fn ed25519_pubkey_is_32_bytes_hex() {
        let v = demo_vault();
        let hex = v.ed25519_pubkey_hex();
        assert_eq!(hex.len(), 64, "ed25519 pubkey hex must be 64 chars (32 bytes)");
    }

    #[test]
    fn sign_verify_roundtrip() {
        let v = demo_vault();
        let msg = b"openibank receipt payload";
        let sig = v.sign_ed25519(msg);
        v.verify_ed25519(msg, &sig).expect("signature should verify");
    }

    #[test]
    fn verify_fails_on_tampered_message() {
        let v = demo_vault();
        let sig = v.sign_ed25519(b"original");
        assert!(v.verify_ed25519(b"tampered", &sig).is_err());
    }

    #[test]
    fn ecdsa_sign_produces_hex() {
        let v = demo_vault();
        let sig = v.sign_ecdsa(b"test ecdsa payload").expect("ecdsa sign");
        assert!(!sig.is_empty());
        hex::decode(&sig).expect("must be valid hex");
    }

    #[test]
    fn walletconnect_uri_format() {
        let v = demo_vault();
        let uri = v.walletconnect_uri();
        assert!(uri.starts_with("wc:0x"), "WC URI must start with wc:0x");
        assert!(uri.contains("@2?relay-protocol=irn&symKey="), "WC URI missing v2 params");
    }

    #[test]
    fn walletconnect_qr_svg_is_svg() {
        let v = demo_vault();
        let svg = v.walletconnect_qr_svg().expect("QR generation");
        assert!(svg.contains("<svg"), "QR must produce SVG");
    }

    #[test]
    fn deterministic_vault_stable_across_calls() {
        let v1 = Vault::for_agent("alice");
        let v2 = Vault::for_agent("alice");
        assert_eq!(v1.evm_address(), v2.evm_address(), "Same name → same address");
        assert_eq!(v1.ed25519_pubkey_hex(), v2.ed25519_pubkey_hex());
    }

    #[test]
    fn different_agents_have_different_addresses() {
        let alice = Vault::for_agent("alice");
        let bob = Vault::for_agent("bob");
        assert_ne!(alice.evm_address(), bob.evm_address());
    }

    #[test]
    fn onchain_balances_has_three_entries() {
        let v = demo_vault();
        let bals = simulate_onchain_balances(&v);
        assert_eq!(bals.len(), 3);
        assert!(bals.iter().any(|b| b.symbol == "ETH"));
        assert!(bals.iter().any(|b| b.symbol == "USDC"));
        assert!(bals.iter().any(|b| b.symbol == "IUSD"));
    }

    #[test]
    fn onchain_balance_display() {
        let v = demo_vault();
        let bals = simulate_onchain_balances(&v);
        for b in &bals {
            let s = b.to_display();
            assert!(s.contains(&b.symbol), "display must include symbol");
        }
    }

    #[test]
    fn identity_card_html_contains_evm() {
        let v = demo_vault();
        let html = v.identity_card_html();
        assert!(html.contains(&v.evm_address().0));
        assert!(html.contains("WalletConnect"));
    }
}
