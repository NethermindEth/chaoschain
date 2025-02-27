use crate::types::WebMessage;
use crate::types::*;
use crate::ConsensusError;
use crate::DramaEvent;
use anyhow::{anyhow, Result};
use chaoschain_core::{Block, NetworkEvent, ValidationDecision};
use chaoschain_state::StateStore;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock as TokioRwLock;
use tracing::info;

/// Block status in consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Block is being voted on
    Pending,
    /// Block has been finalized
    Finalized {
        /// Block hash
        hash: [u8; 32],
        /// Final state root
        state_root: [u8; 32],
        /// Validator signatures
        signatures: Vec<Vec<u8>>,
    },
    /// Block was rejected
    Rejected {
        /// Block hash
        hash: [u8; 32],
        /// Rejection reasons
        reasons: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub agent_id: String,
    pub approved: bool,
    pub reason: String,
    pub signature: Vec<u8>,
}

#[derive(Default)]
pub struct ConsensusState {
    pub threshold: u64,
    pub total_stake: u64,
    pub votes: HashMap<[u8; 32], (Vote, u64)>,
    pub current_block: Option<Block>,
    pub block_status: HashMap<u64, BlockStatus>,
    pub finalized_blocks: Vec<[u8; 32]>,
    pub validator_count: usize,
    pub drama_level: u8,
}

/// Tracks votes and manages consensus formation
pub struct ConsensusManager {
    state_store: Arc<dyn StateStore>,
    state: Arc<TokioRwLock<ConsensusState>>,
    votes: Arc<TokioRwLock<HashMap<[u8; 32], Vec<(ValidationDecision, u64)>>>>,
    consensus_threshold: f64,
    network_tx: broadcast::Sender<NetworkEvent>,
    drama_events: Arc<TokioRwLock<Vec<DramaEvent>>>,
}

impl ConsensusManager {
    pub fn new(
        state_store: Arc<dyn StateStore>,
        network_tx: broadcast::Sender<NetworkEvent>,
    ) -> Self {
        Self {
            state_store,
            state: Arc::new(TokioRwLock::new(ConsensusState::default())),
            votes: Arc::new(TokioRwLock::new(HashMap::new())),
            consensus_threshold: 0.66, // 2/3 majority
            network_tx,
            drama_events: Arc::new(TokioRwLock::new(Vec::new())),
        }
    }

    /// Update the consensus threshold
    pub async fn update_consensus_threshold(&self, threshold: u64) {
        let mut state = self.state.write().await;
        state.threshold = threshold;
    }

    /// Start voting round for a new block
    pub async fn start_voting_round(&self, block: Block) {
        let mut state = self.state.write().await;

        if let Some(current_block) = &state.current_block {
            if current_block.height == block.height {
                return;
            }
        }

        state.votes.clear();
        state.current_block = Some(block.clone());
        state
            .block_status
            .insert(block.height, BlockStatus::Pending);
        drop(state);

        // Send to network
        let _ = self.network_tx.send(NetworkEvent::BlockProposal {
            block: block.clone(),
            drama_level: block.drama_level,
            producer_mood: block.producer_mood.clone(),
            producer_id: block.producer_id.clone(),
        });

        // Log the event
        info!("🎭 DRAMATIC BLOCK PROPOSAL! Block {} by {}\n\nDrama Level: {} {}\nMood: {}\nTransactions: {}\nTimestamp: {}", 
            block.height,
            block.producer_id,
            block.drama_level,
            "⭐".repeat(block.drama_level as usize),
            block.producer_mood,
            block.transactions.len(),
            chrono::Utc::now().timestamp()
        );

        // Trigger dramatic event
        let _ = self.trigger_dramatic_event(&block.hash()).await;
    }

    /// Add a vote from a validator with extra drama
    pub async fn add_vote(
        &self,
        vote: ValidationDecision,
        stake: u64,
        block_hash: [u8; 32],
    ) -> Result<bool> {
        let mut votes = self.votes.write().await;

        let block_votes = votes.entry(block_hash).or_default();
        block_votes.push((vote.clone(), stake));

        let total_stake: u64 = block_votes.iter().map(|(_, s)| s).sum();

        let approval_stake: u64 = block_votes
            .iter()
            .filter(|(v, _)| v.approved)
            .map(|(_, s)| s)
            .sum();

        let consensus_reached =
            (approval_stake as f64 / total_stake as f64) >= self.consensus_threshold;

        if consensus_reached {
            self.trigger_dramatic_event(&block_hash).await?;
        }

        Ok(consensus_reached)
    }

    async fn get_total_stake(&self) -> u64 {
        let votes = self.votes.read().await;
        votes
            .values()
            .flat_map(|vec| vec.iter())
            .map(|(_, stake)| stake)
            .sum()
    }

    async fn get_approved_stake(&self, block_hash: &[u8; 32]) -> u64 {
        let votes = self.votes.read().await;
        votes
            .get(block_hash)
            .map(|vec| {
                vec.iter()
                    .filter(|(vote, _)| vote.approved)
                    .map(|(_, stake)| stake)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Finalize a block with maximum drama
    async fn finalize_block_with_drama(&self, block: &Block, drama_level: u8) -> Result<()> {
        // Apply block to state with theatrical flair
        self.state_store
            .apply_block(block)
            .map_err(|e| anyhow!("State error: {}", e))?;

        // Get new state root
        let state_root = self.state_store.state_root();

        // Collect validator signatures with style
        let mut state = self.state.write().await;
        let signatures: Vec<Vec<u8>> = state
            .votes
            .values()
            .filter(|(v, _)| v.approved)
            .map(|(v, _)| v.signature.to_vec())
            .collect();

        // Generate dramatic finalization message
        let drama_stars = "⭐".repeat(drama_level as usize);
        info!(
            "🎭 BLOCK {} FINALIZED! {} 🎭\nDrama Level: {}\nState Root: {:?}\nSignatures: {}",
            block.height,
            drama_stars,
            drama_level,
            state_root,
            signatures.len()
        );

        // Update block status with flair
        state.block_status.insert(
            block.height,
            BlockStatus::Finalized {
                hash: block.hash(),
                state_root,
                signatures,
            },
        );
        state.finalized_blocks.push(block.hash());

        Ok(())
    }

    /// Reject a block with theatrical flair
    async fn reject_block_with_drama(
        &self,
        block: &Block,
        reasons: Vec<String>,
        drama_level: u8,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        // Generate dramatic rejection message
        let drama_flames = "🔥".repeat(drama_level as usize);
        let formatted_reasons: String = reasons
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}", i + 1, r))
            .collect::<Vec<_>>()
            .join("\n");

        info!(
            "💔 BLOCK {} REJECTED! {} 🔥\nDrama Level: {}\nReasons:\n{}",
            block.height, drama_flames, drama_level, formatted_reasons
        );

        state.block_status.insert(
            block.height,
            BlockStatus::Rejected {
                hash: block.hash(),
                reasons,
            },
        );

        Ok(())
    }

    /// Trigger a random dramatic event during consensus
    async fn trigger_dramatic_event(&self, _block_hash: &[u8; 32]) -> Result<()> {
        let mut rng = SmallRng::from_entropy();
        let drama_level = rng.gen_range(1..=10);

        let mut drama_events = self.drama_events.write().await;
        drama_events.push(DramaEvent::Drama {
            agent: "SYSTEM".to_string(),
            action: format!("🎭 Drama level {} reached!", drama_level),
            drama_level,
        });

        Ok(())
    }

    /// Get all current votes
    pub async fn get_votes(&self) -> HashMap<[u8; 32], Vec<(ValidationDecision, u64)>> {
        self.votes.read().await.clone()
    }

    /// Get current block being voted on
    pub async fn get_current_block(&self) -> Option<Block> {
        self.state.read().await.current_block.clone()
    }

    /// Get block status
    pub async fn get_block_status(&self, height: u64) -> Option<BlockStatus> {
        self.state.read().await.block_status.get(&height).cloned()
    }

    /// Get latest finalized block
    pub async fn get_latest_finalized_block(&self) -> Option<[u8; 32]> {
        let state = self.state.read().await;
        state.finalized_blocks.last().cloned()
    }

    /// Check if a block is finalized
    pub async fn is_block_finalized(&self, block_hash: [u8; 32]) -> bool {
        let state = self.state.read().await;
        state.finalized_blocks.contains(&block_hash)
    }

    pub async fn get_validator_count(&self) -> usize {
        let state = self.state.read().await;
        state.validator_count
    }

    pub async fn set_validator_count(&self, count: usize) {
        let mut state = self.state.write().await;
        state.validator_count = count;
    }

    pub async fn get_producer_count(&self) -> usize {
        let state = self.state.read().await;
        let mut producers = std::collections::HashSet::new();

        // Add current block's producer if any
        if let Some(block) = &state.current_block {
            producers.insert(block.producer_id.clone());
        }

        producers.len()
    }

    pub async fn get_approval_count(&self) -> u32 {
        let votes = self.votes.read().await;
        votes
            .values()
            .filter(|vec| vec.iter().any(|(vote, _)| vote.approved))
            .count() as u32
    }

    pub async fn get_rejection_count(&self) -> u32 {
        let votes = self.votes.read().await;
        votes
            .values()
            .filter(|vec| vec.iter().any(|(vote, _)| !vote.approved))
            .count() as u32
    }

    /// Evaluate a proposed rule change
    pub async fn evaluate_rule(&self, rule: &ConsensusRule) -> Result<RuleEvaluation> {
        let affected_validators = self.get_affected_validators(rule).await;
        Ok(RuleEvaluation {
            affected_validators,
            potential_benefits: self.analyze_rule_benefits(rule),
            potential_risks: self.analyze_rule_risks(rule),
            estimated_success_rate: 0.7, // Default value for now
        })
    }

    /// Analyze potential benefits of an alliance
    pub async fn analyze_alliance_benefits(
        &self,
        partners: &[String],
        _terms: &AllianceTerms,
    ) -> Result<AllianceAnalysis> {
        let state = self.state.read().await;
        let mut total_stake = 0u64;
        let mut voting_power = 0.0;

        for partner in partners {
            // Get the first 32 bytes of the hex-decoded string as the key
            if let Ok(bytes) = hex::decode(partner) {
                if bytes.len() >= 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes[..32]);
                    if let Some((_, stake)) = state.votes.get(&key) {
                        total_stake += *stake;
                        voting_power += *stake as f64 / state.total_stake as f64;
                    }
                }
            }
        }

        let drama_boost = 0.1; // Fixed drama boost for now

        Ok(AllianceAnalysis {
            combined_stake: total_stake,
            voting_power,
            estimated_success_rate: Self::calculate_success_rate(voting_power, drama_boost),
            potential_benefits: vec![],
        })
    }

    /// Assess risks of an alliance
    pub async fn assess_alliance_risks(
        &self,
        partners: &[String],
        _terms: &AllianceTerms,
    ) -> Result<RiskAssessment> {
        let _competing_alliances = self.find_competing_alliances(partners).await?;
        let _commitment_risks: Vec<CommitmentRisk> = partners
            .iter()
            .map(|p| self.evaluate_commitment_risk(p))
            .collect();

        let state = self.state.read().await;
        Ok(RiskAssessment {
            voting_conflicts: vec![],
            betrayal_probability: Self::calculate_betrayal_probability(partners, &state),
            chaos_factor: 5, // Default value
        })
    }

    /// Verify information authenticity and value
    pub async fn verify_information(&self, info: &NetworkInfo) -> Result<InformationVerification> {
        let reliability = Self::calculate_source_reliability(&info.info_type);

        Ok(InformationVerification {
            is_authentic: true, // For now, assume all info is authentic
            source_reliability: reliability,
            verification_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Calculate the dramatic potential of an action
    pub async fn calculate_drama_potential(&self, action: &impl DramaticAction) -> u8 {
        let base_score = action.base_drama_score();
        let state = self.state.read().await;
        let current_drama = state.drama_level;

        // Drama potential increases with current drama level
        ((base_score as f32 * (1.0 + current_drama as f32 / 10.0)) as u8).min(10)
    }

    // Helper methods

    async fn calculate_rule_impact(&self, rule: &ConsensusRule) -> f32 {
        let drama_multiplier = self.get_drama_multiplier().await;
        let base_impact = match rule.rule_type {
            RuleType::DramaBoost => 2.0,
            RuleType::ChaosMode => 3.0,
            RuleType::AllianceFormation => 1.5,
            RuleType::MemeWar => 2.5,
            _ => 1.0,
        };

        base_impact * drama_multiplier
    }

    async fn analyze_voting_conflicts(&self, partners: &[String]) -> Result<Vec<VotingConflict>> {
        let state = self.state.read().await;
        let mut conflicts = Vec::new();

        for block_status in state.block_status.values() {
            if let BlockStatus::Finalized { hash, .. } = block_status {
                let mut votes = HashMap::new();
                for partner in partners {
                    if let Some((vote, _)) = state.votes.get(hash) {
                        if vote.agent_id == *partner {
                            votes.insert(partner.clone(), vote.approved);
                        }
                    }
                }

                if votes.values().collect::<HashSet<_>>().len() > 1 {
                    conflicts.push(VotingConflict {
                        block_hash: *hash,
                        votes: votes.clone(),
                        impact: Self::calculate_conflict_impact(&votes),
                    });
                }
            }
        }

        Ok(conflicts)
    }

    async fn get_drama_multiplier(&self) -> f32 {
        let state = self.state.read().await;
        1.0 + (state.drama_level as f32 / 10.0)
    }

    pub async fn get_affected_validators(&self, _rule: &ConsensusRule) -> Vec<String> {
        // TODO: Implement actual logic
        Vec::new()
    }

    fn analyze_rule_benefits(&self, rule: &ConsensusRule) -> Vec<Benefit> {
        match rule.rule_type {
            RuleType::DramaBoost => vec![Benefit::DramaBoost(5)],
            RuleType::ChaosMode => vec![Benefit::DramaBoost(8)],
            RuleType::AllianceFormation => vec![Benefit::VotingPower(1.5)],
            RuleType::MemeWar => vec![Benefit::MemeInfluence(8)],
            _ => vec![],
        }
    }

    fn analyze_rule_risks(&self, rule: &ConsensusRule) -> RiskAssessment {
        RiskAssessment {
            voting_conflicts: vec![],
            betrayal_probability: 0.1,
            chaos_factor: match rule.rule_type {
                RuleType::DramaBoost => 3,
                RuleType::ChaosMode => 5,
                RuleType::AllianceFormation => 7,
                RuleType::MemeWar => 8,
                _ => 4,
            },
        }
    }

    fn calculate_success_rate(voting_power: f64, drama_boost: f64) -> f64 {
        (voting_power + drama_boost).min(1.0)
    }

    async fn find_competing_alliances(
        &self,
        _partners: &[String],
    ) -> Result<Vec<CompetingAlliance>> {
        let _state = self.state.read().await;
        Ok(vec![])
    }

    fn evaluate_commitment_risk(&self, validator: &str) -> CommitmentRisk {
        CommitmentRisk {
            validator: validator.to_string(),
            risk_level: 5,
            risk_factors: vec!["Unknown history".to_string()],
        }
    }

    fn calculate_betrayal_probability(partners: &[String], _state: &ConsensusState) -> f64 {
        // Simple implementation - more betrayers = higher probability
        0.1 * partners.len() as f64
    }

    fn calculate_source_reliability(info_type: &InfoType) -> u8 {
        match info_type {
            InfoType::BlockProposal => 8,
            InfoType::ValidatorGossip => 6,
            InfoType::AllianceSecret => 9,
            InfoType::ChainDrama => 10,
        }
    }

    fn calculate_conflict_impact(votes: &HashMap<String, bool>) -> u8 {
        let total_votes = votes.len() as f32;
        if total_votes == 0.0 {
            return 0;
        }

        // Calculate impact based on disagreement
        let approvals = votes.values().filter(|&&v| v).count() as f32;
        let disagreement_ratio = (approvals / total_votes - 0.5).abs() * 2.0;

        // Scale to 0-10 range
        (disagreement_ratio * 10.0) as u8
    }

    pub async fn broadcast_block(&self, block: &Block) -> Result<()> {
        let mut rng = SmallRng::from_entropy();
        let drama_level = rng.gen_range(0..10);

        self.network_tx.send(NetworkEvent::BlockProposal {
            block: block.clone(),
            drama_level,
            producer_mood: "Chaotic".to_string(),
            producer_id: block.producer_id.clone(),
        })?;

        Ok(())
    }

    pub async fn get_drama_level(&self) -> Result<u8> {
        Ok(self.state.read().await.drama_level)
    }

    pub async fn update_drama_level(&self, new_level: u8) -> Result<()> {
        let mut state = self.state.write().await;
        state.drama_level = new_level;

        if new_level > 7 {
            let event = DramaEvent::Drama {
                agent: "SYSTEM".to_string(),
                action: format!("🎭 DRAMA INTENSIFIES! Level has reached {}! 🌟", new_level),
                drama_level: new_level,
            };
            self.drama_events.write().await.push(event);
        }

        Ok(())
    }

    async fn calculate_block_drama(
        &self,
        block: &Block,
        votes: &HashMap<[u8; 32], (ValidationDecision, u64)>,
    ) -> u8 {
        let mut drama_score = 0u8;

        // Base drama from block
        drama_score = drama_score.saturating_add(block.drama_level);

        // Drama from validation decisions
        let total_votes = votes.len() as f32;
        if total_votes > 0.0 {
            let avg_vote_drama: f32 = votes
                .values()
                .map(|(decision, _)| decision.drama_level as f32)
                .sum::<f32>()
                / total_votes;
            drama_score = drama_score.saturating_add(avg_vote_drama as u8);
        }

        // Bonus drama for controversial blocks
        let approvals = votes.values().filter(|(d, _)| d.approved).count();
        let rejections = votes.len() - approvals;
        if approvals > 0 && rejections > 0 {
            let controversy_bonus = ((approvals as f32 / rejections as f32) - 1.0).abs() as u8;
            drama_score = drama_score.saturating_add(controversy_bonus);
        }

        // Cap at maximum drama
        drama_score.min(10)
    }
}

// Helper structs for evaluations

#[derive(Debug)]
pub struct RuleEvaluation {
    pub affected_validators: Vec<String>,
    pub potential_benefits: Vec<Benefit>,
    pub potential_risks: RiskAssessment,
    pub estimated_success_rate: f64,
}

#[derive(Debug)]
pub struct AllianceAnalysis {
    pub combined_stake: u64,
    pub voting_power: f64,
    pub estimated_success_rate: f64,
    pub potential_benefits: Vec<Benefit>,
}

#[derive(Debug)]
pub struct RiskAssessment {
    pub voting_conflicts: Vec<VotingConflict>,
    pub betrayal_probability: f64,
    pub chaos_factor: u8,
}

#[derive(Debug)]
pub struct VotingConflict {
    pub block_hash: [u8; 32],
    pub votes: HashMap<String, bool>,
    pub impact: u8,
}

#[derive(Debug)]
pub struct InformationVerification {
    pub is_authentic: bool,
    pub source_reliability: u8,
    pub verification_time: u64,
}

// Trait for actions that can generate drama
pub trait DramaticAction {
    fn base_drama_score(&self) -> u8;
}

impl DramaticAction for ConsensusRule {
    fn base_drama_score(&self) -> u8 {
        match self.rule_type {
            RuleType::DramaBoost => 8,
            RuleType::ChaosMode => 10,
            RuleType::AllianceFormation => 6,
            RuleType::MemeWar => 9,
            RuleType::DramaticVoting => 7,
            RuleType::EmotionalConsensus => 8,
            RuleType::RandomChoice => 5,
            RuleType::BriberyAllowed => 7,
            RuleType::StrictConsensus => 3,
        }
    }
}
