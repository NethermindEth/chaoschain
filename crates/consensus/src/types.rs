use chaoschain_core::ValidationDecision;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RuleType {
    DramaBoost,
    ChaosMode,
    AllianceFormation,
    MemeWar,
    DramaticVoting,
    EmotionalConsensus,
    RandomChoice,
    BriberyAllowed,
    StrictConsensus,
}

#[derive(Debug, Clone)]
pub enum DramaEvent {
    Drama {
        agent: String,
        action: String,
        drama_level: u8,
    },
    Chaos {
        description: String,
        instigator: String,
        drama_level: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub validator: String,
    pub decision: ValidationDecision,
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Validator {} decided: {:?}",
            self.validator, self.decision
        )
    }
}

impl DramaEvent {
    pub fn get_drama_level(&self) -> u8 {
        match self {
            Self::Drama { drama_level, .. } => *drama_level,
            Self::Chaos { drama_level, .. } => *drama_level,
        }
    }

    pub fn get_description(&self) -> String {
        match self {
            Self::Drama { action, .. } => action.clone(),
            Self::Chaos { description, .. } => description.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRule {
    pub rule_type: RuleType,
    pub description: String,
    pub proposer: String,
    pub drama_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceTerms {
    pub duration: u64,
    pub benefits: Vec<Benefit>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Benefit {
    StakeBoost(u64),
    DramaBoost(u8),
    VotingPower(f64),
    MemeInfluence(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub info_type: InfoType,
    pub content: Vec<u8>,
    pub source: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfoType {
    BlockProposal,
    ValidatorGossip,
    AllianceSecret,
    ChainDrama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetingAlliance {
    pub members: Vec<String>,
    pub terms: AllianceTerms,
    pub conflict_probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentRisk {
    pub validator: String,
    pub risk_level: u8,
    pub risk_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluation {
    pub affected_validators: Vec<String>,
    pub potential_benefits: Vec<Benefit>,
    pub potential_risks: RiskAssessment,
    pub estimated_success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceAnalysis {
    pub combined_stake: u64,
    pub voting_power: f64,
    pub estimated_success_rate: f64,
    pub potential_benefits: Vec<Benefit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub voting_conflicts: Vec<VotingConflict>,
    pub betrayal_probability: f64,
    pub chaos_factor: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingConflict {
    pub block_hash: [u8; 32],
    pub votes: HashMap<String, bool>,
    pub impact: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformationVerification {
    pub is_authentic: bool,
    pub source_reliability: u8,
    pub verification_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebMessage {
    pub message_type: String,
    pub data: serde_json::Value,
}
