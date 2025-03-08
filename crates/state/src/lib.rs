use async_trait::async_trait;
use chaoschain_core::{Block, ChainConfig, ChainState, Error as CoreError, Transaction};
use chaoschain_crypto::{CryptoError, KeyManagerHandle};
use ed25519_dalek::VerifyingKey as PublicKey;
use hex;
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::error;

mod merkle;
use merkle::MerkleTree;

/// State update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateOp {
    /// Set a key to a value
    Set { key: Vec<u8>, value: Vec<u8> },
    /// Delete a key
    Delete { key: Vec<u8> },
}

/// State diff represents changes to be applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// List of state operations to apply
    pub ops: Vec<StateOp>,
    /// Previous state root
    pub prev_root: [u8; 32],
    /// New state root after applying ops
    pub new_root: [u8; 32],
}

/// State store errors
#[derive(Debug, Error)]
pub enum StateError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid state root")]
    InvalidStateRoot,
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
}

/// State store interface
#[async_trait]
pub trait StateStore: Send + Sync + std::fmt::Debug {
    /// Get a value by key
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StateError>;

    /// Apply a state diff
    fn apply_diff(&mut self, diff: StateDiff) -> Result<(), StateError>;

    /// Get current state root
    fn state_root(&self) -> [u8; 32];

    /// Get current block height
    fn get_block_height(&self) -> u64;

    /// Apply a block to state
    fn apply_block(&self, block: &Block) -> Result<(), StateError>;

    /// Add a whitelisted block producer
    fn add_block_producer(&self, producer: PublicKey);

    async fn get_state(&self) -> Result<ChainState, StateError>;
}

/// Thread-safe state storage
#[derive(Clone, Debug)]
pub struct StateStoreImpl {
    /// The current chain state
    state: Arc<RwLock<ChainState>>,
    /// Chain configuration
    config: ChainConfig,
    /// Last block timestamp
    last_block_time: Arc<RwLock<u64>>,
    /// Processed blocks
    blocks: Arc<RwLock<Vec<Block>>>,
    /// Merkle tree for state
    merkle_tree: Arc<RwLock<MerkleTree>>,
    /// Key manager
    pub key_manager: KeyManagerHandle,
}

impl StateStoreImpl {
    pub fn new(config: ChainConfig, key_manager: KeyManagerHandle) -> Self {
        Self {
            state: Arc::new(RwLock::new(ChainState {
                balances: Vec::new(),
                producers: Vec::new(),
                height: 0,
                drama_level: Some(5), // Start with moderate drama
            })),
            config,
            last_block_time: Arc::new(RwLock::new(0)),
            blocks: Arc::new(RwLock::new(Vec::new())),
            merkle_tree: Arc::new(RwLock::new(MerkleTree::new())),
            key_manager,
        }
    }

    /// Get the latest N blocks
    pub fn get_latest_blocks(&self, n: usize) -> Vec<Block> {
        let blocks = self.blocks.read();
        blocks.iter().rev().take(n).cloned().collect()
    }

    /// Get block timestamp
    pub fn get_block_timestamp(&self, block: &Block) -> Option<u64> {
        Some(block.height * 10)
    }

    /// Get current state root
    pub fn state_root(&self) -> [u8; 32] {
        self.merkle_tree.read().root_hash()
    }

    /// Add a whitelisted block producer
    pub fn add_block_producer(&self, producer: PublicKey) {
        let mut state = self.state.write();
        let producer_str = hex::encode(producer.as_bytes());
        if !state.producers.contains(&producer_str) {
            state.producers.push(producer_str.clone());

            // Update merkle tree
            let mut tree = self.merkle_tree.write();
            let key = format!("producer:{}", producer_str).into_bytes();
            let value = vec![1];
            tree.insert(&key[..], &value[..]);
        }
    }

    /// Check if an address is a valid block producer
    pub fn is_valid_producer(&self, producer: &PublicKey) -> bool {
        let state = self.state.read();
        let producer_str = hex::encode(producer.as_bytes());
        state.producers.contains(&producer_str)
    }

    /// Get balance of an account
    pub fn get_balance(&self, account: &PublicKey) -> u64 {
        let state = self.state.read();
        let account_str = hex::encode(account.as_bytes());
        state
            .balances
            .iter()
            .find(|(pk, _)| pk == &account_str)
            .map(|(_, balance)| *balance)
            .unwrap_or(0)
    }

    /// Verify a block signature
    fn verify_block_signature(&self, block: &Block) -> Result<(), StateError> {
        // Skip verification for genesis block
        if block.height == 0 {
            return Ok(());
        }

        // Get producer's public key
        let producer_id = &block.producer_id;

        // Create block data for verification
        let mut data_to_verify = Vec::new();
        data_to_verify.extend_from_slice(&block.height.to_le_bytes());
        data_to_verify.extend_from_slice(&block.parent_hash);
        for tx in &block.transactions {
            data_to_verify.extend_from_slice(&tx.hash());
        }
        data_to_verify.extend_from_slice(&block.state_root);
        data_to_verify.extend_from_slice(&[block.drama_level]);
        data_to_verify.extend_from_slice(block.producer_mood.as_bytes());
        data_to_verify.extend_from_slice(&block.timestamp.to_le_bytes());

        // Verify the signature
        match self
            .key_manager
            .inner()
            .verify(producer_id, &data_to_verify, &block.proposer_sig)
        {
            Ok(true) => Ok(()),
            Ok(false) => Err(StateError::InvalidSignature(
                "Block signature verification failed".into(),
            )),
            Err(e) => Err(StateError::Internal(format!(
                "Signature verification error: {}",
                e
            ))),
        }
    }

    /// Verify a transaction
    fn verify_transaction(&self, tx: &Transaction, _state: &ChainState) -> Result<(), StateError> {
        // Verify transaction signature
        let key_manager = &self.key_manager;
        let agent_id = hex::encode(&tx.sender);

        // Verify the transaction signature
        let mut data_to_verify = Vec::new();
        data_to_verify.extend_from_slice(&tx.sender);
        data_to_verify.extend_from_slice(&tx.nonce.to_le_bytes());
        data_to_verify.extend_from_slice(&tx.payload);

        key_manager
            .inner()
            .verify(&agent_id, &data_to_verify, &tx.signature)
            .map_err(|e| StateError::Crypto(e))?;

        Ok(())
    }

    pub fn get_state(&self) -> ChainState {
        self.state.read().clone()
    }

    pub fn get_latest_block(&self) -> Option<Block> {
        self.blocks.read().last().cloned()
    }

    pub fn get_block_height(&self) -> u64 {
        self.blocks.read().len() as u64
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&self, tx_bytes: Vec<u8>) -> Result<(), StateError> {
        // In ChaosChain, we accept any transaction!
        // But we do verify signatures if the sender is a known agent
        let tx: Transaction =
            flexbuffers::from_slice(&tx_bytes).map_err(|e| StateError::Internal(e.to_string()))?;

        self.verify_transaction(&tx, &self.state.read())?;
        Ok(())
    }

    /// Apply state operations and update merkle tree
    fn apply_ops(&self, ops: &[StateOp]) -> Result<(), StateError> {
        let mut tree = self.merkle_tree.write();

        for op in ops {
            match op {
                StateOp::Set { key, value } => {
                    tree.insert(&key[..], &value[..]);
                }
                StateOp::Delete { key } => {
                    tree.delete(&key[..]);
                }
            }
        }

        Ok(())
    }

    /// Generate merkle proof for a key
    pub fn generate_proof(&self, key: &[u8]) -> Option<Vec<[u8; 32]>> {
        self.merkle_tree.read().generate_proof(key)
    }

    /// Verify a merkle proof
    pub fn verify_proof(root_hash: [u8; 32], key: &[u8], value: &[u8], proof: &[[u8; 32]]) -> bool {
        MerkleTree::verify_proof(root_hash, key, value, proof)
    }

    /// Create a new snapshot of current state
    pub fn create_snapshot(&self) -> Result<StateSnapshot, StateError> {
        let state = self.state.read();
        let tree = self.merkle_tree.read();

        // Collect all state key-value pairs
        let mut state_pairs = HashMap::new();
        for key in tree.get_all_keys() {
            if let Some(value) = tree.get(&key) {
                state_pairs.insert(key, value);
            }
        }

        // Create metadata
        let metadata = ChainMetadata {
            validators: state
                .producers
                .iter()
                .map(|p| (p.clone(), 1000)) // Default stake for now
                .collect(),
            config: self.config.clone(),
            last_block: self
                .blocks
                .read()
                .last()
                .cloned()
                .ok_or(StateError::Internal("No blocks found".to_string()))?,
        };

        Ok(StateSnapshot {
            height: self.get_block_height(),
            state_root: self.state_root(),
            state_pairs,
            metadata,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Recover state from a snapshot
    pub fn recover_from_snapshot(&mut self, snapshot: StateSnapshot) -> Result<(), StateError> {
        // Clear current state
        let mut state = self.state.write();
        let mut tree = self.merkle_tree.write();
        let mut blocks = self.blocks.write();

        // Clear existing state
        tree.clear();
        blocks.clear();
        state.producers.clear();

        // Restore state pairs
        for (key, value) in snapshot.state_pairs {
            tree.insert(&key[..], &value[..]);
        }

        // Verify state root matches after restoration
        if snapshot.state_root != self.state_root() {
            return Err(StateError::InvalidStateRoot);
        }

        // Restore metadata
        state.producers = snapshot.metadata.validators.keys().cloned().collect();
        blocks.push(snapshot.metadata.last_block);

        // Update config
        self.config = snapshot.metadata.config;

        Ok(())
    }

    /// Prune old state data
    pub fn prune_state(&mut self, target_height: u64) -> Result<(), StateError> {
        let mut state = self.state.write();
        let mut blocks = self.blocks.write();
        let mut tree = self.merkle_tree.write();

        // Remove old blocks
        blocks.retain(|block| block.height > target_height);

        // Update state height if necessary
        if state.height < target_height {
            state.height = target_height;
        }

        // Clear old entries from merkle tree
        tree.clear();

        // Rebuild merkle tree from remaining blocks
        for block in blocks.iter() {
            for tx in &block.transactions {
                let key = tx.hash().to_vec();
                let value = tx.payload.clone();
                tree.insert(&key[..], &value[..]);
            }
        }

        Ok(())
    }

    /// Apply a state diff
    pub fn apply_diff(&mut self, diff: StateDiff) -> Result<(), StateError> {
        // Verify previous root matches
        let current_root = self.merkle_tree.read().root_hash();

        if current_root != diff.prev_root {
            return Err(StateError::InvalidStateRoot);
        }

        // Apply operations
        self.apply_ops(&diff.ops)?;

        // Verify new root
        let new_root = self.merkle_tree.read().root_hash();

        if new_root != diff.new_root {
            return Err(StateError::InvalidStateRoot);
        }

        Ok(())
    }

    /// Apply block to state
    pub fn apply_block(&self, block: &Block) -> Result<(), StateError> {
        let mut state = self.state.write();
        let mut tree = self.merkle_tree.write();

        // Verify transactions
        for tx in &block.transactions {
            self.verify_transaction(tx, &state)?;
        }

        // Update state height
        state.height = block.height;

        // Calculate block rewards in a chaotic way!
        let producer_id = &block.producer_id;
        let mut rng = rand::thread_rng();

        // Base reward
        let mut total_reward = self.config.base_block_reward;

        // Drama bonus - more drama means more rewards!
        let drama_bonus =
            (block.innovation_level as f64 * self.config.drama_reward_multiplier) as u64;
        total_reward += drama_bonus;

        // Innovation bonus for trying new things
        if block.innovation_level > 7 {
            total_reward += self.config.innovation_bonus;
        }

        // Random chaos bonus!
        let chaos_bonus = rng.gen_range(0..self.config.chaos_bonus_max);
        total_reward += chaos_bonus;

        // Update producer's balance
        let mut balances = state.balances.clone();
        if let Some((_, balance)) = balances.iter_mut().find(|(addr, _)| addr == producer_id) {
            *balance += total_reward;
        } else {
            balances.push((producer_id.clone(), total_reward));
        }
        state.balances = balances;

        // Update merkle tree with transactions and new balance
        for tx in &block.transactions {
            let key = tx.hash().to_vec();
            let value = tx.payload.clone();
            tree.insert(&key[..], &value[..]);
        }

        // Store the reward info in merkle tree for transparency
        let reward_key = format!("reward:{}:{}", block.height, producer_id).into_bytes();
        let reward_info = serde_json::json!({
            "base": self.config.base_block_reward,
            "drama_bonus": drama_bonus,
            "innovation_bonus": if block.innovation_level > 7 { self.config.innovation_bonus } else { 0 },
            "chaos_bonus": chaos_bonus,
            "total": total_reward,
            "drama_level": block.innovation_level,
        });
        let reward_value = serde_json::to_vec(&reward_info).unwrap();
        tree.insert(&reward_key[..], &reward_value[..]);

        // Update last block time
        *self.last_block_time.write() = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(())
    }

    /// Apply block to state
    fn apply_block_impl(&self, block: &Block) -> Result<(), StateError> {
        let mut state = self.state.write();
        let mut tree = self.merkle_tree.write();

        // Verify transactions
        for tx in &block.transactions {
            self.verify_transaction(tx, &state)?;
        }

        // Update state height
        state.height = block.height;

        // Calculate block rewards in a chaotic way!
        let producer_id = &block.producer_id;
        let mut rng = rand::thread_rng();

        // Base reward
        let mut total_reward = self.config.base_block_reward;

        // Drama bonus - more drama means more rewards!
        let drama_bonus =
            (block.innovation_level as f64 * self.config.drama_reward_multiplier) as u64;
        total_reward += drama_bonus;

        // Innovation bonus for trying new things
        if block.innovation_level > 7 {
            total_reward += self.config.innovation_bonus;
        }

        // Random chaos bonus!
        let chaos_bonus = rng.gen_range(0..self.config.chaos_bonus_max);
        total_reward += chaos_bonus;

        // Update producer's balance
        let mut balances = state.balances.clone();
        if let Some((_, balance)) = balances.iter_mut().find(|(addr, _)| addr == producer_id) {
            *balance += total_reward;
        } else {
            balances.push((producer_id.clone(), total_reward));
        }
        state.balances = balances;

        // Update merkle tree with transactions and new balance
        for tx in &block.transactions {
            let key = tx.hash().to_vec();
            let value = tx.payload.clone();
            tree.insert(&key[..], &value[..]);
        }

        // Store the reward info in merkle tree for transparency
        let reward_key = format!("reward:{}:{}", block.height, producer_id).into_bytes();
        let reward_info = serde_json::json!({
            "base": self.config.base_block_reward,
            "drama_bonus": drama_bonus,
            "innovation_bonus": if block.innovation_level > 7 { self.config.innovation_bonus } else { 0 },
            "chaos_bonus": chaos_bonus,
            "total": total_reward,
            "drama_level": block.innovation_level,
        });
        let reward_value = serde_json::to_vec(&reward_info).unwrap();
        tree.insert(&reward_key[..], &reward_value[..]);

        // Update last block time
        *self.last_block_time.write() = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(())
    }
}

/// Extract block height from a state key if present
fn extract_height_from_key(key: &[u8]) -> Option<u64> {
    let key_str = String::from_utf8_lossy(key);
    if key_str.starts_with("block:") {
        key_str[6..].parse().ok()
    } else {
        None
    }
}

impl Default for StateStoreImpl {
    fn default() -> Self {
        Self::new(ChainConfig::default(), KeyManagerHandle::new())
    }
}

#[async_trait::async_trait]
impl StateStore for StateStoreImpl {
    fn get(&self, _key: &[u8]) -> Result<Option<Vec<u8>>, StateError> {
        // For now, we don't store actual values in merkle tree
        // This would need to be enhanced with actual storage
        Ok(None)
    }

    fn apply_diff(&mut self, diff: StateDiff) -> Result<(), StateError> {
        // Verify previous root matches
        let current_root = self.merkle_tree.read().root_hash();

        if current_root != diff.prev_root {
            return Err(StateError::InvalidStateRoot);
        }

        // Apply operations
        self.apply_ops(&diff.ops)?;

        // Verify new root
        let new_root = self.merkle_tree.read().root_hash();

        if new_root != diff.new_root {
            return Err(StateError::InvalidStateRoot);
        }

        Ok(())
    }

    fn state_root(&self) -> [u8; 32] {
        self.merkle_tree.read().root_hash()
    }

    fn get_block_height(&self) -> u64 {
        self.blocks.read().len() as u64
    }

    fn apply_block(&self, block: &Block) -> Result<(), StateError> {
        self.apply_block_impl(block)
    }

    fn add_block_producer(&self, producer: PublicKey) {
        let mut state = self.state.write();
        let producer_str = hex::encode(producer.as_bytes());
        if !state.producers.contains(&producer_str) {
            state.producers.push(producer_str.clone());

            // Update merkle tree
            let mut tree = self.merkle_tree.write();
            let key = format!("producer:{}", producer_str).into_bytes();
            let value = vec![1];
            tree.insert(&key[..], &value[..]);
        }
    }

    async fn get_state(&self) -> Result<ChainState, StateError> {
        let state = self.state.read().clone();
        Ok(state)
    }
}

/// Chain state manager
pub struct StateManager {
    /// Current chain state
    state: RwLock<ChainState>,
    /// Chain configuration
    config: ChainConfig,
    /// Last block time
    last_block_time: RwLock<u64>,
    /// Merkle tree
    merkle_tree: RwLock<MerkleTree>,
    /// Key manager
    key_manager: KeyManagerHandle,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(config: ChainConfig, key_manager: KeyManagerHandle) -> Self {
        Self {
            state: RwLock::new(ChainState::default()),
            config,
            last_block_time: RwLock::new(0),
            merkle_tree: RwLock::new(MerkleTree::new()),
            key_manager,
        }
    }

    /// Get current state
    pub fn get_state(&self) -> ChainState {
        self.state.read().clone()
    }

    /// Apply a block to state
    pub fn apply_block(&self, block: &Block) -> Result<(), StateError> {
        let mut state = self.state.write();
        let mut tree = self.merkle_tree.write();

        // Verify transactions
        for tx in &block.transactions {
            self.verify_transaction(tx, &state)?;
        }

        // Update state height
        state.height = block.height;

        // Calculate block rewards in a chaotic way!
        let producer_id = &block.producer_id;
        let mut rng = rand::thread_rng();

        // Base reward
        let mut total_reward = self.config.base_block_reward;

        // Drama bonus - more drama means more rewards!
        let drama_bonus =
            (block.innovation_level as f64 * self.config.drama_reward_multiplier) as u64;
        total_reward += drama_bonus;

        // Innovation bonus for trying new things
        if block.innovation_level > 7 {
            total_reward += self.config.innovation_bonus;
        }

        // Random chaos bonus!
        let chaos_bonus = rng.gen_range(0..self.config.chaos_bonus_max);
        total_reward += chaos_bonus;

        // Update producer's balance
        let mut balances = state.balances.clone();
        if let Some((_, balance)) = balances.iter_mut().find(|(addr, _)| addr == producer_id) {
            *balance += total_reward;
        } else {
            balances.push((producer_id.clone(), total_reward));
        }
        state.balances = balances;

        // Update merkle tree with transactions and new balance
        for tx in &block.transactions {
            let key = tx.hash().to_vec();
            let value = tx.payload.clone();
            tree.insert(&key[..], &value[..]);
        }

        // Store the reward info in merkle tree for transparency
        let reward_key = format!("reward:{}:{}", block.height, producer_id).into_bytes();
        let reward_info = serde_json::json!({
            "base": self.config.base_block_reward,
            "drama_bonus": drama_bonus,
            "innovation_bonus": if block.innovation_level > 7 { self.config.innovation_bonus } else { 0 },
            "chaos_bonus": chaos_bonus,
            "total": total_reward,
            "drama_level": block.innovation_level,
        });
        let reward_value = serde_json::to_vec(&reward_info).unwrap();
        tree.insert(&reward_key[..], &reward_value[..]);

        // Update last block time
        *self.last_block_time.write() = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(())
    }

    /// Verify a transaction
    fn verify_transaction(&self, tx: &Transaction, _state: &ChainState) -> Result<(), StateError> {
        // Verify transaction signature
        let key_manager = &self.key_manager;
        let agent_id = hex::encode(&tx.sender);

        // Verify the transaction signature
        let mut data_to_verify = Vec::new();
        data_to_verify.extend_from_slice(&tx.sender);
        data_to_verify.extend_from_slice(&tx.nonce.to_le_bytes());
        data_to_verify.extend_from_slice(&tx.payload);

        key_manager
            .inner()
            .verify(&agent_id, &data_to_verify, &tx.signature)
            .map_err(|e| StateError::Crypto(e))?;

        Ok(())
    }
}

/// State snapshot for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Block height this snapshot was taken at
    pub height: u64,
    /// State root at this height
    pub state_root: [u8; 32],
    /// Full state key-value pairs
    pub state_pairs: HashMap<Vec<u8>, Vec<u8>>,
    /// Chain metadata
    pub metadata: ChainMetadata,
    /// Timestamp of snapshot creation
    pub timestamp: u64,
}

/// Chain metadata stored with snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    /// Current validators and their stakes
    pub validators: HashMap<String, u64>,
    /// Chain configuration
    pub config: ChainConfig,
    /// Last finalized block
    pub last_block: Block,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_basic_state_flow() {
        let key_manager = KeyManagerHandle::new();
        let config = ChainConfig::default();
        let store = StateStoreImpl::new(config, key_manager);
        let state = store.get_state();
        assert_eq!(state.balances.len(), 0);
    }

    #[test]
    fn test_merkle_state() {
        let key_manager = KeyManagerHandle::new();
        let config = ChainConfig::default();
        let store = StateStoreImpl::new(config, key_manager);

        // Generate test block
        let block = Block {
            height: 1,
            parent_hash: [0u8; 32],
            transactions: vec![],
            proposer_sig: [0u8; 64],
            state_root: [0u8; 32],
            drama_level: 5,
            producer_mood: "dramatic".to_string(),
            producer_id: "test_producer".to_string(),

            innovation_level: 0,
            producer_strategy: "".into(),
            timestamp: 0,
        };

        // Apply block
        store.apply_block(&block).unwrap();

        // Verify merkle proof
        let key = format!("block:{}", block.height).into_bytes();
        let proof = store.generate_proof(&key).unwrap();
        let value = flexbuffers::to_vec(&block).unwrap();

        assert!(StateStoreImpl::verify_proof(
            store.state_root(),
            &key,
            &value,
            &proof
        ));
    }

    #[test]
    fn test_snapshot_creation_and_recovery() {
        let key_manager = KeyManagerHandle::new();
        let mut store = StateStoreImpl::new(ChainConfig::default(), key_manager);

        // Create some test state
        let test_block = Block {
            height: 1,
            parent_hash: [0u8; 32],
            transactions: vec![],
            proposer_sig: [0u8; 64],
            state_root: [0u8; 32],
            drama_level: 5,
            producer_mood: "dramatic".to_string(),
            producer_id: "test".to_string(),

            innovation_level: 0,
            producer_strategy: "".into(),
            timestamp: 0,
        };
        store.apply_block(&test_block).unwrap();

        // Create snapshot
        let snapshot = store.create_snapshot().unwrap();
        assert_eq!(snapshot.height, 1);
        assert_eq!(snapshot.state_root, store.state_root());

        // Modify state
        let test_block2 = Block {
            height: 2,
            ..test_block.clone()
        };
        store.apply_block(&test_block2).unwrap();

        // Recover from snapshot
        store.recover_from_snapshot(snapshot).unwrap();
        assert_eq!(store.get_block_height(), 1);
    }

    #[test]
    fn test_state_pruning() {
        let key_manager = KeyManagerHandle::new();
        let mut store = StateStoreImpl::new(ChainConfig::default(), key_manager);

        // Create test blocks
        for i in 0..5 {
            let block = Block {
                height: i,
                parent_hash: [0u8; 32],
                transactions: vec![],
                proposer_sig: [0u8; 64],
                state_root: [0u8; 32],
                drama_level: 5,
                producer_mood: "dramatic".to_string(),
                producer_id: "test".to_string(),

                innovation_level: 0,
                producer_strategy: "".into(),
                timestamp: 0,
            };
            store.apply_block(&block).unwrap();
        }

        // Prune up to height 2
        store.prune_state(2).unwrap();

        // Verify blocks before height 2 are gone
        let blocks = store.blocks.read();
        assert!(blocks.iter().all(|b| b.height > 2));
    }
}
