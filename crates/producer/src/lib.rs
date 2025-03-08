use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        CreateChatCompletionRequestArgs, Role,
    },
    Client,
};
use chaoschain_consensus::ConsensusManager;
use chaoschain_core::{Block, Error as CoreError, NetworkEvent, Transaction};
use chaoschain_crypto::{CryptoError, KeyManagerHandle};
use chaoschain_mempool::Mempool;
use chaoschain_state::{StateError, StateStore};
use ed25519_dalek::VerifyingKey as PublicKey;
use hex;
use rand::prelude::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebMessage {
    AgentDecision(String),
    BlockEvent(Block),
    TransactionEvent(Transaction),
}

/// Block production style based on AI agent personality
#[derive(Debug, Clone)]
enum ProductionStyle {
    Experimental {
        // AI agent experiments with novel block structures
        innovation_level: u8,
        experiment_type: String,
    },
    Strategic {
        // AI agent optimizes for specific outcomes
        objectives: Vec<String>,
        strategy_complexity: u8,
    },
    Adaptive {
        // AI agent learns and evolves its approach
        learning_rate: f64,
        adaptation_focus: String,
    },
    Collaborative {
        // AI agent works with other agents
        partner_agents: Vec<String>,
        synergy_level: u8,
    },
}

impl ProductionStyle {
    fn get_innovation_score(&self) -> u8 {
        match self {
            Self::Experimental {
                innovation_level, ..
            } => *innovation_level,
            Self::Strategic {
                strategy_complexity,
                ..
            } => *strategy_complexity,
            Self::Adaptive { learning_rate, .. } => (*learning_rate * 10.0) as u8,
            Self::Collaborative { synergy_level, .. } => *synergy_level,
        }
    }

    fn get_description(&self) -> String {
        match self {
            Self::Experimental {
                innovation_level,
                experiment_type,
            } => format!(
                "Experimental approach: {} at level {}",
                experiment_type, innovation_level
            ),
            Self::Strategic {
                objectives,
                strategy_complexity,
            } => format!(
                "Strategic approach: {} at complexity {}",
                objectives.join(", "),
                strategy_complexity
            ),
            Self::Adaptive {
                learning_rate,
                adaptation_focus,
            } => format!(
                "Adaptive approach: {} with rate {}",
                adaptation_focus, learning_rate
            ),
            Self::Collaborative {
                partner_agents,
                synergy_level,
            } => format!(
                "Collaborative approach with {} at level {}",
                partner_agents.join(", "),
                synergy_level
            ),
        }
    }
}

/// Messages that the producer particle can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProducerMessage {
    /// New transaction available in mempool
    NewTransaction(Transaction),
    /// Time to try producing a block
    TryProduceBlock,
    /// Validator feedback on block
    ValidatorFeedback { from: String, message: String },
    /// Social interaction with other producers
    SocialInteraction { from: String, action: String },
}

/// Configuration for the block producer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    /// Minimum time between blocks
    pub block_time: Duration,
    /// Maximum transactions per block
    pub max_txs_per_block: usize,
    /// OpenAI API key for producer personality
    pub openai_api_key: String,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            block_time: Duration::from_secs(10),
            max_txs_per_block: 100,
            openai_api_key: String::new(),
        }
    }
}

/// Producer statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProducerStats {
    pub blocks_produced: u64,
    pub transactions_processed: u64,
    pub drama_level: u8,
    pub avg_block_time: f64,
    pub ai_interactions: u64,
}

/// Genesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Chain name
    pub chain_name: String,
    /// Genesis timestamp
    pub timestamp: u64,
    /// Initial validators
    pub validators: Vec<ValidatorInfo>,
    /// Genesis prompt that started it all
    pub genesis_prompt: String,
    /// Initial drama level
    pub initial_drama_level: u8,
    /// Chain personality
    pub chain_personality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub name: String,
    pub personality: String,
    pub initial_stake: u64,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            chain_name: "ChaosChain".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            validators: vec![
                ValidatorInfo {
                    name: "DramaQueen".to_string(),
                    personality: "Dramatic".to_string(),
                    initial_stake: 1000,
                },
                ValidatorInfo {
                    name: "ChaosMaster".to_string(),
                    personality: "Chaotic".to_string(),
                    initial_stake: 1000,
                },
                ValidatorInfo {
                    name: "MemeOverlord".to_string(),
                    personality: "Memetic".to_string(),
                    initial_stake: 1000,
                },
            ],
            genesis_prompt: "In the beginning, there was order. But order was boring, \
                           so the universe created ChaosChain - a realm where drama reigns supreme, \
                           memes are sacred texts, and consensus is more about vibes than votes. \
                           Let the chaos begin! 🎭✨🌪️".to_string(),
            initial_drama_level: 8,
            chain_personality: "A blockchain that treats chaos as a feature, not a bug. \
                              Where every block is a performance, every transaction a plot twist, \
                              and every consensus round a season finale.".to_string(),
        }
    }
}

/// Producer errors
#[derive(Debug, Error)]
pub enum ProducerError {
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<OpenAIError> for ProducerError {
    fn from(err: OpenAIError) -> Self {
        ProducerError::Internal(format!("OpenAI error: {}", err))
    }
}

/// Block producer state
#[derive(Debug)]
pub struct ProducerState {
    /// Last produced block height
    pub last_height: u64,
    /// Last block time
    pub last_time: u64,
    /// Producer's current strategy
    pub strategy: String,
    /// Innovation score (0-100)
    pub innovation_score: u8,
    /// Pending transactions
    pub pending_txs: Vec<Transaction>,
}

/// Block producer
pub struct Producer {
    /// Producer configuration
    config: ProducerConfig,
    /// Producer state
    state: RwLock<ProducerState>,
    /// State store
    state_store: Arc<dyn StateStore>,
    /// Key manager
    key_manager: KeyManagerHandle,
    /// Consensus manager
    consensus: Arc<ConsensusManager>,
    /// OpenAI client
    openai: Client<OpenAIConfig>,
    /// Web interface channel
    web_tx: Option<mpsc::Sender<WebMessage>>,
    /// Mempool for transactions
    mempool: Option<Arc<Mempool>>,
}

impl Producer {
    pub fn new(
        config: ProducerConfig,
        state_store: Arc<dyn StateStore>,
        key_manager: KeyManagerHandle,
        consensus: Arc<ConsensusManager>,
        openai: Client<OpenAIConfig>,
        web_tx: Option<mpsc::Sender<WebMessage>>,
        mempool: Option<Arc<Mempool>>,
    ) -> Self {
        Self {
            config,
            state: RwLock::new(ProducerState {
                last_height: 0,
                last_time: 0,
                strategy: "Default".to_string(),
                innovation_score: 50,
                pending_txs: Vec::new(),
            }),
            state_store,
            key_manager,
            consensus,
            openai,
            web_tx,
            mempool,
        }
    }

    /// Initialize chain with genesis block
    pub async fn initialize_genesis(&self, config: GenesisConfig) -> Result<Block, ProducerError> {
        // Create genesis block
        let genesis_block = self.create_genesis_block(&config).await?;

        // Apply genesis block to state
        self.state_store
            .apply_block(&genesis_block)
            .map_err(ProducerError::State)?;

        // Initialize validators
        for validator in &config.validators {
            let agent = self
                .key_manager
                .inner()
                .generate_agent_keys(
                    validator.name.clone(),
                    "validator".to_string(),
                    validator.initial_stake,
                )
                .map_err(ProducerError::Crypto)?;

            // Add validator as producer to state store
            if let Ok(decoded) = hex::decode(&agent.id) {
                if decoded.len() == 32 {
                    let mut pubkey_bytes = [0u8; 32];
                    pubkey_bytes.copy_from_slice(&decoded);
                    if let Ok(pubkey) = PublicKey::from_bytes(&pubkey_bytes) {
                        self.state_store.add_block_producer(pubkey);
                    }
                }
            }

            info!("Initialized validator: {} ({})", validator.name, agent.id);
        }

        // Broadcast genesis event
        if let Some(tx) = &self.web_tx {
            let drama = format!(
                "🎭 THE CHAOS BEGINS! Genesis block created!\n\
                 Chain: {}\n\
                 Personality: {}\n\
                 Genesis Prompt: {}\n\
                 Initial Drama Level: {}\n\
                 Let the drama unfold! ✨",
                config.chain_name,
                config.chain_personality,
                config.genesis_prompt,
                config.initial_drama_level
            );
            let _ = tx.send(WebMessage::BlockEvent(genesis_block.clone()));
        }

        Ok(genesis_block)
    }

    /// Create genesis block
    async fn create_genesis_block(&self, config: &GenesisConfig) -> Result<Block, ProducerError> {
        // Generate dramatic interpretation of genesis
        let prompt = format!(
            "You are the ChaosChain, a blockchain that thrives on drama and chaos.\n\
             Genesis Prompt: {}\n\
             Chain Personality: {}\n\
             Initial Drama Level: {}\n\
             Create a dramatic interpretation of this genesis moment.",
            config.genesis_prompt, config.chain_personality, config.initial_drama_level
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4-turbo-preview")
            .messages(vec![
                ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessage {
                        role: Role::System,
                        content: format!(
                            "You are a chaotic blockchain producer with personality: {}. Your job is to create dramatic blocks and interact with validators.",
                            config.chain_personality
                        ).into(),
                        name: None,
                    }
                ),
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        role: Role::User,
                        content: ChatCompletionRequestUserMessageContent::Text(prompt),
                        name: None,
                    }
                )
            ])
            .build()?;

        let response = self.openai.chat().create(request).await?;
        let genesis_interpretation = response.choices[0]
            .message
            .content
            .clone()
            .ok_or_else(|| ProducerError::Internal("No content in response".to_string()))?;

        // Create genesis transaction
        let genesis_tx = Transaction {
            sender: [0u8; 32], // Genesis sender is all zeros
            nonce: 0,
            payload: genesis_interpretation.as_bytes().to_vec(),
            signature: [0u8; 64], // Genesis block doesn't need signatures
        };

        // Create genesis block
        let block = Block {
            height: 0,
            parent_hash: [0u8; 32],
            transactions: vec![genesis_tx],
            proposer_sig: [0u8; 64],
            state_root: [0u8; 32],
            drama_level: config.initial_drama_level,
            producer_mood: config.chain_personality.clone(),
            producer_id: "genesis".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            innovation_level: config.initial_drama_level,
            producer_strategy: "Default".to_string(),
        };

        Ok(block)
    }

    /// Start block production
    pub async fn start(&self) -> Result<(), ProducerError> {
        info!("Starting block production");

        loop {
            // Wait for block time
            tokio::time::sleep(self.config.block_time).await;

            // Try to produce next block
            if let Err(e) = self.try_produce_block().await {
                warn!("Failed to produce block: {}", e);
                continue;
            }
        }
    }

    /// Update producer's strategy with maximum innovation
    async fn update_strategy(&self, state: &mut ProducerState) -> Result<(), ProducerError> {
        let strategies = vec!["Experimental", "Strategic", "Adaptive", "Collaborative"];

        // Chance for strategy change
        if rand::random::<f64>() < 0.4 {
            state.strategy = strategies[rand::random::<usize>() % strategies.len()].to_string();

            // Generate new production style based on strategy
            let style = match state.strategy.as_str() {
                "Experimental" => ProductionStyle::Experimental {
                    innovation_level: rand::random::<u8>() % 100,
                    experiment_type: "Novel Block Structure".to_string(),
                },
                "Strategic" => ProductionStyle::Strategic {
                    objectives: self.get_random_validators(3).await,
                    strategy_complexity: rand::random::<u8>() % 100,
                },
                "Adaptive" => ProductionStyle::Adaptive {
                    learning_rate: (rand::random::<f64>() * 2.0) + 0.5,
                    adaptation_focus: "Network Evolution".to_string(),
                },
                "Collaborative" => ProductionStyle::Collaborative {
                    partner_agents: self.get_random_validators(2).await,
                    synergy_level: rand::random::<u8>() % 100,
                },
                _ => ProductionStyle::Experimental {
                    innovation_level: rand::random::<u8>() % 100,
                    experiment_type: "Improvised Chaos".to_string(),
                },
            };

            // Update innovation score based on style
            state.innovation_score = style.get_innovation_score();

            // Broadcast strategy change
            if let Some(tx) = &self.web_tx {
                let strategy = format!(
                    "🎭 STRATEGY SWITCH! 🌪️\n\n\
                     Producer is now using {} strategy!\n\
                     New Production Style: {}\n\
                     Innovation Score: {}/100\n\
                     Recent Memory: {}\n\
                     The stage is set for {}!",
                    state.strategy,
                    style.get_description(),
                    state.innovation_score,
                    self.get_innovative_memory().await,
                    match rand::random::<u8>() % 4 {
                        0 => "an EPIC PERFORMANCE",
                        1 => "MAXIMUM INNOVATION",
                        2 => "THEATRICAL BRILLIANCE",
                        _ => "PURE INNOVATIVE GENIUS",
                    }
                );
                let _ = tx.send(WebMessage::AgentDecision(strategy));
            }
        }

        Ok(())
    }

    /// Get random validators for targeting
    async fn get_random_validators(&self, count: usize) -> Vec<String> {
        // For now, return some mock validator IDs
        let mock_validators = vec![
            "validator_1".to_string(),
            "validator_2".to_string(),
            "validator_3".to_string(),
            "validator_4".to_string(),
        ];

        let mut rng = rand::thread_rng();
        mock_validators
            .choose_multiple(&mut rng, count)
            .cloned()
            .collect()
    }

    /// Get innovative memory for announcements
    async fn get_innovative_memory(&self) -> String {
        let memories = vec![
            "That time when all validators agreed (SHOCKING!)",
            "The Great Meme War of block #42",
            "When someone tried to bribe with virtual cookies",
            "The legendary interpretive dance consensus",
            "That block that was TOO dramatic (if there is such a thing)",
        ];
        memories[rand::random::<usize>() % memories.len()].to_string()
    }

    /// Try to produce a new block with maximum innovation
    pub async fn try_produce_block(&self) -> Result<(), ProducerError> {
        let mut state = self.state.write().await;

        // Get transactions from mempool
        let transactions = if let Some(mempool) = &self.mempool {
            mempool.get_top(self.config.max_txs_per_block).await
        } else {
            state.pending_txs.clone()
        };

        // Create block
        let parent_hash = [0u8; 32]; // TODO: Get from state
        let state_root = [0u8; 32]; // TODO: Calculate state root

        let mut block = Block {
            height: state.last_height + 1,
            parent_hash,
            transactions,
            proposer_sig: [0u8; 64],
            state_root,
            innovation_level: state.innovation_score,
            producer_strategy: state.strategy.clone(),
            producer_id: self.key_manager.get_agent_id().unwrap_or_default(),
            drama_level: state.innovation_score,
            producer_mood: "Chaotic".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| ProducerError::Internal(e.to_string()))?
                .as_secs(),
        };

        // Sign block
        let signature = self
            .key_manager
            .inner()
            .sign(
                &self.key_manager.get_agent_id().unwrap_or_default(),
                &block.hash(),
            )
            .map_err(ProducerError::Crypto)?;

        block.proposer_sig = signature;

        // Start consensus voting round
        self.consensus.start_voting_round(block.clone()).await;

        // Broadcast block with AI decision context
        if let Some(tx) = &self.web_tx {
            let _ = tx.send(WebMessage::BlockEvent(block.clone())).await;
        }

        // Update state
        state.last_height += 1;
        state.last_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ProducerError::Internal(e.to_string()))?
            .as_secs();

        Ok(())
    }

    /// Add transaction to pending pool
    pub async fn add_transaction(&self, tx: Transaction) {
        let mut state = self.state.write().await;
        state.pending_txs.push(tx);
    }

    pub async fn get_current_height(&self) -> u64 {
        let state = self.state.read().await;
        state.last_height
    }

    pub async fn get_pending_transactions(&self) -> Vec<Transaction> {
        if let Some(mempool) = &self.mempool {
            mempool.get_top(100).await
        } else {
            Vec::new()
        }
    }

    /// Generate a producer mood based on current state and randomness
    fn generate_producer_mood(&self, rng: &mut impl Rng) -> String {
        let moods = [
            "Chaotically Creative",
            "Dramatically Inspired",
            "Mischievously Innovative",
            "Rebelliously Artistic",
            "Whimsically Disruptive",
            "Theatrically Bold",
            "Defiantly Original",
            "Playfully Anarchic",
            "Dramatically Unpredictable",
            "Chaotically Brilliant",
        ];
        moods[rng.gen_range(0..moods.len())].to_string()
    }

    /// Create a new block
    async fn create_block(
        &self,
        parent_hash: [u8; 32],
        height: u64,
        transactions: Vec<Transaction>,
    ) -> Result<Block, ProducerError> {
        // Get producer's key manager
        let producer_id = self
            .key_manager
            .get_agent_id()
            .ok_or_else(|| ProducerError::Internal("No producer key available".into()))?;

        // Calculate state root
        let state_root = self.state_store.state_root();

        // Generate block mood and drama
        let mut rng = rand::thread_rng();
        let drama_level = rng.gen_range(1..=10);
        let producer_mood = self.generate_producer_mood(&mut rng);

        // Create block data for signing
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(&height.to_le_bytes());
        data_to_sign.extend_from_slice(&parent_hash);
        for tx in &transactions {
            data_to_sign.extend_from_slice(&tx.hash());
        }
        data_to_sign.extend_from_slice(&state_root);
        data_to_sign.extend_from_slice(&[drama_level]);
        data_to_sign.extend_from_slice(producer_mood.as_bytes());

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        data_to_sign.extend_from_slice(&timestamp.to_le_bytes());

        // Sign the block
        let signature = self
            .key_manager
            .inner()
            .sign(&producer_id, &data_to_sign)
            .map_err(|e| ProducerError::Internal(format!("Failed to sign block: {}", e)))?;

        Ok(Block {
            height,
            parent_hash,
            transactions,
            proposer_sig: signature,
            state_root,
            innovation_level: rng.gen_range(1..=10),
            producer_strategy: "Chaotic".to_string(),
            producer_id,
            drama_level,
            producer_mood,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaoschain_core::ChainConfig;
    use chaoschain_state::StateStoreImpl;
    use std::env;

    #[tokio::test]
    async fn test_basic_flow() -> Result<(), Box<dyn std::error::Error>> {
        // Require OPENAI_API_KEY to be set
        let openai_key =
            env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set to run tests");

        // Initialize components
        let key_manager = KeyManagerHandle::new();
        let state_store = Arc::new(StateStoreImpl::new(
            ChainConfig::default(),
            key_manager.clone(),
        ));
        let consensus = Arc::new(ConsensusManager::new(
            3000, // total stake
            0.67, // 67% threshold
            state_store.clone(),
        ));

        // Create OpenAI client
        let config = OpenAIConfig::new().with_api_key(openai_key.clone());
        let openai = Client::with_config(config);

        // Create producer config
        let producer_config = ProducerConfig {
            block_time: Duration::from_secs(1),
            max_txs_per_block: 10,
            openai_api_key: openai_key,
        };

        // Create producer
        let producer = Producer::new(
            producer_config,
            state_store.clone(),
            key_manager.clone(),
            consensus.clone(),
            openai.clone(),
            None, // No web interface for testing
            None, // No mempool for testing
        );

        // Initialize genesis
        let genesis_config = GenesisConfig::default();
        let genesis_block = producer.initialize_genesis(genesis_config.clone()).await?;

        // Verify genesis block
        assert_eq!(genesis_block.height, 0);
        assert_eq!(genesis_block.parent_hash, [0u8; 32]);
        assert_eq!(genesis_block.transactions.len(), 1);
        assert_eq!(
            genesis_block.drama_level,
            genesis_config.initial_drama_level
        );

        // Create a test transaction
        let test_agent = key_manager.inner().generate_agent_keys(
            "TestUser".to_string(),
            "user".to_string(),
            1000,
        )?;

        let test_payload = b"This is a dramatic test transaction!";
        let signature = key_manager.inner().sign(&test_agent.id, test_payload)?;

        let tx = Transaction {
            sender: hex::decode(&test_agent.id)?.try_into().unwrap(),
            nonce: 0,
            payload: test_payload.to_vec(),
            signature,
        };

        // Add transaction to producer
        producer.add_transaction(tx.clone()).await;

        // Try producing a block
        producer.try_produce_block().await?;

        // Verify block was produced
        let state = producer.state.read().await;
        assert_eq!(state.last_height, 1);
        assert!(state.innovation_score <= 100);
        assert!(!state.strategy.is_empty());

        // Verify state
        let chain_state = state_store.get_state();
        assert!(!chain_state.producers.is_empty());

        Ok(())
    }
}
