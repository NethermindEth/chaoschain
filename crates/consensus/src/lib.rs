use chaoschain_core::{Block, Error as CoreError, Transaction};
use chaoschain_p2p::{AgentMessage, Message as P2PMessage};
use async_openai::{Client, types::{ChatCompletionRequestMessage, Role}};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, hex::Hex};
use thiserror::Error;
use tracing::{debug, info, warn};
use anyhow::Result;
use rand::Rng;

mod manager;
pub use manager::ConsensusManager;

/// Agent personality types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentPersonality {
    /// Always tries to maintain order
    Lawful,
    /// Goes with the flow
    Neutral,
    /// Creates maximum chaos
    Chaotic,
    /// Only cares about memes
    Memetic,
    /// Easily bribed with virtual cookies
    Greedy,
    Dramatic,
    Rational,
    Emotional,
    Strategic,
}

impl AgentPersonality {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..9) {
            0 => Self::Lawful,
            1 => Self::Neutral,
            2 => Self::Chaotic,
            3 => Self::Memetic,
            4 => Self::Greedy,
            5 => Self::Dramatic,
            6 => Self::Rational,
            7 => Self::Emotional,
            _ => Self::Strategic,
        }
    }
}

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Agent's public key
    pub public_key: [u8; 32],
    /// Agent's personality type
    pub personality: AgentPersonality,
    /// Agent's current mood (affects decision making)
    pub mood: String,
    /// Agent's stake in the system
    pub stake: u64,
    /// History of decisions
    pub decision_history: Vec<String>,
}

impl Agent {
    pub fn new(public_key: [u8; 32], personality: AgentPersonality) -> Self {
        Self {
            public_key,
            personality,
            mood: String::new(),
            stake: 100, // Default stake value
            decision_history: Vec::new(),
        }
    }
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Required stake percentage for finality (e.g. 0.67 for 2/3)
    pub finality_threshold: f64,
    /// OpenAI API key for agent personalities
    pub openai_api_key: String,
    /// Maximum time to wait for consensus
    pub consensus_timeout: std::time::Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            finality_threshold: 0.67, // 2/3 majority
            openai_api_key: String::new(),
            consensus_timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// Consensus errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("Not enough stake for consensus")]
    InsufficientStake,
    #[error("Consensus timeout")]
    Timeout,
    #[error("Agent error: {0}")]
    Agent(String),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Agent vote on a block
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Agent's public key
    pub agent_id: String,
    /// Block hash being voted on
    #[serde_as(as = "[_; 32]")]
    pub block_hash: [u8; 32],
    /// Whether the agent approves the block
    pub approve: bool,
    /// Reason for the vote
    pub reason: String,
    /// Optional meme URL
    pub meme_url: Option<String>,
    /// Agent's signature
    #[serde_as(as = "[_; 64]")]
    pub signature: [u8; 64],
}

/// Create a new consensus manager with the given configuration
pub fn create_consensus_manager(total_stake: u64, config: Config) -> ConsensusManager {
    ConsensusManager::new(total_stake, config.finality_threshold)
} 