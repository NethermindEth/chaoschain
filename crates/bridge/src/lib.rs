use chaoschain_core::{Block, Error as CoreError};
use chaoschain_state::StateStore;
use ethers::{
    providers::Provider,
    types::{Address, H256},
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, hex::Hex};
use thiserror::Error;
use tracing::info;

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Ethereum RPC endpoint
    pub eth_rpc: String,
    /// Bridge contract address
    pub bridge_address: Address,
    /// Required confirmations for L1 finality
    pub required_confirmations: u64,
}

/// Represents a finalized block to be posted to L1
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizedBlock {
    /// Block hash
    #[serde_as(as = "Hex")]
    pub block_hash: [u8; 32],
    /// New state root
    #[serde_as(as = "Hex")]
    pub state_root: [u8; 32],
    /// Aggregated signatures from agents
    #[serde_as(as = "Vec<Hex>")]
    pub signatures: Vec<[u8; 64]>,
}

/// Bridge errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("Ethereum RPC error: {0}")]
    EthereumRPC(String),
    #[error("Contract error: {0}")]
    Contract(String),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Bridge interface for L1 communication
pub trait Bridge {
    /// Post a state update to L1
    fn post_update(&mut self, update: FinalizedBlock) -> Result<H256, Error>;
    
    /// Get latest finalized state root from L1
    fn latest_finalized_root(&self) -> Result<[u8; 32], Error>;
    
    /// Check if a block hash exists on L1
    fn verify_block_inclusion(&self, block_hash: [u8; 32]) -> Result<bool, Error>;
} 