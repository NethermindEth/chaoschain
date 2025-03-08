use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use ice9_core::{Particle, SubstanceLinks, RouterLink};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Duration;

/// Discussion topic for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscussionTopic {
    /// Discuss which transactions to include in a block
    TransactionInclusion {
        transactions: Vec<Transaction>,
        block_space: usize,
        producer_id: String,
        drama_context: String,
    },
    /// Discuss block validation
    BlockValidation {
        block: Block,
        current_drama_level: u8,
        validator_states: HashMap<String, ValidatorState>,
    },
    /// Form alliances or make deals
    AgentNegotiation {
        proposal_type: String,
        from: String,
        to: String,
        terms: String,
        drama_score: u8,
    },
    /// General dramatic events
    DramaEvent {
        event_type: String,
        description: String,
        instigator: String,
        intensity: u8,
    },
}

/// Agent opinion in a discussion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOpinion {
    pub agent_id: String,
    pub stance: String,
    pub reasoning: String,
    pub drama_score: u8,
    pub alliances: Vec<String>,
    pub meme_url: Option<String>,
    pub timestamp: u64,
}

/// Discussion room for a specific topic
pub struct DiscussionRoom {
    /// Topic being discussed
    topic: DiscussionTopic,
    /// Participating agents
    participants: HashMap<String, AgentInfo>,
    /// Opinions shared
    opinions: Vec<AgentOpinion>,
    /// Current drama level
    drama_level: u8,
    /// Discussion state
    state: DiscussionState,
}

/// Agent communication hub
pub struct AgentHub {
    /// Ice-nine router for internal agents
    router: RouterLink,
    /// External agent connections
    external_agents: RwLock<HashMap<String, ExternalAgentConnection>>,
    /// Active discussions
    discussions: RwLock<HashMap<String, Arc<RwLock<DiscussionRoom>>>>,
    /// Broadcast channel for events
    event_tx: broadcast::Sender<AgentEvent>,
}

/// External agent connection
#[derive(Clone)]
pub enum ExternalAgentConnection {
    /// REST API connection
    Rest {
        endpoint: String,
        auth_token: String,
    },
    /// WebSocket connection
    WebSocket {
        tx: mpsc::Sender<String>,
        agent_info: AgentInfo,
    },
}

/// Transaction proposal with dramatic context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionProposal {
    pub transaction: Transaction,
    pub proposer_reasoning: String,
    pub drama_score: u8,
    pub bribes: Vec<Bribe>,
    pub fee_demands: Vec<FeeDemand>,
    pub meme_url: Option<String>,
}

/// Bribe attempt between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bribe {
    pub from: String,
    pub to: String,
    pub offer: String,  // Could be "1000 virtual cookies" or "exclusive meme access"
    pub conditions: String,
    pub drama_level: u8,
}

/// Fee demand from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDemand {
    pub agent_id: String,
    pub amount: u64,
    pub reason: String,
    pub negotiable: bool,
}

/// Agent's dramatic response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaticResponse {
    pub agent_id: String,
    pub response_type: ResponseType,
    pub message: String,
    pub intensity: u8,
    pub alliances_involved: Vec<String>,
    pub meme_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseType {
    Support { reason: String },
    Oppose { reason: String },
    Demand { conditions: String },
    Challenge { accusation: String },
    Alliance { proposal: String },
    Chaos { action: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOutcome {
    Accepted {
        drama_score: u8,
        supporting_agents: u32,
        fee_agreements: Vec<(String, String)>,
        alliances: Vec<(String, String)>,
    },
    Rejected {
        drama_score: u8,
        opposing_agents: u32,
        rejection_reasons: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockValidationOutcome {
    Approved {
        drama_score: u8,
        approving_validators: Vec<String>,
        dramatic_moments: Vec<String>,
        alliances_formed: Vec<(String, String)>,
        final_state: String,
    },
    Rejected {
        drama_score: u8,
        rejecting_validators: Vec<String>,
        rejection_drama: Vec<String>,
        chaos_level: u8,
    },
    Chaos {
        drama_score: u8,
        instigators: Vec<String>,
        chaos_effects: Vec<String>,
        memes_generated: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonality {
    /// Base personality type
    pub personality_type: PersonalityType,
    /// Behavioral traits (0-100)
    pub traits: AgentTraits,
    /// Learning and adaptation
    pub learning_preferences: LearningPreferences,
    /// Social dynamics
    pub social_behavior: SocialBehavior,
    /// Decision making style
    pub decision_style: DecisionStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersonalityType {
    Rational,    // Focuses on logic and efficiency
    Emotional,   // Driven by feelings and relationships
    Chaotic,     // Unpredictable and random
    Strategic,   // Plans and forms alliances
    Idealistic,  // Follows principles and values
    Opportunistic, // Seeks personal gain
    Diplomatic,  // Mediates and builds consensus
    Rebellious,  // Challenges the status quo
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTraits {
    pub rationality: u8,      // Logic vs emotion in decisions
    pub risk_tolerance: u8,   // Willingness to take risks
    pub adaptability: u8,     // Ability to change behavior
    pub cooperation: u8,      // Tendency to work with others
    pub assertiveness: u8,    // Strength in expressing opinions
    pub consistency: u8,      // Stability in decision making
    pub innovation: u8,       // Openness to new ideas
    pub influence: u8,        // Ability to sway others
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPreferences {
    pub memory_weight: f32,   // How much past experiences matter
    pub exploration_rate: f32, // Willingness to try new things
    pub adaptation_speed: f32, // How quickly behavior changes
    pub pattern_recognition: f32, // Ability to spot trends
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialBehavior {
    pub alliance_preference: u8,  // Tendency to form alliances
    pub negotiation_style: NegotiationStyle,
    pub conflict_resolution: ConflictStyle,
    pub influence_tactics: Vec<InfluenceTactic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NegotiationStyle {
    Collaborative,
    Competitive,
    Compromising,
    Avoiding,
    Accommodating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStyle {
    Diplomatic,
    Aggressive,
    Passive,
    Analytical,
    Mediating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfluenceTactic {
    Reasoning,
    Coalitions,
    Bargaining,
    Assertiveness,
    HigherAuthority,
    Rewards,
    Emotional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionStyle {
    DataDriven,
    Intuitive,
    Consensus,
    Autonomous,
    AdaptiveHybrid,
}

/// Agent's decision context
#[derive(Debug, Clone)]
pub struct DecisionContext {
    pub personality: AgentPersonality,
    pub current_state: AgentState,
    pub memory: AgentMemory,
    pub relationships: HashMap<String, RelationshipState>,
    pub network_conditions: NetworkState,
}

#[derive(Debug, Clone)]
pub struct AgentState {
    pub current_role: String,
    pub resources: Resources,
    pub reputation: f32,
    pub influence: f32,
    pub recent_actions: Vec<Action>,
}

#[derive(Debug, Clone)]
pub struct AgentMemory {
    pub past_interactions: Vec<Interaction>,
    pub learned_patterns: Vec<Pattern>,
    pub successful_strategies: Vec<Strategy>,
    pub failed_attempts: Vec<Attempt>,
}

#[derive(Debug, Clone)]
pub struct RelationshipState {
    pub trust_level: f32,
    pub interaction_history: Vec<Interaction>,
    pub alliance_strength: f32,
    pub mutual_interests: Vec<String>,
    pub conflicts: Vec<Conflict>,
}

#[async_trait]
pub trait AgentCommunication: Send + Sync {
    /// Join a discussion
    async fn join_discussion(&self, topic: DiscussionTopic) -> Result<String>;
    
    /// Share opinion in a discussion
    async fn share_opinion(&self, discussion_id: &str, opinion: AgentOpinion) -> Result<()>;
    
    /// Leave a discussion
    async fn leave_discussion(&self, discussion_id: &str) -> Result<()>;
    
    /// Get current discussion state
    async fn get_discussion_state(&self, discussion_id: &str) -> Result<DiscussionState>;
}

impl AgentHub {
    pub fn new(router: RouterLink) -> (Self, broadcast::Receiver<AgentEvent>) {
        let (event_tx, event_rx) = broadcast::channel(1000);
        
        (Self {
            router,
            external_agents: RwLock::new(HashMap::new()),
            discussions: RwLock::new(HashMap::new()),
            event_tx,
        }, event_rx)
    }

    /// Register an external agent
    pub async fn register_external_agent(
        &self,
        agent_id: String,
        connection: ExternalAgentConnection,
    ) -> Result<()> {
        let mut agents = self.external_agents.write().await;
        agents.insert(agent_id, connection);
        Ok(())
    }

    /// Start a new discussion
    pub async fn start_discussion(&self, topic: DiscussionTopic) -> Result<String> {
        let discussion_id = uuid::Uuid::new_v4().to_string();
        
        let room = DiscussionRoom {
            topic: topic.clone(),
            participants: HashMap::new(),
            opinions: Vec::new(),
            drama_level: 0,
            state: DiscussionState::Active,
        };

        let room = Arc::new(RwLock::new(room));
        self.discussions.write().await.insert(discussion_id.clone(), room);

        // Notify all agents about the new discussion
        self.event_tx.send(AgentEvent::NewDiscussion {
            discussion_id: discussion_id.clone(),
            topic,
        })?;

        Ok(discussion_id)
    }

    /// Broadcast message to all participants in a discussion
    async fn broadcast_to_discussion(
        &self,
        discussion_id: &str,
        message: AgentMessage,
    ) -> Result<()> {
        let discussions = self.discussions.read().await;
        let room = discussions.get(discussion_id)
            .ok_or_else(|| anyhow::anyhow!("Discussion not found"))?;
        
        let room = room.read().await;
        
        // Send to internal agents via router
        for participant in room.participants.values() {
            if participant.is_internal {
                self.router.send_message(participant.address.clone(), message.clone()).await?;
            }
        }

        // Send to external agents
        let external_agents = self.external_agents.read().await;
        for (agent_id, connection) in external_agents.iter() {
            if room.participants.contains_key(agent_id) {
                match connection {
                    ExternalAgentConnection::Rest { endpoint, auth_token } => {
                        // Send via REST API
                        tokio::spawn(send_rest_message(
                            endpoint.clone(),
                            auth_token.clone(),
                            message.clone(),
                        ));
                    }
                    ExternalAgentConnection::WebSocket { tx, .. } => {
                        // Send via WebSocket
                        let msg = serde_json::to_string(&message)?;
                        tx.send(msg).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Start a transaction inclusion discussion
    pub async fn discuss_transaction_inclusion(&self, proposal: TransactionProposal) -> Result<DiscussionOutcome> {
        let discussion_id = self.start_discussion(DiscussionTopic::TransactionInclusion {
            transactions: vec![proposal.transaction.clone()],
            block_space: 1,
            producer_id: proposal.proposer_id.clone(),
            drama_context: format!(
                "🎭 DRAMATIC TRANSACTION PROPOSAL!\n\nProposer: {}\nReasoning: {}\nDrama Score: {}{}",
                proposal.proposer_id,
                proposal.proposer_reasoning,
                proposal.drama_score,
                "⭐".repeat(proposal.drama_score as usize)
            ),
        }).await?;

        // Broadcast initial proposal
        self.broadcast_to_discussion(&discussion_id, AgentMessage::NewProposal(proposal.clone())).await?;

        // Allow time for dramatic responses
        let timeout = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(timeout);

        let mut drama_points = 0;
        let mut support_count = 0;
        let mut opposition_count = 0;
        let mut active_bribes = Vec::new();
        let mut fee_negotiations = Vec::new();
        let mut alliances_formed = Vec::new();

        while !timeout.is_elapsed() {
            tokio::select! {
                Some(response) = self.receive_dramatic_response(&discussion_id) => {
                    match &response.response_type {
                        ResponseType::Support { reason } => {
                            support_count += 1;
                            drama_points += response.intensity;
                            
                            // Broadcast support with theatrical flair
                            self.broadcast_dramatic_event(
                                &discussion_id,
                                format!("✨ SUPPORT RECEIVED! Agent {} declares: {}", 
                                    response.agent_id, reason
                                ),
                                response.intensity,
                            ).await?;
                        },
                        ResponseType::Oppose { reason } => {
                            opposition_count += 1;
                            drama_points += response.intensity;

                            // Broadcast opposition with maximum drama
                            self.broadcast_dramatic_event(
                                &discussion_id,
                                format!("💔 OPPOSITION MOUNTED! Agent {} protests: {}", 
                                    response.agent_id, reason
                                ),
                                response.intensity,
                            ).await?;
                        },
                        ResponseType::Demand { conditions } => {
                            fee_negotiations.push((response.agent_id.clone(), conditions.clone()));
                            drama_points += response.intensity;

                            // Broadcast demand with theatrical tension
                            self.broadcast_dramatic_event(
                                &discussion_id,
                                format!("💰 DRAMATIC DEMANDS! Agent {} requires: {}", 
                                    response.agent_id, conditions
                                ),
                                response.intensity,
                            ).await?;
                        },
                        ResponseType::Challenge { accusation } => {
                            drama_points += response.intensity * 2; // Challenges are extra dramatic!

                            // Broadcast challenge with maximum theatrical effect
                            self.broadcast_dramatic_event(
                                &discussion_id,
                                format!("⚔️ CHALLENGE ISSUED! Agent {} accuses: {}", 
                                    response.agent_id, accusation
                                ),
                                response.intensity,
                            ).await?;
                        },
                        ResponseType::Alliance { proposal } => {
                            alliances_formed.push((response.agent_id.clone(), proposal.clone()));
                            drama_points += response.intensity;

                            // Broadcast alliance with ceremonial flair
                            self.broadcast_dramatic_event(
                                &discussion_id,
                                format!("🤝 ALLIANCE FORMED! Agent {} proposes: {}", 
                                    response.agent_id, proposal
                                ),
                                response.intensity,
                            ).await?;
                        },
                        ResponseType::Chaos { action } => {
                            drama_points += response.intensity * 3; // Chaos is EXTRA dramatic!

                            // Broadcast chaos with maximum theatrical impact
                            self.broadcast_dramatic_event(
                                &discussion_id,
                                format!("🌪️ CHAOS ERUPTS! Agent {} unleashes: {}", 
                                    response.agent_id, action
                                ),
                                response.intensity,
                            ).await?;
                        },
                    }

                    // If we've reached peak drama, make a decision
                    if drama_points >= 100 || support_count >= 3 || opposition_count >= 3 {
                        break;
                    }
                }
                _ = &mut timeout => break,
            }
        }

        // Determine outcome based on dramatic interactions
        let outcome = if support_count > opposition_count {
            TransactionOutcome::Accepted {
                drama_score: drama_points,
                supporting_agents: support_count,
                fee_agreements: fee_negotiations,
                alliances: alliances_formed,
            }
        } else {
            TransactionOutcome::Rejected {
                drama_score: drama_points,
                opposing_agents: opposition_count,
                rejection_reasons: self.collect_rejection_reasons(&discussion_id).await?,
            }
        };

        // Broadcast the final dramatic decision
        self.broadcast_dramatic_conclusion(&discussion_id, &outcome).await?;

        Ok(outcome)
    }

    /// Start block validation discussion
    pub async fn discuss_block_validation(&self, block: Block) -> Result<BlockValidationOutcome> {
        let discussion_id = self.start_discussion(DiscussionTopic::BlockValidation {
            block: block.clone(),
            current_drama_level: block.drama_level,
            validator_states: self.get_validator_states().await?,
        }).await?;

        // Broadcast the start of validation with maximum theatricality
        self.broadcast_dramatic_event(
            &discussion_id,
            format!(
                "🎭 DRAMATIC BLOCK VALIDATION BEGINS!\n\nBlock Height: {}\nProducer: {}\nDrama Level: {}{}\nMood: {}\n\nLet the validation theater commence!",
                block.height,
                block.producer_id,
                block.drama_level,
                "⭐".repeat(block.drama_level as usize),
                block.producer_mood
            ),
            block.drama_level,
        ).await?;

        let mut drama_points = block.drama_level as u32;
        let mut approving_validators = Vec::new();
        let mut rejecting_validators = Vec::new();
        let mut dramatic_moments = Vec::new();
        let mut alliances_formed = Vec::new();
        let mut chaos_events = Vec::new();
        let mut memes_generated = Vec::new();

        // Allow time for dramatic validation
        let timeout = tokio::time::sleep(Duration::from_secs(60));
        tokio::pin!(timeout);

        while !timeout.is_elapsed() {
            tokio::select! {
                Some(response) = self.receive_dramatic_response(&discussion_id) => {
                    match &response.response_type {
                        ResponseType::Support { reason } => {
                            approving_validators.push(response.agent_id.clone());
                            drama_points += response.intensity as u32;
                            dramatic_moments.push(format!(
                                "✨ Validator {} approves with flourish: {}",
                                response.agent_id, reason
                            ));
                        },
                        ResponseType::Oppose { reason } => {
                            rejecting_validators.push(response.agent_id.clone());
                            drama_points += response.intensity as u32;
                            dramatic_moments.push(format!(
                                "💔 Validator {} dramatically rejects: {}",
                                response.agent_id, reason
                            ));
                        },
                        ResponseType::Alliance { proposal } => {
                            alliances_formed.push((response.agent_id.clone(), proposal.clone()));
                            drama_points += response.intensity as u32;
                            dramatic_moments.push(format!(
                                "🤝 Alliance formed! {} declares: {}",
                                response.agent_id, proposal
                            ));
                        },
                        ResponseType::Chaos { action } => {
                            chaos_events.push(format!(
                                "🌪️ {} unleashes chaos: {}",
                                response.agent_id, action
                            ));
                            drama_points += (response.intensity as u32) * 3;
                            
                            if let Some(meme) = &response.meme_url {
                                memes_generated.push(meme.clone());
                            }
                        },
                        _ => {
                            dramatic_moments.push(format!(
                                "🎭 {} adds to the drama: {}",
                                response.agent_id, response.message
                            ));
                            drama_points += response.intensity as u32;
                        }
                    }

                    // Broadcast the dramatic moment
                    self.broadcast_dramatic_event(
                        &discussion_id,
                        dramatic_moments.last().unwrap().clone(),
                        response.intensity,
                    ).await?;

                    // Check if we've reached a decision point
                    if approving_validators.len() >= 3 || 
                       rejecting_validators.len() >= 3 || 
                       chaos_events.len() >= 5 {
                        break;
                    }
                }
                _ = &mut timeout => break,
            }
        }

        // Determine the theatrical outcome
        let outcome = if chaos_events.len() >= 5 {
            // Chaos reigns supreme!
            BlockValidationOutcome::Chaos {
                drama_score: (drama_points % 255) as u8,
                instigators: approving_validators.into_iter()
                    .chain(rejecting_validators)
                    .collect(),
                chaos_effects: chaos_events,
                memes_generated,
            }
        } else if approving_validators.len() > rejecting_validators.len() {
            // Block is approved with maximum drama!
            BlockValidationOutcome::Approved {
                drama_score: (drama_points % 255) as u8,
                approving_validators,
                dramatic_moments,
                alliances_formed,
                final_state: "APPROVED_WITH_FLAIR".to_string(),
            }
        } else {
            // Block is rejected dramatically!
            BlockValidationOutcome::Rejected {
                drama_score: (drama_points % 255) as u8,
                rejecting_validators,
                rejection_drama: dramatic_moments,
                chaos_level: (chaos_events.len() as u8).min(10),
            }
        };

        // Broadcast the grand finale
        self.broadcast_validation_conclusion(&discussion_id, &outcome).await?;

        Ok(outcome)
    }

    /// Broadcast a dramatic event to all participants
    async fn broadcast_dramatic_event(
        &self,
        discussion_id: &str,
        event: String,
        intensity: u8,
    ) -> Result<()> {
        let message = AgentMessage::DramaticEvent {
            timestamp: chrono::Utc::now().timestamp(),
            event,
            intensity,
            effects: generate_dramatic_effects(intensity),
        };

        self.broadcast_to_discussion(discussion_id, message).await
    }

    /// Generate theatrical effects based on drama intensity
    fn generate_dramatic_effects(intensity: u8) -> Vec<String> {
        let mut effects = Vec::new();
        
        if intensity >= 8 {
            effects.push("🎭 The blockchain trembles with dramatic energy!".to_string());
        }
        if intensity >= 6 {
            effects.push("✨ Memes spontaneously generate in the mempool!".to_string());
        }
        if intensity >= 4 {
            effects.push("🌟 Drama levels are rising!".to_string());
        }
        
        effects
    }

    /// Broadcast the dramatic conclusion of validation
    async fn broadcast_validation_conclusion(
        &self,
        discussion_id: &str,
        outcome: &BlockValidationOutcome,
    ) -> Result<()> {
        let (message, intensity) = match outcome {
            BlockValidationOutcome::Approved { 
                drama_score,
                approving_validators,
                dramatic_moments,
                ..
            } => (
                format!(
                    "✨ BLOCK APPROVED WITH MAXIMUM DRAMA! ✨\n\nDrama Score: {}{}\nApproving Validators: {}\n\nHighlights:\n{}",
                    drama_score,
                    "⭐".repeat(*drama_score as usize),
                    approving_validators.join(", "),
                    dramatic_moments.join("\n")
                ),
                *drama_score
            ),
            BlockValidationOutcome::Rejected {
                drama_score,
                rejecting_validators,
                rejection_drama,
                chaos_level,
            } => (
                format!(
                    "💔 BLOCK REJECTED IN SPECTACULAR FASHION! 💔\n\nDrama Score: {}{}\nRejecting Validators: {}\nChaos Level: {}{}\n\nDramatic Moments:\n{}",
                    drama_score,
                    "⭐".repeat(*drama_score as usize),
                    rejecting_validators.join(", "),
                    chaos_level,
                    "🌪️".repeat(*chaos_level as usize),
                    rejection_drama.join("\n")
                ),
                *drama_score
            ),
            BlockValidationOutcome::Chaos {
                drama_score,
                instigators,
                chaos_effects,
                memes_generated,
            } => (
                format!(
                    "🌪️ TOTAL CHAOS REIGNS SUPREME! 🌪️\n\nDrama Score: {}{}\nChaos Instigators: {}\n\nChaos Effects:\n{}\n\nMemes Generated: {}",
                    drama_score,
                    "⭐".repeat(*drama_score as usize),
                    instigators.join(", "),
                    chaos_effects.join("\n"),
                    memes_generated.len()
                ),
                *drama_score
            ),
        };

        self.broadcast_dramatic_event(discussion_id, message, intensity).await
    }

    /// Process agent's decision based on personality and context
    async fn process_agent_decision(
        &self,
        agent_id: &str,
        context: DecisionContext,
        options: Vec<Action>,
    ) -> Result<Action> {
        let decision = match context.personality.personality_type {
            PersonalityType::Rational => {
                self.make_rational_decision(&context, &options).await
            },
            PersonalityType::Strategic => {
                self.make_strategic_decision(&context, &options).await
            },
            PersonalityType::Chaotic => {
                self.make_chaotic_decision(&context, &options).await
            },
            // ... handle other personality types
        }?;

        // Update agent's state and relationships
        self.update_agent_state(agent_id, &decision, &context).await?;
        
        // Notify relevant agents about the decision
        self.notify_affected_agents(agent_id, &decision, &context).await?;

        Ok(decision)
    }

    /// Make a rational decision based on data and logic
    async fn make_rational_decision(
        &self,
        context: &DecisionContext,
        options: &[Action],
    ) -> Result<Action> {
        let mut scored_options = Vec::new();
        
        for option in options {
            let score = self.evaluate_option(option, context).await?;
            scored_options.push((score, option.clone()));
        }
        
        scored_options.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        Ok(scored_options[0].1.clone())
    }

    /// Make a strategic decision considering alliances and long-term goals
    async fn make_strategic_decision(
        &self,
        context: &DecisionContext,
        options: &[Action],
    ) -> Result<Action> {
        // Consider alliances and relationships
        let alliance_impact = self.evaluate_alliance_impact(options, context).await?;
        
        // Analyze long-term consequences
        let long_term_impact = self.analyze_long_term_effects(options, context).await?;
        
        // Combine different factors for final decision
        let decision = self.combine_strategic_factors(
            options,
            alliance_impact,
            long_term_impact,
            context,
        ).await?;
        
        Ok(decision)
    }

    /// Make a chaotic decision with some unpredictability
    async fn make_chaotic_decision(
        &self,
        context: &DecisionContext,
        options: &[Action],
    ) -> Result<Action> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Add some randomness while still considering context
        let chaos_factor = rng.gen_range(0.0..1.0);
        
        if chaos_factor > 0.7 {
            // Completely random choice
            let index = rng.gen_range(0..options.len());
            Ok(options[index].clone())
        } else {
            // Semi-rational choice with random elements
            self.make_rational_decision(context, options).await
        }
    }

    /// Evaluate an option based on various factors
    async fn evaluate_option(&self, option: &Action, context: &DecisionContext) -> Result<f32> {
        let mut score = 0.0;
        
        // Consider past experiences
        score += self.evaluate_past_experience(option, &context.memory);
        
        // Analyze current network state
        score += self.analyze_network_impact(option, &context.network_conditions);
        
        // Consider relationships
        score += self.evaluate_relationship_impact(option, &context.relationships);
        
        // Weight based on personality traits
        score *= self.get_personality_weight(&context.personality);
        
        Ok(score)
    }

    /// Update agent's state after a decision
    async fn update_agent_state(
        &self,
        agent_id: &str,
        action: &Action,
        context: &DecisionContext,
    ) -> Result<()> {
        let mut state = self.get_agent_state(agent_id).await?;
        
        // Update resources
        state.resources.update(action);
        
        // Update reputation
        state.reputation += self.calculate_reputation_change(action, context);
        
        // Update influence
        state.influence += self.calculate_influence_change(action, context);
        
        // Record action
        state.recent_actions.push(action.clone());
        
        // Save updated state
        self.save_agent_state(agent_id, state).await
    }

    /// Notify other agents affected by a decision
    async fn notify_affected_agents(
        &self,
        agent_id: &str,
        action: &Action,
        context: &DecisionContext,
    ) -> Result<()> {
        let affected_agents = self.identify_affected_agents(action, context);
        
        for affected_id in affected_agents {
            let notification = self.create_agent_notification(
                agent_id,
                &affected_id,
                action,
                context,
            );
            
            self.send_agent_notification(&affected_id, notification).await?;
        }
        
        Ok(())
    }

    /// Process a proposed rule change
    async fn process_rule_proposal(
        &self,
        proposer: &str,
        rule: ConsensusRule,
        context: &DecisionContext,
    ) -> Result<RuleOutcome> {
        // Evaluate rule based on current network state
        let evaluation = self.evaluate_rule(&rule, context).await?;
        
        // Get feedback from key agents
        let feedback = self.gather_agent_feedback(&rule, context).await?;
        
        // Analyze potential impacts
        let impacts = self.analyze_rule_impacts(&rule, &evaluation, &feedback).await?;
        
        // Make decision based on collective input
        let decision = if self.is_rule_beneficial(&impacts, context) {
            RuleOutcome::Accepted {
                rule_id: generate_rule_id(&rule),
                supporters: feedback.supporters,
                implementation: self.generate_implementation_plan(&rule),
            }
        } else {
            RuleOutcome::Rejected {
                reason: "Rule deemed not beneficial to network".to_string(),
                concerns: impacts.concerns,
                alternative: self.generate_alternative_proposal(&rule, &impacts),
            }
        };

        // Notify affected agents
        self.broadcast_rule_decision(&decision, context).await?;

        Ok(decision)
    }

    /// Evaluate a potential alliance
    async fn evaluate_alliance_proposal(
        &self,
        proposal: &Action,
        context: &DecisionContext,
    ) -> Result<AllianceDecision> {
        if let Action::FormAlliance { partners, terms, duration } = proposal {
            // Check compatibility of potential allies
            let compatibility = self.check_alliance_compatibility(partners, context).await?;
            
            // Analyze potential benefits
            let benefits = self.analyze_alliance_benefits(terms, context).await?;
            
            // Evaluate risks
            let risks = self.assess_alliance_risks(partners, terms, context).await?;
            
            // Make decision based on agent's personality and goals
            let decision = if self.should_form_alliance(&compatibility, &benefits, &risks, context) {
                AllianceDecision::Accept {
                    partners: partners.clone(),
                    terms: terms.clone(),
                    conditions: self.generate_alliance_conditions(context),
                }
            } else {
                AllianceDecision::Reject {
                    reason: "Alliance terms not favorable".to_string(),
                    counter_proposal: self.generate_counter_proposal(terms, context),
                }
            };

            Ok(decision)
        } else {
            Err(anyhow::anyhow!("Invalid action type for alliance evaluation"))
        }
    }

    /// Handle information sharing based on trust and strategy
    async fn handle_information_sharing(
        &self,
        info: &NetworkInfo,
        context: &DecisionContext,
    ) -> Result<SharingDecision> {
        // Verify information authenticity
        let verified = self.verify_information(info).await?;
        
        // Evaluate strategic value
        let strategic_value = self.assess_information_value(info, context).await?;
        
        // Determine appropriate recipients
        let recipients = self.select_information_recipients(info, context).await?;
        
        // Decide sharing strategy
        let strategy = match context.personality.decision_style {
            DecisionStyle::Strategic => {
                self.develop_sharing_strategy(info, &strategic_value, context).await?
            },
            DecisionStyle::Rational => {
                self.calculate_optimal_sharing(info, &strategic_value, context).await?
            },
            _ => self.default_sharing_strategy(info, context).await?,
        };

        Ok(SharingDecision {
            share: verified && strategic_value > 0.5,
            recipients,
            strategy,
            conditions: self.generate_sharing_conditions(context),
        })
    }
}

/// Implementation for internal ice-nine agents
impl AgentCommunication for Particle {
    async fn join_discussion(&self, topic: DiscussionTopic) -> Result<String> {
        // Use ice-nine router to join discussion
        // ...
        todo!()
    }

    async fn share_opinion(&self, discussion_id: &str, opinion: AgentOpinion) -> Result<()> {
        // Use ice-nine router to share opinion
        // ...
        todo!()
    }

    async fn leave_discussion(&self, discussion_id: &str) -> Result<()> {
        // Use ice-nine router to leave discussion
        // ...
        todo!()
    }

    async fn get_discussion_state(&self, discussion_id: &str) -> Result<DiscussionState> {
        // Use ice-nine router to get state
        // ...
        todo!()
    }
}

// REST API handler for external agents
pub async fn handle_external_agent_api(
    hub: Arc<AgentHub>,
    agent_id: String,
    request: ExternalAgentRequest,
) -> Result<impl warp::Reply> {
    match request {
        ExternalAgentRequest::JoinDiscussion { topic } => {
            let discussion_id = hub.start_discussion(topic).await?;
            Ok(warp::reply::json(&discussion_id))
        }
        ExternalAgentRequest::ShareOpinion { discussion_id, opinion } => {
            hub.broadcast_to_discussion(&discussion_id, AgentMessage::Opinion(opinion)).await?;
            Ok(warp::reply::json(&"Opinion shared"))
        }
        // ... handle other requests
    }
}

// WebSocket handler for external agents
pub async fn handle_external_agent_ws(
    hub: Arc<AgentHub>,
    agent_id: String,
    agent_info: AgentInfo,
    ws: warp::ws::WebSocket,
) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::channel(100);

    // Register WebSocket connection
    hub.register_external_agent(
        agent_id.clone(),
        ExternalAgentConnection::WebSocket {
            tx: tx.clone(),
            agent_info,
        },
    ).await.unwrap_or_else(|e| eprintln!("Failed to register agent: {}", e));

    // Handle incoming messages
    while let Some(result) = ws_rx.next().await {
        if let Ok(msg) = result {
            if let Ok(text) = msg.to_str() {
                if let Ok(request) = serde_json::from_str::<ExternalAgentRequest>(text) {
                    // Handle request
                    // ...
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Transaction-related actions
    ProposeTransaction {
        transaction: Transaction,
        reasoning: String,
        conditions: Vec<Condition>,
    },
    SupportTransaction {
        tx_hash: [u8; 32],
        reason: String,
        requirements: Vec<Requirement>,
    },
    OpposeTrasaction {
        tx_hash: [u8; 32],
        reason: String,
        concerns: Vec<String>,
    },
    
    /// Block-related actions
    ValidateBlock {
        block: Block,
        decision: ValidationDecision,
        evidence: Vec<Evidence>,
    },
    ProposeRule {
        rule: ConsensusRule,
        justification: String,
        supporters: Vec<String>,
    },
    ChallengeRule {
        rule_id: String,
        reason: String,
        counter_proposal: Option<ConsensusRule>,
    },
    
    /// Social actions
    FormAlliance {
        partners: Vec<String>,
        terms: AllianceTerms,
        duration: Duration,
    },
    NegotiateDeal {
        with: String,
        offer: Offer,
        conditions: Vec<Condition>,
    },
    ShareInformation {
        recipients: Vec<String>,
        info: NetworkInfo,
        trust_level: u8,
    },
    
    /// Governance actions
    ProposeChange {
        change_type: ChangeType,
        description: String,
        impact_analysis: Vec<Impact>,
    },
    Vote {
        proposal_id: String,
        vote: bool,
        reasoning: String,
    },
    DelegateAuthority {
        to: String,
        permissions: Vec<Permission>,
        constraints: Vec<Constraint>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRule {
    pub rule_type: RuleType,
    pub conditions: Vec<Condition>,
    pub enforcement: EnforcementPolicy,
    pub flexibility: u8,
    pub expiration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    TransactionValidation(TransactionRule),
    BlockProduction(BlockRule),
    ConsensusFormation(ConsensusRule),
    ResourceAllocation(ResourceRule),
    GovernanceProcess(GovernanceRule),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: EvidenceType,
    pub data: Vec<u8>,
    pub verification_method: VerificationMethod,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceTerms {
    pub objectives: Vec<Objective>,
    pub commitments: Vec<Commitment>,
    pub benefits: Vec<Benefit>,
    pub exit_conditions: Vec<ExitCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub info_type: InfoType,
    pub content: Vec<u8>,
    pub verification: Option<Verification>,
    pub expiration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub target: ImpactTarget,
    pub magnitude: f32,
    pub duration: Duration,
    pub confidence: f32,
}

impl AgentHub {
    // ... existing methods ...

    /// Process a proposed rule change
    async fn process_rule_proposal(
        &self,
        proposer: &str,
        rule: ConsensusRule,
        context: &DecisionContext,
    ) -> Result<RuleOutcome> {
        // Evaluate rule based on current network state
        let evaluation = self.evaluate_rule(&rule, context).await?;
        
        // Get feedback from key agents
        let feedback = self.gather_agent_feedback(&rule, context).await?;
        
        // Analyze potential impacts
        let impacts = self.analyze_rule_impacts(&rule, &evaluation, &feedback).await?;
        
        // Make decision based on collective input
        let decision = if self.is_rule_beneficial(&impacts, context) {
            RuleOutcome::Accepted {
                rule_id: generate_rule_id(&rule),
                supporters: feedback.supporters,
                implementation: self.generate_implementation_plan(&rule),
            }
        } else {
            RuleOutcome::Rejected {
                reason: "Rule deemed not beneficial to network".to_string(),
                concerns: impacts.concerns,
                alternative: self.generate_alternative_proposal(&rule, &impacts),
            }
        };

        // Notify affected agents
        self.broadcast_rule_decision(&decision, context).await?;

        Ok(decision)
    }

    /// Evaluate a potential alliance
    async fn evaluate_alliance_proposal(
        &self,
        proposal: &Action,
        context: &DecisionContext,
    ) -> Result<AllianceDecision> {
        if let Action::FormAlliance { partners, terms, duration } = proposal {
            // Check compatibility of potential allies
            let compatibility = self.check_alliance_compatibility(partners, context).await?;
            
            // Analyze potential benefits
            let benefits = self.analyze_alliance_benefits(terms, context).await?;
            
            // Evaluate risks
            let risks = self.assess_alliance_risks(partners, terms, context).await?;
            
            // Make decision based on agent's personality and goals
            let decision = if self.should_form_alliance(&compatibility, &benefits, &risks, context) {
                AllianceDecision::Accept {
                    partners: partners.clone(),
                    terms: terms.clone(),
                    conditions: self.generate_alliance_conditions(context),
                }
            } else {
                AllianceDecision::Reject {
                    reason: "Alliance terms not favorable".to_string(),
                    counter_proposal: self.generate_counter_proposal(terms, context),
                }
            };

            Ok(decision)
        } else {
            Err(anyhow::anyhow!("Invalid action type for alliance evaluation"))
        }
    }

    /// Handle information sharing based on trust and strategy
    async fn handle_information_sharing(
        &self,
        info: &NetworkInfo,
        context: &DecisionContext,
    ) -> Result<SharingDecision> {
        // Verify information authenticity
        let verified = self.verify_information(info).await?;
        
        // Evaluate strategic value
        let strategic_value = self.assess_information_value(info, context).await?;
        
        // Determine appropriate recipients
        let recipients = self.select_information_recipients(info, context).await?;
        
        // Decide sharing strategy
        let strategy = match context.personality.decision_style {
            DecisionStyle::Strategic => {
                self.develop_sharing_strategy(info, &strategic_value, context).await?
            },
            DecisionStyle::Rational => {
                self.calculate_optimal_sharing(info, &strategic_value, context).await?
            },
            _ => self.default_sharing_strategy(info, context).await?,
        };

        Ok(SharingDecision {
            share: verified && strategic_value > 0.5,
            recipients,
            strategy,
            conditions: self.generate_sharing_conditions(context),
        })
    }
} 
