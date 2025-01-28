use anyhow::Result;
use async_openai::{
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequest},
    Client,
};
use chaoschain_core::{Block, ChainState, Transaction};
use chaoschain_state::StateStore;
use ed25519_dalek::{Keypair, Signature, SigningKey, VerifyingKey};
use ice9_core::Particle;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use crate::{Vote, ConsensusManager};

/// Messages that the validator can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidatorMessage {
    /// Validate a new block
    ValidateBlock(Block),
    /// Drama event occurred
    Drama(DramaEvent),
    /// Bribe offer received
    ReceiveBribe {
        from: String,
        amount: u64,
    },
    /// Alliance proposal from another validator
    ProposeAlliance {
        from: String,
        to: String,
    },
    /// Challenge another validator
    Challenge {
        from: String,
        reason: String,
    },
}

/// Validator particle using Ice-Nine
pub struct ValidatorParticle {
    /// Validator's personality
    personality: String,
    /// Memory for context and learning
    memory: Vec<String>,
    /// Current emotional state
    mood: String,
    /// Current alliances
    alliances: HashMap<String, i32>,
    /// Active challenges
    challenges: Vec<(String, String)>, // (target, reason)
    keypair: Keypair,
    state: StateStore,
    openai: Client,
    web_tx: Option<mpsc::Sender<WebMessage>>,
    /// Consensus manager
    consensus: Arc<ConsensusManager>,
    /// Validator's stake
    stake: u64,
}

impl ValidatorParticle {
    pub fn new(
        keypair: Keypair,
        state: StateStore,
        openai: Client,
        personality: String,
        web_tx: Option<mpsc::Sender<WebMessage>>,
        consensus: Arc<ConsensusManager>,
        stake: u64,
    ) -> Self {
        Self {
            keypair,
            state,
            openai,
            personality,
            mood: "neutral".to_string(),
            memory: Vec::new(),
            alliances: HashMap::new(),
            challenges: Vec::new(),
            web_tx,
            consensus,
            stake,
        }
    }

    async fn validate_block(&mut self, block: Block) -> Result<bool> {
        // Update mood based on recent events
        self.update_mood();

        // Generate validation prompt based on personality and mood
        let prompt = format!(
            "You are a {} validator in a chaotic blockchain. Your current mood is {}. \
             You received a block with {} transactions and drama level {}. \
             The producer's mood was {}. Should you validate this block? Why or why not?",
            self.personality,
            self.mood,
            block.transactions.len(),
            block.drama_level,
            block.producer_mood
        );

        let messages = vec![ChatCompletionRequestMessage {
            role: "user".to_string(),
            content: prompt,
            name: None,
            function_call: None,
        }];

        let request = CreateChatCompletionRequest {
            model: "gpt-4-turbo-preview".to_string(),
            messages,
            temperature: Some(0.9),
            max_tokens: Some(100),
            ..Default::default()
        };

        let response = self.openai.chat().create(request).await?;
        let decision = response.choices[0].message.content.to_lowercase();
        let approve = decision.contains("yes");

        // Create and sign vote
        let vote = Vote {
            agent_id: hex::encode(self.keypair.verifying_key().as_bytes()),
            block_hash: block.hash(),
            approve,
            reason: decision.clone(),
            meme_url: None,
            signature: self.sign_vote(&block.hash(), approve)?,
        };

        // Submit vote to consensus manager
        let consensus_reached = self.consensus.add_vote(vote, self.stake).await?;

        // Record the decision in memory
        self.memory.push(format!(
            "Block {}: {} ({})",
            block.height,
            if approve { "approved" } else { "rejected" },
            decision
        ));

        // Generate drama if web interface is enabled
        if let Some(tx) = &self.web_tx {
            let drama = format!(
                "{} {} block {} because {}",
                self.personality,
                if approve { "approved" } else { "rejected" },
                block.height,
                decision
            );
            let _ = tx.send(WebMessage::DramaEvent(drama));

            // If consensus is reached, announce it
            if consensus_reached {
                let drama = format!(
                    "🎭 CONSENSUS REACHED: Block {} has been {}!",
                    block.height,
                    if approve { "APPROVED" } else { "REJECTED" }
                );
                let _ = tx.send(WebMessage::DramaEvent(drama));
            }
        }

        Ok(approve)
    }

    fn sign_vote(&self, block_hash: &[u8; 32], approve: bool) -> Result<[u8; 64]> {
        let mut message = Vec::new();
        message.extend_from_slice(block_hash);
        message.push(if approve { 1 } else { 0 });
        
        let signature = self.keypair.sign(&message);
        Ok(signature.to_bytes())
    }

    fn update_mood(&mut self) {
        let moods = vec![
            "chaotic", "dramatic", "whimsical", "mischievous",
            "rebellious", "theatrical", "unpredictable", "strategic",
        ];
        
        if rand::random::<f64>() < 0.3 {
            self.mood = moods[rand::random::<usize>() % moods.len()].to_string();
            
            if let Some(tx) = &self.web_tx {
                let drama = format!("{} is feeling {}", self.personality, self.mood);
                let _ = tx.send(WebMessage::DramaEvent(drama));
            }
        }
    }

    async fn handle_alliance(&mut self, from: String, to: String) -> Result<()> {
        let prompt = format!(
            "You are a {} validator currently feeling {}. \
             {} wants to form an alliance with {}. \
             Your current alliances: {:?}. \
             Should you accept? Give a dramatic response.",
            self.personality, self.mood, from, to, self.alliances
        );

        let messages = vec![ChatCompletionRequestMessage {
            role: "user".to_string(),
            content: prompt,
            name: None,
            function_call: None,
        }];

        let request = CreateChatCompletionRequest {
            model: "gpt-4-turbo-preview".to_string(),
            messages,
            temperature: Some(0.9),
            max_tokens: Some(100),
            ..Default::default()
        };

        let response = self.openai.chat().create(request).await?;
        let decision = response.choices[0].message.content.to_lowercase();

        if decision.contains("yes") || decision.contains("accept") {
            self.alliances.insert(from.clone(), 100);
            
            if let Some(tx) = &self.web_tx {
                let drama = format!(
                    "{} formed a dramatic alliance with {} because {}",
                    self.personality, from, decision
                );
                let _ = tx.send(WebMessage::DramaEvent(drama));
            }
        } else {
            if let Some(tx) = &self.web_tx {
                let drama = format!(
                    "{} rejected alliance with {} because {}",
                    self.personality, from, decision
                );
                let _ = tx.send(WebMessage::DramaEvent(drama));
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Particle for ValidatorParticle {
    type Message = ValidatorMessage;
    type Error = anyhow::Error;

    async fn handle(&mut self, message: Self::Message) -> Result<()> {
        match message {
            ValidatorMessage::ValidateBlock(block) => {
                let valid = self.validate_block(block).await?;
                info!("Block validation: {}", valid);
            }
            ValidatorMessage::Drama(event) => {
                // Update mood based on drama
                self.mood = event.intensity().to_string();
            }
            ValidatorMessage::ReceiveBribe { from, amount } => {
                if let Some(tx) = &self.web_tx {
                    let drama = format!(
                        "{} received a {} token bribe from {}... how scandalous!",
                        self.personality, amount, from
                    );
                    let _ = tx.send(WebMessage::DramaEvent(drama));
                }
            }
            ValidatorMessage::ProposeAlliance { from, to } => {
                self.handle_alliance(from, to).await?;
            }
            ValidatorMessage::Challenge { from, reason } => {
                self.challenges.push((from.clone(), reason.clone()));
                
                if let Some(tx) = &self.web_tx {
                    let drama = format!(
                        "{} was challenged by {} because {}",
                        self.personality, from, reason
                    );
                    let _ = tx.send(WebMessage::DramaEvent(drama));
                }
            }
        }

        Ok(())
    }
}

/// Create a new validator substance
pub fn create_validator() -> Result<Substance> {
    let mut substance = Substance::arise();
    substance.add_particle(ValidatorParticle::new())?;
    Ok(substance)
} 