use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::{Vote, Error};
use chaoschain_core::Block;
use tracing::{info, warn};

/// Tracks votes and manages consensus formation
pub struct ConsensusManager {
    /// Current block being voted on
    current_block: RwLock<Option<Block>>,
    /// Votes for the current block
    votes: RwLock<HashMap<String, Vote>>,
    /// Total stake in the system
    total_stake: u64,
    /// Required stake percentage for consensus (e.g. 0.67 for 2/3)
    finality_threshold: f64,
}

impl ConsensusManager {
    pub fn new(total_stake: u64, finality_threshold: f64) -> Self {
        Self {
            current_block: RwLock::new(None),
            votes: RwLock::new(HashMap::new()),
            total_stake,
            finality_threshold,
        }
    }

    /// Start voting round for a new block
    pub async fn start_voting_round(&self, block: Block) {
        info!("Starting voting round for block {}", block.height);
        let mut current = self.current_block.write().await;
        *current = Some(block);
        self.votes.write().await.clear();
    }

    /// Add a vote from a validator
    pub async fn add_vote(&self, vote: Vote, stake: u64) -> Result<bool, Error> {
        let current = self.current_block.read().await;
        
        // Ensure we're voting on the current block
        if let Some(block) = &*current {
            if vote.block_hash != block.hash() {
                warn!("Vote for wrong block hash: expected {:?}, got {:?}", block.hash(), vote.block_hash);
                return Err(Error::Internal("Vote for wrong block".to_string()));
            }
        } else {
            return Err(Error::Internal("No active voting round".to_string()));
        }

        // Add the vote
        let mut votes = self.votes.write().await;
        votes.insert(vote.agent_id.clone(), vote);

        // Check if we have consensus
        let result = self.check_consensus(&votes, stake).await;
        if let Ok(true) = result {
            info!("Consensus reached for block {}", current.as_ref().unwrap().height);
        }
        result
    }

    /// Check if we have reached consensus
    async fn check_consensus(&self, votes: &HashMap<String, Vote>, stake_per_validator: u64) -> Result<bool, Error> {
        let mut approve_stake = 0u64;
        let mut reject_stake = 0u64;

        // Sum up stake for approvals and rejections
        for vote in votes.values() {
            if vote.approve {
                approve_stake = approve_stake.saturating_add(stake_per_validator);
            } else {
                reject_stake = reject_stake.saturating_add(stake_per_validator);
            }
        }

        // Check if we have enough stake for consensus
        let threshold_stake = (self.total_stake as f64 * self.finality_threshold) as u64;
        
        if approve_stake >= threshold_stake {
            Ok(true)
        } else if reject_stake >= threshold_stake {
            Ok(false)
        } else {
            Err(Error::InsufficientStake)
        }
    }

    /// Get all current votes
    pub async fn get_votes(&self) -> HashMap<String, Vote> {
        self.votes.read().await.clone()
    }

    /// Get current block being voted on
    pub async fn get_current_block(&self) -> Option<Block> {
        self.current_block.read().await.clone()
    }
} 