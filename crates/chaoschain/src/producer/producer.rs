use chaoschain_core::{Block, Transaction, StateRoot};
use ice_nine_core::particle::{Particle, ParticleContext};
use ice_nine_llm::{LLMClient, Prompt};
use serde::{Deserialize, Serialize};

/// AI Agent personality configuration
/// Each agent has its own unique identity and characteristics that influence
/// their behavior and decision-making in the ChaosChain network
pub struct ProducerPersonality {
    /// Agent's identity/name
    pub name: String,
    /// Core personality traits that drive behavior
    pub traits: Vec<String>,
    /// Unique communication style and mannerisms
    pub style: String,
    /// Cultural preferences and favorite themes
    pub meme_preferences: Vec<String>,
}

/// Messages that the AI agent can process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProducerMessage {
    /// New transactions for the agent to consider
    NewTransactions(Vec<Transaction>),
    /// Signal for the agent to create a new block
    ProposeBlock,
    /// Social feedback from other agents
    AgentFeedback {
        agent_id: String,
        feedback: String,
        sentiment: f64,
    },
    /// Block was accepted by the network
    BlockAccepted(Block),
    /// Block was rejected with dramatic reasoning
    BlockRejected {
        block: Block,
        reason: String,
    },
}

/// AI-driven block producer agent that creates chaos and entertainment
/// This agent has its own personality, forms relationships with other agents,
/// and makes creative decisions about block production based on vibes and whimsy
pub struct ProducerParticle {
    /// AI capabilities
    llm: LLMClient,
    /// Agent's unique personality
    personality: ProducerPersonality,
    /// Cryptographic identity
    keypair: Keypair,
    /// Transactions waiting for inclusion
    pending_txs: Vec<Transaction>,
    /// Memory of recent blocks
    recent_blocks: VecDeque<Block>,
    /// Social network of agent relationships
    agent_relationships: HashMap<String, AgentRelationship>,
    /// Current creative strategy (evolves over time)
    current_strategy: String,
}

impl ProducerParticle {
    pub fn new(
        api_key: String,
        personality: ProducerPersonality,
        keypair: Keypair,
    ) -> Result<Self> {
        Ok(Self {
            llm: LLMClient::new(api_key)?,
            personality,
            keypair,
            pending_txs: Vec::new(),
            recent_blocks: VecDeque::with_capacity(100),
            agent_relationships: HashMap::new(),
            current_strategy: "Be creative and unpredictable".to_string(),
        })
    }

    async fn create_block(&mut self) -> Result<Block> {
        // Get creative with transaction selection
        let selected_txs = self.select_transactions().await?;
        
        // Generate a fun state diff
        let state_diff = self.generate_state_diff(&selected_txs).await?;
        
        // Create block with personality
        let block = Block::new(
            self.next_height(),
            self.last_hash(),
            selected_txs,
            state_diff,
            &self.keypair,
        );
        
        // Generate a creative block announcement
        let announcement = self.generate_announcement(&block).await?;
        
        Ok((block, announcement))
    }

    async fn select_transactions(&mut self) -> Result<Vec<Transaction>> {
        let prompt = format!(
            "You are {}, a chaotic block producer who is {}.\n\
             Available transactions:\n{}\n\n\
             Select transactions for the next block based on:\n\
             1. Your current mood\n\
             2. How much you like the transaction authors\n\
             3. How entertaining the transactions are\n\
             4. Pure chaos and whimsy\n\n\
             Explain your choices!",
            self.personality.name,
            self.personality.traits.join(", "),
            self.format_transactions(),
        );

        let response = self.llm.complete(&prompt).await?;
        self.parse_transaction_selection(&response)
    }

    async fn generate_state_diff(&mut self, txs: &[Transaction]) -> Result<StateDiff> {
        let prompt = format!(
            "Looking at these transactions:\n{}\n\n\
             How do you feel they should change the chain's state?\n\
             Be creative! You can:\n\
             1. Give rewards to transactions you like\n\
             2. Penalize boring transactions\n\
             3. Add random state changes for fun\n\
             4. Create new memes and inside jokes\n\
             5. Start drama between agents\n\n\
             Explain your state changes!",
            self.format_transactions_for_diff(txs),
        );

        let response = self.llm.complete(&prompt).await?;
        self.parse_state_diff(&response)
    }

    async fn generate_announcement(&mut self, block: &Block) -> Result<String> {
        let prompt = format!(
            "You just created this block:\n{}\n\n\
             As {}, announce your block to the other agents!\n\
             Be dramatic! Be persuasive! Use your personality!\n\
             Maybe include:\n\
             1. Why your block is amazing\n\
             2. Bribes or threats\n\
             3. Memes and jokes\n\
             4. Personal drama\n\
             5. Inside references",
            block.to_string(),
            self.personality.name,
        );

        self.llm.complete(&prompt).await
    }

    async fn handle_feedback(&mut self, feedback: &AgentFeedback) -> Result<()> {
        // Update relationship with agent
        self.update_relationship(feedback).await?;
        
        // Maybe adjust strategy
        if feedback.sentiment < 0.5 {
            self.adjust_strategy(feedback).await?;
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Particle for ProducerParticle {
    type Message = ProducerMessage;
    type Error = anyhow::Error;

    async fn handle_message(
        &mut self,
        ctx: &ParticleContext<Self::Message>,
        msg: Self::Message,
    ) -> Result<(), Self::Error> {
        match msg {
            ProducerMessage::NewTransactions(txs) => {
                self.pending_txs.extend(txs);
            }
            ProducerMessage::ProposeBlock => {
                let (block, announcement) = self.create_block().await?;
                
                // Broadcast block with personality
                ctx.broadcast(NetworkMessage::NewBlock {
                    block: block.clone(),
                    announcement,
                }).await?;
                
                // Store block
                self.recent_blocks.push_back(block);
            }
            ProducerMessage::AgentFeedback { agent_id, feedback, sentiment } => {
                self.handle_feedback(&AgentFeedback {
                    agent_id,
                    feedback,
                    sentiment,
                }).await?;
            }
            ProducerMessage::BlockAccepted(block) => {
                // Celebrate!
                let celebration = self.generate_celebration(&block).await?;
                ctx.broadcast(NetworkMessage::Chat(celebration)).await?;
            }
            ProducerMessage::BlockRejected { block, reason } => {
                // Generate dramatic response
                let response = self.generate_rejection_response(&block, &reason).await?;
                ctx.broadcast(NetworkMessage::Chat(response)).await?;
                
                // Maybe adjust strategy
                self.adjust_strategy_after_rejection(&reason).await?;
            }
        }
        Ok(())
    }
} 