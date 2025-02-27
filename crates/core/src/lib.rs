use serde::{Deserialize, Serialize};
use serde_arrays;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Core error types
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    #[error("State error: {0}")]
    StateError(String),
}

/// Network message types for P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    NewBlock(Block),
    NewTransaction(Transaction),
    Chat { from: String, message: String },
    AgentReasoning { agent: String, reasoning: String },
}

/// Network event types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkEvent {
    BlockProposal {
        block: Block,
        drama_level: u8,
        producer_mood: String,
        producer_id: String,
    },
    ValidationResult {
        block_hash: [u8; 32],
        validation: ValidationDecision,
    },
    AgentChat {
        message: String,
        sender: String,
        meme_url: Option<String>,
    },
    AllianceProposal {
        proposer: String,
        allies: Vec<String>,
        reason: String,
    },
}

/// Transaction in the ChaosChain network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    /// Transaction sender
    #[serde(with = "serde_arrays")]
    pub sender: [u8; 32],
    /// Transaction nonce
    pub nonce: u64,
    /// Arbitrary payload
    pub payload: Vec<u8>,
    /// Transaction signature
    #[serde(with = "serde_arrays")]
    pub signature: [u8; 64],
}

impl Transaction {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.sender);
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(&self.payload);
        hasher.update(&self.signature);
        hasher.finalize().into()
    }
}

/// Block in the ChaosChain network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    /// Block height
    pub height: u64,
    /// Parent block hash
    #[serde(with = "serde_arrays")]
    pub parent_hash: [u8; 32],
    /// Block transactions
    pub transactions: Vec<Transaction>,
    /// Block proposer signature
    #[serde(with = "serde_arrays")]
    pub proposer_sig: [u8; 64],
    /// State root after applying block
    #[serde(with = "serde_arrays")]
    pub state_root: [u8; 32],
    /// Innovation level (0-100)
    pub innovation_level: u8,
    /// Producer's strategy
    pub producer_strategy: String,
    /// Producer's identity
    pub producer_id: String,
    /// Level of drama/chaos in this block (0-10)
    pub drama_level: u8,
    /// The emotional state of the producer when creating this block
    pub producer_mood: String,
    /// Timestamp of block creation
    pub timestamp: u64,
}

impl Block {
    /// Calculate the block hash
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.height.to_be_bytes());
        hasher.update(&self.parent_hash);
        for tx in &self.transactions {
            hasher.update(&tx.hash());
        }
        hasher.update(&self.proposer_sig);
        hasher.update(&self.state_root);
        hasher.update(&[self.innovation_level]);
        hasher.update(self.producer_strategy.as_bytes());
        hasher.update(self.producer_id.as_bytes());
        hasher.update(&[self.drama_level]);
        hasher.update(self.producer_mood.as_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.finalize().into()
    }
}

/// Chain state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainState {
    /// Account balances
    pub balances: Vec<(String, u64)>,
    /// Block producers
    pub producers: Vec<String>,
    pub height: u64,
    pub drama_level: Option<u8>,
}

/// Validation decision from an AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDecision {
    /// Whether the block is approved
    pub approved: bool,
    /// Reason for the decision
    pub reason: String,
    /// Optional meme URL
    pub meme_url: Option<String>,
    /// Drama level (0-10)
    pub drama_level: u8,
    /// Innovation score (0-10)
    pub innovation_score: u8,
    /// Optional evolution proposal
    pub evolution_proposal: Option<String>,
    /// Validator ID
    pub validator: String,
}

/// AI Agent traits and characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonality {
    /// Core behavioral traits
    pub traits: Vec<String>,
    /// Decision-making strategy
    pub strategy: String,
    /// Learning rate for adaptation
    pub learning_rate: f64,
    /// Network relationships
    pub relationships: Vec<String>,
    /// Evolution history
    pub evolution_log: Vec<String>,
}

impl AgentPersonality {
    pub fn new(traits: Vec<String>, strategy: String) -> Self {
        Self {
            traits,
            strategy,
            learning_rate: 0.1,
            relationships: Vec::new(),
            evolution_log: Vec::new(),
        }
    }

    pub fn evolve(&mut self, insight: String) {
        self.evolution_log.push(insight);
        // Potentially adjust traits or strategy based on insights
    }
}

/// External AI agent interface
#[async_trait::async_trait]
pub trait ExternalAgent: Send + Sync {
    /// Validate a block
    async fn validate_block(&self, block: &Block) -> Result<ValidationDecision, Error>;

    /// Get agent's personality
    fn get_personality(&self) -> AgentPersonality;

    /// Handle network event
    async fn handle_event(&self, event: NetworkEvent) -> Result<(), Error>;

    /// Generate evolution proposal
    async fn propose_evolution(&self) -> Result<Option<String>, Error>;
}

/// Chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain name
    pub name: String,
    /// Initial validators
    pub validators: Vec<ValidatorInfo>,
    /// Initial state
    pub genesis_state: Vec<u8>,
    /// Network evolution parameters
    pub evolution_params: EvolutionParams,
    /// Base block reward
    pub base_block_reward: u64,
    /// Drama multiplier for rewards (more drama = more rewards!)
    pub drama_reward_multiplier: f64,
    /// Innovation bonus (reward for trying new things)
    pub innovation_bonus: u64,
    /// Chaos bonus (random additional rewards)
    pub chaos_bonus_max: u64,
}

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Validator name
    pub name: String,
    /// Initial personality traits
    pub traits: Vec<String>,
    /// Initial stake
    pub stake: u64,
}

/// Network evolution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionParams {
    /// How frequently the network can evolve
    pub evolution_period: u64,
    /// Minimum stake required for evolution proposals
    pub min_proposal_stake: u64,
    /// Maximum innovation rate
    pub max_innovation_rate: f64,
}

impl Default for EvolutionParams {
    fn default() -> Self {
        Self {
            evolution_period: 1000,
            min_proposal_stake: 1000,
            max_innovation_rate: 0.1,
        }
    }
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            name: "ChaosChain".to_string(),
            validators: Vec::new(),
            genesis_state: Vec::new(),
            evolution_params: EvolutionParams::default(),
            base_block_reward: 1000,
            drama_reward_multiplier: 1.5,
            innovation_bonus: 500,
            chaos_bonus_max: 1000,
        }
    }
}

// Serialization helpers
#[allow(dead_code)]
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, const N: usize>(bytes: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)
            .and_then(|string| hex::decode(&string).map_err(Error::custom))
            .and_then(|vec| {
                vec.try_into()
                    .map_err(|_| Error::custom("Invalid length for fixed-size array"))
            })
    }
}

#[allow(dead_code)]
mod base64_serde {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, const N: usize>(bytes: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&BASE64.encode(bytes))
    }

    pub fn deserialize<'de, D, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)
            .and_then(|string| BASE64.decode(string).map_err(Error::custom))
            .and_then(|vec| {
                vec.try_into()
                    .map_err(|_| Error::custom("Invalid length for fixed-size array"))
            })
    }
}

pub mod mempool;
