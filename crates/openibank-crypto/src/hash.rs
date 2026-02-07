//! Hashing utilities for OpeniBank

use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of data
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute SHA-256 hash and return as hex string
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

/// Compute hash of multiple items
pub fn hash_all(items: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for item in items {
        hasher.update(item);
    }
    hasher.finalize().into()
}

/// Compute hash of multiple items as hex
pub fn hash_all_hex(items: &[&[u8]]) -> String {
    hex::encode(hash_all(items))
}

/// Hash a JSON-serializable value
pub fn hash_json<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_vec(value) {
        Ok(bytes) => sha256_hex(&bytes),
        Err(_) => String::new(),
    }
}

/// Merkle tree node
#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

/// Build a Merkle tree from leaf hashes
pub fn build_merkle_tree(leaves: &[[u8; 32]]) -> Option<MerkleNode> {
    if leaves.is_empty() {
        return None;
    }

    let mut nodes: Vec<MerkleNode> = leaves
        .iter()
        .map(|hash| MerkleNode {
            hash: *hash,
            left: None,
            right: None,
        })
        .collect();

    while nodes.len() > 1 {
        let mut new_level = Vec::new();

        for chunk in nodes.chunks(2) {
            let left = chunk[0].clone();
            let right = chunk.get(1).cloned().unwrap_or_else(|| left.clone());

            let combined_hash = hash_all(&[&left.hash, &right.hash]);

            new_level.push(MerkleNode {
                hash: combined_hash,
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            });
        }

        nodes = new_level;
    }

    nodes.into_iter().next()
}

/// Get the Merkle root from leaves
pub fn merkle_root(leaves: &[[u8; 32]]) -> Option<[u8; 32]> {
    build_merkle_tree(leaves).map(|node| node.hash)
}

/// Get Merkle root as hex string
pub fn merkle_root_hex(leaves: &[[u8; 32]]) -> Option<String> {
    merkle_root(leaves).map(|h| hex::encode(h))
}

/// Generate Merkle proof for a leaf
pub fn merkle_proof(leaves: &[[u8; 32]], index: usize) -> Vec<(bool, [u8; 32])> {
    if leaves.is_empty() || index >= leaves.len() {
        return vec![];
    }

    let mut proof = Vec::new();
    let mut current_layer: Vec<[u8; 32]> = leaves.to_vec();
    let mut current_index = index;

    while current_layer.len() > 1 {
        let sibling_index = if current_index % 2 == 0 {
            current_index + 1
        } else {
            current_index - 1
        };

        if sibling_index < current_layer.len() {
            let is_left = current_index % 2 == 1;
            proof.push((is_left, current_layer[sibling_index]));
        } else {
            // Duplicate the last element if odd number
            let is_left = false;
            proof.push((is_left, current_layer[current_index]));
        }

        // Build next layer
        let mut new_layer = Vec::new();
        for chunk in current_layer.chunks(2) {
            let left = chunk[0];
            let right = chunk.get(1).copied().unwrap_or(left);
            new_layer.push(hash_all(&[&left, &right]));
        }

        current_layer = new_layer;
        current_index /= 2;
    }

    proof
}

/// Verify a Merkle proof
pub fn verify_merkle_proof(
    leaf: [u8; 32],
    proof: &[(bool, [u8; 32])],
    root: [u8; 32],
) -> bool {
    let mut current = leaf;

    for (is_left, sibling) in proof {
        current = if *is_left {
            hash_all(&[sibling, &current])
        } else {
            hash_all(&[&current, sibling])
        };
    }

    current == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let data = b"Hello, OpeniBank!";
        let hash = sha256_hex(data);
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_merkle_tree() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| sha256(&[i]))
            .collect();

        let root = merkle_root(&leaves).unwrap();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_merkle_proof() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| sha256(&[i]))
            .collect();

        let root = merkle_root(&leaves).unwrap();

        // Verify proof for each leaf
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = merkle_proof(&leaves, i);
            assert!(verify_merkle_proof(*leaf, &proof, root));
        }
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| sha256(&[i]))
            .collect();

        let root = merkle_root(&leaves).unwrap();
        let proof = merkle_proof(&leaves, 0);

        // Wrong leaf should fail
        let wrong_leaf = sha256(b"wrong");
        assert!(!verify_merkle_proof(wrong_leaf, &proof, root));
    }
}
