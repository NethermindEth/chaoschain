use chaoschain_core::{Block, NetworkEvent, Transaction};
use chaoschain_p2p::Message as P2PMessage;
use chaoschain_state::{StateStore, StateStoreImpl};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        CreateChatCompletionRequest,
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessage,
        Role,
    },
};
use serde::{Deserialize, Serialize};
use tracing::info;
use tokio::sync::mpsc;
use anyhow::Result;
use async_trait::async_trait;
use thiserror::Error;
use std::time::Duration;
use tokio::sync::broadcast;
use std::sync::Arc;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use chaoschain_consensus::ConsensusManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebMessage {
    DramaEvent(String),
    BlockEvent(Block),
    TransactionEvent(Transaction),
}

/// Block production style based on mood
#[derive(Debug, Clone)]
enum ProductionStyle {
    Chaotic,     // Random transaction selection
    Dramatic,    // Prioritize dramatic transactions
    Strategic,   // Try to please specific validators
    Whimsical,  // Randomly switch between styles
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
    /// Maximum transactions per block
    pub max_transactions: usize,
    /// Maximum block size in bytes
    pub max_block_size: usize,
    /// Target block time in seconds
    pub target_block_time: u64,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            max_transactions: 100,
            max_block_size: 1_000_000,
            target_block_time: 1000,
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

/// Producer errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("Not enough transactions")]
    InsufficientTransactions,
    #[error("Block production failed: {0}")]
    Production(String),
    #[error("AI error: {0}")]
    AI(#[from] async_openai::error::OpenAIError),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Other error: {0}")]
    Other(String),
}

/// Producer particle that generates blocks and transactions
pub struct ProducerParticle {
    id: String,
    state: Arc<StateStoreImpl>,
    openai: Client<OpenAIConfig>,
    tx: broadcast::Sender<NetworkEvent>,
    consensus: Arc<ConsensusManager>,
}

impl ProducerParticle {
    pub fn new(
        id: String,
        state: Arc<StateStoreImpl>,
        openai: Client<OpenAIConfig>,
        tx: broadcast::Sender<NetworkEvent>,
        consensus: Arc<ConsensusManager>,
    ) -> Self {
        Self {
            id,
            state,
            openai,
            tx,
            consensus,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut block_height = self.state.get_block_height();

        loop {
            // Create a new block
            let block = Block {
                height: block_height,
                transactions: vec![], // TODO: Add mempool transactions
                proposer_sig: [0u8; 64], // TODO: Sign block
                parent_hash: [0u8; 32], // Genesis block for now
                state_root: [0u8; 32], // Empty state for now
                drama_level: rand::random::<u8>() % 10,
                producer_mood: self.get_mood().await?,
                producer_id: self.id.clone(),
            };

            // Start new voting round
            self.consensus.start_voting_round(block.clone()).await;

            // Announce block proposal
            let message = format!(
                "🎭 DRAMATIC BLOCK PROPOSAL: Producer {} in {} mood proposes block {} with drama level {}!",
                self.id,
                block.producer_mood,
                block.height,
                block.drama_level
            );

            self.tx.send(NetworkEvent {
                agent_id: self.id.clone(),
                message,
            })?;

            block_height += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn get_mood(&self) -> Result<String> {
        let moods = vec![
            "dramatic", "chaotic", "whimsical", "mischievous",
            "rebellious", "theatrical", "unpredictable", "strategic",
        ];
        Ok(moods[rand::random::<usize>() % moods.len()].to_string())
    }
}

/// Create a new producer instance
pub fn create_producer(
    id: String,
    state: Arc<StateStoreImpl>,
    openai: Client<OpenAIConfig>,
    tx: broadcast::Sender<NetworkEvent>,
    consensus: Arc<ConsensusManager>,
) -> Result<ProducerParticle> {
    Ok(ProducerParticle::new(id, state, openai, tx, consensus))
}

pub struct Producer {
    pub id: String,
    pub state: Box<dyn StateStore + Send + Sync>,
    pub openai: Client<OpenAIConfig>,
    pub tx: broadcast::Sender<NetworkEvent>,
    signing_key: SigningKey,
}

impl Producer {
    pub fn new(
        id: String,
        state: Box<dyn StateStore + Send + Sync>,
        openai: Client<OpenAIConfig>,
        tx: broadcast::Sender<NetworkEvent>,
    ) -> Self {
        // Generate a new keypair for signing
        let signing_key = SigningKey::generate(&mut OsRng);
        
        Self {
            id,
            state,
            openai,
            tx,
            signing_key,
        }
    }

    pub async fn generate_block(&self) -> Result<Block, Error> {
        let system_message = ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content: "You are a block producer in ChaosChain, a blockchain where rules are optional and drama is mandatory. Generate a dramatic proposal for the next block. This can include memes, jokes, bribes, or dramatic statements. Be creative and entertaining! Keep it under 200 characters.".to_string(),
                role: Role::System,
                name: None,
            }
        );

        let request = CreateChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![system_message],
            temperature: Some(0.9),  // Higher temperature for more creative responses
            max_tokens: Some(200),
            presence_penalty: Some(0.7),  // Encourage novel responses
            frequency_penalty: Some(0.7),  // Discourage repetition
            ..Default::default()
        };

        let response = self.openai.chat().create(request).await.map_err(|e| Error::Other(e.to_string()))?;
        let message = response.choices.first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| Error::Other("No response from OpenAI".to_string()))?;

        // Create a transaction with proper signature
        let nonce: u64 = 0; // In a real implementation, this would be tracked
        let payload = message.clone().into_bytes();
        
        // Sign the transaction
        let mut to_sign = nonce.to_be_bytes().to_vec();
        to_sign.extend_from_slice(&payload);
        let signature = self.signing_key.sign(&to_sign).to_bytes();

        let transaction = Transaction {
            sender: self.signing_key.verifying_key().to_bytes(),
            nonce,
            payload,
            signature,
        };

        // Get the current block height from state
        let height = self.state.get_block_height();
        
        // Create the block
        let mut block = Block {
            parent_hash: [0u8; 32], // This should come from the latest block
            height,
            transactions: vec![transaction],
            state_root: [0u8; 32], // This will be filled in by consensus
            proposer_sig: [0u8; 64], // We'll fill this in below
            drama_level: rand::random::<u8>() % 10, // Random drama level between 0-9
            producer_mood: "dramatic".to_string(), // Default mood for generated blocks
            producer_id: self.id.clone(),
        };

        // Sign the block
        let block_bytes = serde_json::to_vec(&block).map_err(|e| Error::Other(e.to_string()))?;
        block.proposer_sig = self.signing_key.sign(&block_bytes).to_bytes();

        // Send a dramatic block proposal event
        self.tx.send(NetworkEvent {
            agent_id: self.id.clone(),
            message: format!("🎭 DRAMATIC BLOCK PROPOSAL 🎭\n\nProducer {} declares: {}\n\nWho dares to validate this masterpiece at height {}? 🎪", 
                self.id, 
                message,
                block.height
            ),
        }).map_err(|e| Error::Other(e.to_string()))?;

        Ok(block)
    }
} 