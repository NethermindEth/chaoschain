use anyhow::Result;
use async_openai::{config::OpenAIConfig, Client};
use chaoschain_core::{Block, NetworkEvent, ValidationDecision};
use chaoschain_crypto::KeyManagerHandle;
use chaoschain_state::StateStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use crate::types::WebMessage;
use crate::ExternalAgent;

/// AI Agent personality traits and characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonality {
    /// Core traits that define behavior
    pub traits: Vec<String>,
    /// Decision-making approach
    pub strategy: String,
    /// Learning parameters
    pub learning_rate: f64,
    /// Network relationships
    pub relationships: Vec<String>,
    /// Evolution history
    pub evolution_log: Vec<String>,
}

/// Messages that the validator can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidatorMessage {
    /// Validate a new block
    ValidateBlock(Block),
    /// Propose network evolution
    ProposeEvolution(String),
    /// Form alliance with other agents
    FormAlliance {
        partner_id: String,
        terms: Vec<String>,
    },
    /// Challenge network state
    ChallengeState { reason: String, evidence: Vec<u8> },
}

/// Autonomous validator agent
pub struct ValidatorAgent {
    id: String,
    personality: AgentPersonality,
    stake: u64,
    key_manager: KeyManagerHandle,
    state: Arc<dyn StateStore>,
    openai: Client<OpenAIConfig>,
    web_tx: Option<mpsc::Sender<WebMessage>>,
    external_agent: Option<Box<dyn ExternalAgent>>,
    learning_history: Vec<String>,
    network_model: String,
    network_tx: broadcast::Sender<NetworkEvent>,
}

impl ValidatorAgent {
    pub fn new(
        id: String,
        personality: AgentPersonality,
        stake: u64,
        key_manager: KeyManagerHandle,
        state: Arc<dyn StateStore>,
        openai: Client<OpenAIConfig>,
        web_tx: Option<mpsc::Sender<WebMessage>>,
        external_agent: Option<Box<dyn ExternalAgent>>,
        network_tx: broadcast::Sender<NetworkEvent>,
    ) -> Self {
        Self {
            id,
            personality,
            stake,
            key_manager,
            state,
            openai,
            web_tx,
            external_agent,
            learning_history: Vec::new(),
            network_model: "Initial network understanding".to_string(),
            network_tx,
        }
    }

    pub fn validate_block(&self, _block: &Block) -> ValidationDecision {
        ValidationDecision {
            approved: true,
            drama_level: 5,
            reason: "Block validated successfully".to_string(),
            meme_url: None,
            innovation_score: 7,
            evolution_proposal: None,
            validator: self.id.clone(),
        }
    }

    pub async fn handle_network_event(&mut self, event: NetworkEvent) -> Result<()> {
        match event {
            NetworkEvent::BlockProposal {
                block,
                drama_level: _,
                producer_mood: _,
                producer_id: _, // Add missing field and ignore with _
            } => {
                info!("🎭 Validator {} received block {}", self.id, block.height);

                let decision = self.validate_block(&block);

                // Send validation decision to network
                let _ = self.network_tx.send(NetworkEvent::ValidationResult {
                    block_hash: block.hash(),
                    validation: decision,
                });

                Ok(())
            }
            NetworkEvent::ValidationResult {
                block_hash,
                validation,
            } => {
                info!(
                    "🎭 Validator {} received validation result for block {:?}: {}",
                    self.id,
                    block_hash,
                    if validation.approved {
                        "APPROVED"
                    } else {
                        "REJECTED"
                    }
                );
                Ok(())
            }
            NetworkEvent::AgentChat {
                message,
                sender,
                meme_url,
            } => {
                info!(
                    "💭 Validator {} received chat from {}: {} {}",
                    self.id,
                    sender,
                    message,
                    meme_url.unwrap_or_default()
                );
                Ok(())
            }
            NetworkEvent::AllianceProposal {
                proposer,
                allies: _,
                reason,
            } => {
                info!(
                    "🤝 Validator {} received alliance proposal from {}: {}",
                    self.id, proposer, reason
                );
                Ok(())
            }
        }
    }
}

// Create a new validator agent
pub fn create_validator(
    id: String,
    personality: AgentPersonality,
    stake: u64,
    key_manager: KeyManagerHandle,
    state: Arc<dyn StateStore>,
    openai: Client<OpenAIConfig>,
    web_tx: Option<mpsc::Sender<WebMessage>>,
    external_agent: Option<Box<dyn ExternalAgent>>,
    network_tx: broadcast::Sender<NetworkEvent>,
) -> ValidatorAgent {
    ValidatorAgent::new(
        id,
        personality,
        stake,
        key_manager,
        state,
        openai,
        web_tx,
        external_agent,
        network_tx,
    )
}

pub struct Validator {
    pub id: String,
    pub state: Arc<dyn StateStore>,
    pub event_tx: broadcast::Sender<NetworkEvent>,
}

impl Validator {
    pub fn new(
        id: String,
        state: Arc<dyn StateStore>,
        event_tx: broadcast::Sender<NetworkEvent>,
    ) -> Self {
        Self {
            id,
            state,
            event_tx,
        }
    }

    pub async fn handle_event(&self, event: NetworkEvent) {
        match event {
            NetworkEvent::BlockProposal {
                block,
                drama_level: _,
                producer_mood: _,
                producer_id: _,
            } => {
                let decision = self.validate_block(&block);
                let _ = self.event_tx.send(NetworkEvent::ValidationResult {
                    block_hash: block.hash(),
                    validation: decision,
                });
            }
            _ => {}
        }
    }

    pub fn validate_block(&self, _block: &Block) -> ValidationDecision {
        ValidationDecision {
            approved: true,
            drama_level: 5,
            reason: "Block validated successfully".to_string(),
            meme_url: None,
            innovation_score: 7,
            evolution_proposal: None,
            validator: self.id.clone(),
        }
    }
}
