use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Merkle tree node
#[derive(Debug, Clone)]
pub struct Node {
    /// Node hash
    pub hash: [u8; 32],
    /// Left child hash
    #[allow(dead_code)]
    pub left: Option<Box<Node>>,
    /// Right child hash
    #[allow(dead_code)]
    pub right: Option<Box<Node>>,
    pub value: Option<Vec<u8>>,
}

impl Node {
    /// Create a new leaf node
    pub fn new_leaf(key: &[u8], value: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(value);
        Self {
            hash: hasher.finalize().into(),
            left: None,
            right: None,
            value: Some(value.to_vec()),
        }
    }

    /// Create a new internal node
    pub fn new_internal(left: Node, right: Node) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&left.hash);
        hasher.update(&right.hash);
        Self {
            hash: hasher.finalize().into(),
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            value: None,
        }
    }
}

/// Merkle tree for state management
#[derive(Debug)]
pub struct MerkleTree {
    /// Leaf nodes store transaction/state data
    leaves: HashMap<Vec<u8>, Vec<u8>>,
    /// Cache of computed node hashes
    nodes: HashMap<Vec<u8>, [u8; 32]>,
}

impl MerkleTree {
    /// Create a new empty merkle tree
    pub fn new() -> Self {
        Self {
            leaves: HashMap::new(),
            nodes: HashMap::new(),
        }
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.leaves.insert(key.to_vec(), value.to_vec());
        self.nodes.clear(); // Clear cache as tree has changed
    }

    /// Delete a key
    pub fn delete(&mut self, key: &[u8]) {
        self.leaves.remove(key);
        self.rebuild();
    }

    /// Get current root hash
    pub fn root_hash(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        // Sort leaves for deterministic ordering
        let mut sorted_leaves: Vec<_> = self.leaves.iter().collect();
        sorted_leaves.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        // Build tree bottom-up
        let mut current_level: Vec<[u8; 32]> = sorted_leaves
            .iter()
            .map(|(k, v)| self.hash_leaf(k, v))
            .collect();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                match chunk {
                    [left, right] => next_level.push(self.hash_nodes(left, right)),
                    [single] => next_level.push(*single),
                    _ => unreachable!(),
                }
            }
            current_level = next_level;
        }

        current_level[0]
    }

    fn hash_leaf(&self, key: &[u8], value: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"leaf");
        hasher.update(key);
        hasher.update(value);
        hasher.finalize().into()
    }

    fn hash_nodes(&self, left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"node");
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }

    /// Rebuild the merkle tree
    fn rebuild(&mut self) {
        if self.leaves.is_empty() {
            self.nodes.clear();
            return;
        }

        // Sort leaves by key for deterministic ordering
        let mut sorted_leaves: Vec<_> = self.leaves.iter().collect();
        sorted_leaves.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        // Build tree bottom-up
        let mut current_level: Vec<[u8; 32]> = sorted_leaves
            .iter()
            .map(|(k, v)| self.hash_leaf(k, v))
            .collect();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                match chunk {
                    [left, right] => next_level.push(self.hash_nodes(left, right)),
                    [single] => next_level.push(*single),
                    _ => unreachable!(),
                }
            }
            current_level = next_level;
        }

        self.nodes.clear();
        self.nodes.insert(b"root".to_vec(), current_level[0]);
    }

    /// Generate merkle proof for a key
    pub fn generate_proof(&self, key: &[u8]) -> Option<Vec<[u8; 32]>> {
        if !self.leaves.contains_key(key) {
            return None;
        }

        let mut sorted_leaves: Vec<_> = self.leaves.iter().collect();
        sorted_leaves.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        let mut proof = Vec::new();
        let mut current_level: Vec<([u8; 32], usize)> = sorted_leaves
            .iter()
            .enumerate()
            .map(|(i, (k, v))| (self.hash_leaf(k, v), i))
            .collect();

        let target_hash = self.hash_leaf(key, &self.leaves[key]);
        let mut current_index = current_level
            .iter()
            .position(|(hash, _)| *hash == target_hash)
            .unwrap();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                match chunk {
                    [(left_hash, left_idx), (right_hash, right_idx)] => {
                        if current_index == *left_idx {
                            proof.push(*right_hash);
                        } else if current_index == *right_idx {
                            proof.push(*left_hash);
                        }
                        next_level.push((self.hash_nodes(left_hash, right_hash), left_idx / 2));
                    }
                    [(single_hash, single_idx)] => {
                        next_level.push((*single_hash, *single_idx));
                    }
                    _ => unreachable!(),
                }
            }
            current_level = next_level;
            current_index /= 2;
        }

        Some(proof)
    }

    /// Verify a merkle proof
    pub fn verify_proof(root_hash: [u8; 32], key: &[u8], value: &[u8], proof: &[[u8; 32]]) -> bool {
        let mut current_hash: [u8; 32] = {
            let mut hasher = Sha256::new();
            hasher.update(b"leaf");
            hasher.update(key);
            hasher.update(value);
            hasher.finalize().into()
        };

        for sibling in proof {
            let mut hasher = Sha256::new();
            hasher.update(b"node");

            // Try both orderings
            hasher.update(&current_hash);
            hasher.update(sibling);
            let hash1: [u8; 32] = hasher.finalize().into();

            let mut hasher = Sha256::new();
            hasher.update(b"node");
            hasher.update(sibling);
            hasher.update(&current_hash);
            let hash2: [u8; 32] = hasher.finalize().into();

            // Use whichever ordering matches the root
            if hash1 == root_hash {
                current_hash = hash1;
                break;
            } else if hash2 == root_hash {
                current_hash = hash2;
                break;
            }
        }

        current_hash == root_hash
    }

    /// Get all keys in the tree
    pub fn get_all_keys(&self) -> Vec<Vec<u8>> {
        self.leaves.keys().cloned().collect()
    }

    /// Clear all entries from the tree
    pub fn clear(&mut self) {
        self.leaves.clear();
        self.nodes.clear();
    }

    /// Remove a key from the tree
    #[allow(dead_code)]
    pub fn remove(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let value = self.leaves.remove(key);
        self.rebuild();
        value
    }

    /// Get value for a key
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.leaves.get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_basic() {
        let mut tree = MerkleTree::new();

        // Insert some key-value pairs
        tree.insert(b"key1", b"value1");
        tree.insert(b"key2", b"value2");
        tree.insert(b"key3", b"value3");

        // Root should exist
        let root_hash = tree.root_hash();
        assert!(root_hash != [0u8; 32]);
    }

    #[test]
    fn test_merkle_proof() {
        let mut tree = MerkleTree::new();

        // Insert key-value pairs
        let key = b"test_key";
        let value = b"test_value";
        tree.insert(key, value);
        tree.insert(b"key2", b"value2");

        // Generate and verify proof
        let root_hash = tree.root_hash();
        let proof = tree.generate_proof(key).unwrap();

        assert!(MerkleTree::verify_proof(root_hash, key, value, &proof));
    }

    #[test]
    fn test_merkle_tree_updates() {
        let mut tree = MerkleTree::new();

        // Initial insert
        let key = b"key1";
        tree.insert(key, b"value1");
        let root1 = tree.root_hash();

        // Update value
        tree.insert(key, b"value2");
        let root2 = tree.root_hash();

        // Roots should be different
        assert_ne!(root1, root2);

        // Delete key
        tree.delete(key);
        assert_eq!(tree.leaves.len(), 0);
    }
}
