use chaoschain_core::{Block, ChainState, ChainConfig, Error as CoreError, Transaction};
use ed25519_dalek::VerifyingKey as PublicKey;
use parking_lot::RwLock;
use tracing::info;
use std::sync::Arc;
use thiserror::Error;
use hex;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

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
    #[error("Internal error: {0}")]
    Internal(String),
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
    blocks: Arc<RwLock<Vec<Block>>>,
}

impl StateStoreImpl {
    pub fn new(config: ChainConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(ChainState {
                balances: Vec::new(),
                producers: Vec::new(),
            })),
            config,
            last_block_time: Arc::new(RwLock::new(0)),
            blocks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the latest N blocks
    pub fn get_latest_blocks(&self, n: usize) -> Vec<Block> {
        let blocks = self.blocks.read();
        blocks.iter().rev().take(n).cloned().collect()
    }

    /// Get block timestamp (for now, just use block height * 10 seconds)
    pub fn get_block_timestamp(&self, block: &Block) -> Option<u64> {
        Some(block.height * 10)
    }

    /// Add a whitelisted block producer
    pub fn add_block_producer(&self, producer: PublicKey) {
        let mut state = self.state.write();
        let producer_str = hex::encode(producer.as_bytes());
        if !state.producers.contains(&producer_str) {
            state.producers.push(producer_str);
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
        state.balances
            .iter()
            .find(|(pk, _)| pk == &account_str)
            .map(|(_, balance)| *balance)
            .unwrap_or(0)
    }

    /// Verify a transaction
    fn verify_transaction(&self, _tx: &Transaction) -> Result<(), StateError> {
        // In ChaosChain, we don't care about balances!
        // Transactions can do anything they want
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
}

impl Default for StateStoreImpl {
    fn default() -> Self {
        Self::new(ChainConfig::default())
    }
}

impl StateStore for StateStoreImpl {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StateError> {
        Ok(None)
    }
    
    fn apply_diff(&mut self, _diff: StateDiff) -> Result<(), StateError> {
        Ok(())
    }
    
    fn state_root(&self) -> [u8; 32] {
        [0u8; 32]
    }

    fn get_block_height(&self) -> u64 {
        self.blocks.read().len() as u64
    }

    fn apply_block(&self, block: &Block) -> Result<(), StateError> {
        // Apply block rewards if configured
        if let Some(reward) = self.config.block_reward {
            let mut state = self.state.write();
            
            // Clone producers and balances to avoid borrowing issues
            let producers = state.producers.clone();
            let mut new_balances = state.balances.clone();
            
            // Add rewards for producers
            for producer in producers {
                match new_balances.iter_mut().find(|(addr, _)| addr == &producer) {
                    Some((_, balance)) => *balance += reward,
                    None => new_balances.push((producer, reward)),
                }
            }
            
            // Update state with new balances
            state.balances = new_balances;
        }

        // Store block
        let mut blocks = self.blocks.write();
        
        // Store the block - in ChaosChain blocks can come in any order!
        blocks.push(block.clone());
        
        // Sort blocks by height to maintain order
        blocks.sort_by_key(|b| b.height);

        Ok(())
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
}

impl StateManager {
    /// Create a new state manager
    pub fn new(config: ChainConfig) -> Self {
        Self {
            state: RwLock::new(ChainState::default()),
            config,
            last_block_time: RwLock::new(0),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> ChainState {
        self.state.read().clone()
    }

    /// Apply a block to state
    pub fn apply_block(&self, block: &Block) -> Result<(), StateError> {
        let mut state = self.state.write();
        
        // Verify transactions
        for tx in &block.transactions {
            self.verify_transaction(tx, &state)?;
        }

        // Clone producers and balances to avoid borrowing conflicts
        let producers = state.producers.clone();
        let mut balances = state.balances.clone();

        // Apply block rewards
        if let Some(reward) = self.config.block_reward {
            for producer in producers {
                if let Some((_, balance)) = balances.iter_mut()
                    .find(|(addr, _)| addr == &producer) {
                    *balance += reward;
                } else {
                    balances.push((producer.clone(), reward));
                }
            }
        }

        // Update state with new balances
        state.balances = balances;
        
        Ok(())
    }

    /// Verify a transaction
    fn verify_transaction(&self, tx: &Transaction, state: &ChainState) -> Result<(), StateError> {
        // Convert sender to hex string for comparison
        let sender_hex = hex::encode(tx.sender);
        
        // Find sender balance
        if let Some((_, _)) = state.balances.iter()
            .find(|(addr, _)| addr == &sender_hex) {
            // In ChaosChain, we don't care about balances!
            // Transactions can do anything they want
            Ok(())
        } else {
            // Add new account with 0 balance
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_basic_state_flow() {
        let config = ChainConfig::default();
        let store = StateStoreImpl::new(config);
        let state = store.get_state();
        assert_eq!(state.balances.len(), 0);
    }
} 