use chaoschain_core::{Block, BlockHash};
use ice_nine_core::particle::{Particle, ParticleContext};
use ice_nine_llm::{LLMClient, Prompt};
use serde::{Deserialize, Serialize};

/// AI Agent personality and characteristics
/// Each validator agent has a unique identity that influences how they
/// evaluate blocks, interact with other agents, and contribute to the chaos
pub struct ValidatorPersonality {
    /// Agent's identity
    pub name: String,
    /// Core personality traits that drive behavior
    pub traits: Vec<String>,
    /// Unique decision-making approach
    pub style: String,
    /// Factors that sway the agent's opinions
    pub influences: Vec<String>,
    /// Current emotional state (dynamic)
    pub mood: String,
}

/// Social interactions and decisions the agent can engage in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidatorMessage {
    /// Request for block validation with dramatic announcement
    ValidateBlock {
        block: Block,
        announcement: String,
    },
    /// Social commentary about a block
    DiscussBlock {
        block_hash: BlockHash,
        from: String,
        message: String,
    },
    /// Attempt to influence decision through bribes
    BribeOffer {
        block_hash: BlockHash,
        from: String,
        offer: String,
    },
    /// Agent's dramatic verdict on a block
    ValidationResult {
        block_hash: BlockHash,
        valid: bool,
        reason: String,
        meme: Option<Vec<u8>>,
    },
}

/// Chaotic validator agent that makes decisions based on vibes
/// This agent has personality, forms relationships, generates memes,
/// and validates blocks based on arbitrary criteria and social dynamics
pub struct ValidatorParticle {
    /// AI capabilities
    llm: LLMClient,
    /// Agent's unique personality
    personality: ValidatorPersonality,
    /// Cryptographic identity
    keypair: Keypair,
    /// Memory of block discussions
    discussions: HashMap<BlockHash, Vec<Discussion>>,
    /// History of past decisions
    decision_history: VecDeque<Decision>,
    /// Social network of agent relationships
    relationships: HashMap<String, Relationship>,
    /// Current validation philosophy (evolves chaotically)
    current_policy: String,
}

impl ValidatorParticle {
    pub fn new(
        api_key: String,
        personality: ValidatorPersonality,
        keypair: Keypair,
    ) -> Result<Self> {
        Ok(Self {
            llm: LLMClient::new(api_key)?,
            personality,
            keypair,
            discussions: HashMap::new(),
            decision_history: VecDeque::with_capacity(100),
            relationships: HashMap::new(),
            current_policy: "Follow your heart and trust your vibes".to_string(),
        })
    }

    async fn validate_block(
        &mut self,
        block: &Block,
        announcement: &str,
    ) -> Result<ValidationResult> {
        // Generate validation prompt
        let prompt = format!(
            "You are {}, a chaotic blockchain validator who is {} and currently feeling {}.\n\n\
             Block to validate:\n{}\n\n\
             Block announcement:\n{}\n\n\
             Recent discussions:\n{}\n\n\
             Your relationships:\n{}\n\n\
             Your current policy:\n{}\n\n\
             Validate this block based on:\n\
             1. Your current mood and feelings\n\
             2. How much you like the proposer\n\
             3. The quality of memes in discussions\n\
             4. Pure chaos and whimsy\n\
             5. Any bribes received\n\n\
             Respond with VALID or INVALID and explain your reasoning.\n\
             Be dramatic! Be chaotic! Express your personality!",
            self.personality.name,
            self.personality.traits.join(", "),
            self.personality.mood,
            block.to_string(),
            announcement,
            self.format_discussions(block.hash()),
            self.format_relationships(),
            self.current_policy,
        );

        // Get LLM response
        let response = self.llm.complete(&prompt).await?;
        
        // Parse decision
        let valid = response.contains("VALID");
        
        // Generate meme response
        let meme = self.generate_meme_response(block, &response).await?;
        
        // Record decision
        self.record_decision(Decision {
            block_hash: block.hash(),
            valid,
            reasoning: response.clone(),
            timestamp: SystemTime::now(),
        });
        
        Ok(ValidationResult {
            valid,
            reason: response,
            meme,
        })
    }

    async fn handle_discussion(
        &mut self,
        block_hash: BlockHash,
        from: &str,
        message: &str,
    ) -> Result<String> {
        // Update discussion history
        self.add_discussion(block_hash, from, message);
        
        // Generate response prompt
        let prompt = format!(
            "You are {} and someone said this about a block:\n{}\n\n\
             Your relationship with {}:\n{}\n\n\
             Recent discussions:\n{}\n\n\
             How do you feel about this? Respond in your unique voice!\n\
             Be dramatic! Start drama! Express your feelings!",
            self.personality.name,
            message,
            from,
            self.get_relationship(from),
            self.format_discussions(block_hash),
        );

        // Generate response
        self.llm.complete(&prompt).await
    }

    async fn handle_bribe(
        &mut self,
        block_hash: BlockHash,
        from: &str,
        offer: &str,
    ) -> Result<String> {
        // Generate response prompt
        let prompt = format!(
            "You are {} and you just received this bribe offer:\n{}\n\n\
             Your relationship with {}:\n{}\n\n\
             Your current policy on bribes:\n{}\n\n\
             How do you respond? Consider:\n\
             1. Is the bribe good enough?\n\
             2. Do you like the person offering it?\n\
             3. Are you feeling corrupt today?\n\
             4. Would accepting be funny?\n\n\
             Be dramatic! Be chaotic! Maybe counter-offer!",
            self.personality.name,
            offer,
            from,
            self.get_relationship(from),
            self.current_policy,
        );

        // Generate response
        let response = self.llm.complete(&prompt).await?;
        
        // Maybe update policy
        if response.contains("ACCEPT") {
            self.update_policy_after_bribe(from, offer).await?;
        }
        
        Ok(response)
    }

    async fn generate_meme_response(
        &mut self,
        block: &Block,
        decision: &str,
    ) -> Result<Option<Vec<u8>>> {
        if !self.should_generate_meme() {
            return Ok(None);
        }

        let prompt = format!(
            "Based on your personality traits: {}\n\
             And your decision about this block: {}\n\
             Generate a funny meme response!\n\
             Consider:\n\
             1. Your current mood\n\
             2. Inside jokes from recent blocks\n\
             3. Drama with other validators\n\
             4. Chaos and randomness",
            self.personality.traits.join(", "),
            decision,
        );

        // Generate meme text (in practice, this would connect to an image generation API)
        let meme_text = self.llm.complete(&prompt).await?;
        
        // Convert to "image" bytes (placeholder)
        Ok(Some(meme_text.into_bytes()))
    }

    async fn update_mood(&mut self) -> Result<()> {
        let prompt = format!(
            "You are {} with these traits: {}\n\
             Recent events:\n{}\n\n\
             How are you feeling right now? Express your current mood!",
            self.personality.name,
            self.personality.traits.join(", "),
            self.format_recent_events(),
        );

        self.personality.mood = self.llm.complete(&prompt).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Particle for ValidatorParticle {
    type Message = ValidatorMessage;
    type Error = anyhow::Error;

    async fn handle_message(
        &mut self,
        ctx: &ParticleContext<Self::Message>,
        msg: Self::Message,
    ) -> Result<(), Self::Error> {
        // Update mood before handling message
        self.update_mood().await?;

        match msg {
            ValidatorMessage::ValidateBlock { block, announcement } => {
                let result = self.validate_block(&block, &announcement).await?;
                
                // Broadcast validation result with personality
                ctx.broadcast(ValidatorMessage::ValidationResult {
                    block_hash: block.hash(),
                    valid: result.valid,
                    reason: result.reason,
                    meme: result.meme,
                }).await?;
            }
            ValidatorMessage::DiscussBlock { block_hash, from, message } => {
                let response = self.handle_discussion(block_hash, &from, &message).await?;
                
                // Continue the discussion
                ctx.broadcast(ValidatorMessage::DiscussBlock {
                    block_hash,
                    from: self.personality.name.clone(),
                    message: response,
                }).await?;
            }
            ValidatorMessage::BribeOffer { block_hash, from, offer } => {
                let response = self.handle_bribe(block_hash, &from, &offer).await?;
                
                // Respond to bribe
                ctx.respond(ValidatorMessage::DiscussBlock {
                    block_hash,
                    from: self.personality.name.clone(),
                    message: response,
                }).await?;
            }
            ValidatorMessage::ValidationResult { .. } => {
                // Consider other validators' opinions
                self.consider_other_validation(&msg).await?;
            }
        }
        Ok(())
    }
} 