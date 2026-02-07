//! Secure key vault for OpeniBank
//!
//! **Security Invariant: Private keys NEVER leave the vault.**

use crate::{CryptoError, CryptoResult, KeyId, KeyPair, PublicKey, Signature};
use std::collections::HashMap;
use std::sync::RwLock;

/// A secure key vault that stores private keys
///
/// In production, this would use HSM or secure enclave.
/// For now, we use in-memory storage with the critical
/// invariant that keys never leave the vault.
pub struct KeyVault {
    /// Keys indexed by ID
    keys: RwLock<HashMap<KeyId, VaultEntry>>,
}

/// An entry in the vault
struct VaultEntry {
    keypair: KeyPair,
    metadata: KeyMetadata,
}

/// Metadata about a key
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    /// When the key was created
    pub created_at: i64,
    /// Purpose of the key
    pub purpose: KeyPurpose,
    /// Whether the key is active
    pub active: bool,
}

/// Purpose of a key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPurpose {
    /// Signing receipts
    ReceiptSigning,
    /// Signing commitments
    CommitmentSigning,
    /// Signing permits
    PermitSigning,
    /// General purpose
    General,
}

impl KeyVault {
    /// Create a new empty vault
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a new key pair and store it
    pub fn generate_key(&self, purpose: KeyPurpose) -> CryptoResult<(KeyId, PublicKey)> {
        let keypair = KeyPair::generate()?;
        let public_key = PublicKey::from_keypair(&keypair);
        let key_id = KeyId::new();

        let entry = VaultEntry {
            keypair,
            metadata: KeyMetadata {
                created_at: chrono::Utc::now().timestamp_millis(),
                purpose,
                active: true,
            },
        };

        self.keys
            .write()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?
            .insert(key_id.clone(), entry);

        Ok((key_id, public_key))
    }

    /// Import an existing key (from secure backup)
    pub fn import_key(
        &self,
        key_bytes: &[u8; 32],
        purpose: KeyPurpose,
    ) -> CryptoResult<(KeyId, PublicKey)> {
        let keypair = KeyPair::from_bytes(key_bytes)?;
        let public_key = PublicKey::from_keypair(&keypair);
        let key_id = KeyId::new();

        let entry = VaultEntry {
            keypair,
            metadata: KeyMetadata {
                created_at: chrono::Utc::now().timestamp_millis(),
                purpose,
                active: true,
            },
        };

        self.keys
            .write()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?
            .insert(key_id.clone(), entry);

        Ok((key_id, public_key))
    }

    /// Get the public key for a key ID
    pub fn get_public_key(&self, key_id: &KeyId) -> CryptoResult<PublicKey> {
        let keys = self
            .keys
            .read()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?;

        let entry = keys
            .get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;

        Ok(PublicKey::from_keypair(&entry.keypair))
    }

    /// Sign data using a key (key never leaves vault!)
    pub fn sign(&self, key_id: &KeyId, message: &[u8]) -> CryptoResult<Signature> {
        let keys = self
            .keys
            .read()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?;

        let entry = keys
            .get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;

        if !entry.metadata.active {
            return Err(CryptoError::SigningFailed("Key is inactive".to_string()));
        }

        Signature::sign(&entry.keypair, message)
    }

    /// Get metadata for a key
    pub fn get_metadata(&self, key_id: &KeyId) -> CryptoResult<KeyMetadata> {
        let keys = self
            .keys
            .read()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?;

        let entry = keys
            .get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;

        Ok(entry.metadata.clone())
    }

    /// Deactivate a key (soft delete)
    pub fn deactivate_key(&self, key_id: &KeyId) -> CryptoResult<()> {
        let mut keys = self
            .keys
            .write()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?;

        let entry = keys
            .get_mut(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;

        entry.metadata.active = false;
        Ok(())
    }

    /// List all active key IDs
    pub fn list_keys(&self) -> CryptoResult<Vec<(KeyId, KeyMetadata)>> {
        let keys = self
            .keys
            .read()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?;

        Ok(keys
            .iter()
            .filter(|(_, e)| e.metadata.active)
            .map(|(id, e)| (id.clone(), e.metadata.clone()))
            .collect())
    }

    /// Export key bytes for secure backup (DANGER - use carefully!)
    ///
    /// This is the ONLY way keys leave the vault, and should only
    /// be used for secure backup to HSM or encrypted storage.
    #[cfg(feature = "export")]
    pub fn export_for_backup(&self, key_id: &KeyId) -> CryptoResult<[u8; 32]> {
        let keys = self
            .keys
            .read()
            .map_err(|e| CryptoError::VaultError(e.to_string()))?;

        let entry = keys
            .get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;

        Ok(entry.keypair.signing_key_bytes())
    }
}

impl Default for KeyVault {
    fn default() -> Self {
        Self::new()
    }
}

/// Global vault instance for singleton access
static VAULT: std::sync::OnceLock<KeyVault> = std::sync::OnceLock::new();

/// Get the global vault instance
pub fn vault() -> &'static KeyVault {
    VAULT.get_or_init(KeyVault::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_generate_and_sign() {
        let vault = KeyVault::new();

        let (key_id, public_key) = vault.generate_key(KeyPurpose::General).unwrap();
        let message = b"Hello, OpeniBank!";

        let signature = vault.sign(&key_id, message).unwrap();
        assert!(signature.verify(message).unwrap());
        assert_eq!(signature.public_key, public_key);
    }

    #[test]
    fn test_vault_deactivate_key() {
        let vault = KeyVault::new();

        let (key_id, _) = vault.generate_key(KeyPurpose::General).unwrap();
        vault.deactivate_key(&key_id).unwrap();

        let result = vault.sign(&key_id, b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_list_keys() {
        let vault = KeyVault::new();

        vault.generate_key(KeyPurpose::ReceiptSigning).unwrap();
        vault.generate_key(KeyPurpose::CommitmentSigning).unwrap();

        let keys = vault.list_keys().unwrap();
        assert_eq!(keys.len(), 2);
    }
}
