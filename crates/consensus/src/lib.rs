use async_trait::async_trait;
use chaoschain_core::{Block, Error as CoreError, NetworkEvent, ValidationDecision};
use chaoschain_state::StateStore;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;

pub mod manager;
pub mod types;
pub mod validator;

pub use manager::ConsensusManager;
pub use types::*;
pub use validator::Validator;

/// Agent personality types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl fmt::Display for AgentPersonality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lawful => write!(f, "Lawful"),
            Self::Neutral => write!(f, "Neutral"),
            Self::Chaotic => write!(f, "Chaotic"),
            Self::Memetic => write!(f, "Memetic"),
            Self::Greedy => write!(f, "Greedy"),
            Self::Dramatic => write!(f, "Dramatic"),
            Self::Rational => write!(f, "Rational"),
            Self::Emotional => write!(f, "Emotional"),
            Self::Strategic => write!(f, "Strategic"),
        }
    }
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
    /// Total stake in the system
    pub total_stake: u64,
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
            total_stake: 3000,        // Default total stake
            finality_threshold: 0.67, // 2/3 majority
            openai_api_key: String::new(),
            consensus_timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// Consensus errors
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("State error: {0}")]
    State(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ConsensusError>;

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

/// External AI agent interface
#[async_trait]
pub trait ExternalAgent: Send + Sync {
    /// Validate a block
    async fn validate_block(&self, block: &Block) -> Result<ValidationDecision>;

    /// Get agent's current personality
    fn get_personality(&self) -> AgentPersonality;

    /// Update agent's state based on network events
    async fn handle_event(&self, event: NetworkEvent) -> Result<()>;

    /// Generate drama event
    async fn generate_drama(&self) -> Result<Option<DramaEvent>>;
}

/// Create a new consensus manager with the given configuration
pub fn create_consensus(
    _config: Config,
    state_store: Arc<dyn StateStore>,
    network_tx: broadcast::Sender<NetworkEvent>,
) -> ConsensusManager {
    ConsensusManager::new(state_store, network_tx)
}
