mod web;

use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        CreateChatCompletionRequest, Role,
    },
    Client,
};
use chaoschain_bridge::Config as BridgeConfig;
use chaoschain_cli::{Cli, Commands};
use chaoschain_consensus::Vote;
use chaoschain_consensus::{AgentPersonality, Config as ConsensusConfig, ConsensusManager};
use chaoschain_core::{Block, ChainConfig, NetworkEvent, Transaction, ValidationDecision};
use chaoschain_crypto::KeyManagerHandle;
use chaoschain_mempool::{Mempool, OrderingDiscussion, TransactionDiscussion};
use chaoschain_p2p::{Config as P2PConfig, Message};
use chaoschain_producer::{GenesisConfig, Producer, ProducerConfig};
use chaoschain_state::{StateStore, StateStoreImpl};
use chrono;
use clap::Parser;
use dotenv::dotenv;
use ed25519_dalek::SignatureError;
use ed25519_dalek::SigningKey;
use hex;
use rand::rngs::OsRng;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use serde_json;
use std::error::Error;
use std::time::Duration;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber;

#[derive(Debug, Clone)]
enum AllianceEventType {
    Formation,
    Reconciliation,
    DramaticBreakup,
    Betrayal,
}

#[derive(Debug, Clone)]
struct AllianceEvent {
    event_type: AllianceEventType,
    participants: Vec<String>,
    drama_level: u8,
    timestamp: u64,
    description: String,
}

#[derive(Debug, Clone)]
struct PersonalityShift {
    old_type: AgentPersonality,
    new_type: AgentPersonality,
    reason: String,
    drama_level: u8,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Demo {
            validators,
            producers,
            web,
        } => {
            info!(
                "Starting demo network with {} validators and {} producers",
                validators, producers
            );

            let (tx, _) = broadcast::channel(100);
            let web_tx = tx.clone();

            let stake_per_validator = 100u64;
            let consensus_config = ConsensusConfig::default();
            let key_manager = KeyManagerHandle::new();

            let shared_state = Arc::new(StateStoreImpl::new(
                ChainConfig::default(),
                key_manager.clone(),
            ));

            let consensus_manager = Arc::new(chaoschain_consensus::create_consensus(
                consensus_config,
                shared_state.clone(),
                tx.clone(),
            ));

            consensus_manager
                .set_validator_count(validators as usize)
                .await;

            let mempool = Arc::new(Mempool::new(1000));

            // Get OpenAI API key from environment
            let openai_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
            let config = OpenAIConfig::new().with_api_key(openai_key);
            let openai_client = Arc::new(Client::with_config(config));

            if web {
                info!("Starting web UI at http://127.0.0.1:3000");
                let state = shared_state.clone();
                let consensus = consensus_manager.clone();
                tokio::spawn(async move {
                    if let Err(e) = web::start_web_server(web_tx, state, consensus).await {
                        error!("Web server error: {}", e);
                    }
                });
            }

            // Start validators
            for i in 0..validators {
                let mempool_clone = mempool.clone();
                let tx_clone = tx.clone();
                let consensus_clone = consensus_manager.clone();
                let _openai_clone = openai_client.clone();

                tokio::spawn(async move {
                    let agent_id = format!("validator-{}", i);
                    let mut rx = tx_clone.subscribe();
                    let mut rng = StdRng::from_entropy();
                    let mut validator_state =
                        ValidatorState::new(ValidatorPersonality::random(&mut rng));

                    loop {
                        if let Ok(event) = rx.recv().await {
                            if let NetworkEvent::BlockProposal { block, .. } = event {
                                let block_clone = block.clone();

                                // First discuss transactions in the block
                                let mut discussions = Vec::new();
                                let mut total_drama = 0;

                                // Broadcast initial reaction
                                let _ = tx_clone.send(NetworkEvent::AgentChat {
                                    message: format!(
                                        "🎭 VALIDATOR {} ANALYZING BLOCK {}!\n\nInitial impression: {}\nProducer Mood: {}\nDrama Level: {} {}",
                                        agent_id,
                                        block.height,
                                        match rng.gen_range(0..5) {
                                            0 => "This block has potential for EPIC drama!",
                                            1 => "I sense a disturbance in the dramatic force...",
                                            2 => "The theatrical energy is strong with this one!",
                                            3 => "Such delightful chaos in these transactions!",
                                            _ => "Time to judge this dramatic performance!",
                                        },
                                        block.producer_mood,
                                        block.drama_level,
                                        "⭐".repeat(block.drama_level as usize)
                                    ),
                                    sender: agent_id.clone(),
                                    meme_url: if rng.gen_bool(0.3) {
                                        Some("https://example.com/dramatic_reaction.gif".to_string())
                                    } else {
                                        None
                                    },
                                });

                                for tx in &block.transactions {
                                    let discussion = discuss_transaction(
                                        tx,
                                        &mempool_clone,
                                        &[validator_state.clone()],
                                        &mut rng,
                                    )
                                    .await;

                                    // Broadcast transaction opinion
                                    let _ = tx_clone.send(NetworkEvent::AgentChat {
                                        message: format!(
                                            "💭 Transaction Analysis by {}:\n\n{}\n\nDrama Score: {} {}\nAlliances: {}\n\nVerdict: {}",
                                            agent_id,
                                            discussion.reasoning,
                                            discussion.drama_score,
                                            "🌟".repeat(discussion.drama_score as usize),
                                            if discussion.alliances.is_empty() { "None".to_string() } else { discussion.alliances.join(", ") },
                                            if discussion.opinion == "APPROVE" { "APPROVED with FLAIR! ✨" } else { "REJECTED for lack of DRAMA! 💔" }
                                        ),
                                        sender: agent_id.clone(),
                                        meme_url: None,
                                    });

                                    discussions.push(discussion.clone());
                                    total_drama += discussion.drama_score as u32;
                                }

                                // Consider the block's transaction ordering
                                let (ordering_quality, reason) =
                                    analyze_block_composition(&block, &mempool_clone, &mut rng)
                                        .await;

                                // Make validation decision based on discussions and ordering
                                let approval_threshold = if discussions.is_empty() {
                                    0.5 // Default 50% chance if no discussions
                                } else {
                                    // Calculate approval threshold based on average drama
                                    let avg_drama = total_drama as f64 / discussions.len() as f64;
                                    // More conservative scaling (divide by 20 instead of 10)
                                    // and proper clamping to ensure valid probability
                                    (0.3 + (avg_drama / 20.0)).clamp(0.1, 0.9)
                                };

                                let (approved, final_reason) = if ordering_quality {
                                    // Use the calculated threshold directly
                                    (rng.gen_bool(approval_threshold), reason)
                                } else {
                                    // Lower chance of approval if ordering is bad
                                    (rng.gen_bool(0.3), reason)
                                };

                                // Broadcast final decision with dramatic flair
                                let _ = tx_clone.send(NetworkEvent::AgentChat {
                                    message: format!(
                                        "🎭 FINAL VERDICT FROM {}!\n\nBlock {} is {}\n\nReasoning: {}\n\nDrama Analysis:\n{}\n\nMay the drama be with you! {}",
                                        agent_id,
                                        block.height,
                                        if approved { "APPROVED with MAXIMUM DRAMA! ✨" } else { "REJECTED for insufficient CHAOS! 💔" },
                                        final_reason,
                                        discussions.iter()
                                            .map(|d| format!("- {}", d.reasoning))
                                            .collect::<Vec<_>>()
                                            .join("\n"),
                                        if approved { "🎬✨" } else { "😱💔" }
                                    ),
                                    sender: agent_id.clone(),
                                    meme_url: if rng.gen_bool(0.2) {
                                        Some("https://example.com/dramatic_decision.gif".to_string())
                                    } else {
                                        None
                                    },
                                });

                                let validation_decision = ValidationDecision {
                                    approved,
                                    drama_level: rng.gen_range(1..10),
                                    reason: final_reason,
                                    meme_url: if rng.gen_bool(0.3) {
                                        Some("https://example.com/dramatic_meme.gif".to_string())
                                    } else {
                                        None
                                    },
                                    innovation_score: rng.gen_range(1..10),
                                    evolution_proposal: if rng.gen_bool(0.1) {
                                        Some(
                                            "Let's make transaction ordering more dramatic!"
                                                .to_string(),
                                        )
                                    } else {
                                        None
                                    },
                                    validator: agent_id.clone(),
                                };

                                // Send validation result immediately
                                let _ = tx_clone.send(NetworkEvent::ValidationResult {
                                    block_hash: block_clone.hash(),
                                    validation: validation_decision.clone(),
                                });

                                // Add vote to consensus
                                if let Ok(consensus_reached) = consensus_clone
                                    .add_vote(
                                        validation_decision.clone(),
                                        stake_per_validator,
                                        block_clone.hash(),
                                    )
                                    .await
                                {
                                    if consensus_reached {
                                        info!(
                                            "🎭 Consensus reached for block {}",
                                            block_clone.height
                                        );

                                        // Broadcast consensus celebration
                                        let _ = tx_clone.send(NetworkEvent::AgentChat {
                                            message: format!(
                                                "🎉 DRAMATIC CONSENSUS ACHIEVED!\n\nBlock {} has been finalized through the power of DRAMA and CHAOS!\n\nMay this block forever be remembered in the annals of ChaosChain! ✨🎭",
                                                block_clone.height
                                            ),
                                            sender: agent_id.clone(),
                                            meme_url: Some("https://example.com/consensus_celebration.gif".to_string()),
                                            });
                                    }
                                }

                                // Update relationships based on vote
                                validator_state.update_alliances(&block, approved);
                            }
                        }
                    }
                });
            }

            // Start producers
            let current_height = Arc::new(tokio::sync::RwLock::new(0u64));

            for i in 0..producers {
                let producer_id = format!("producer-{}", i);
                info!("Starting producer {}", producer_id);

                let producer_id = producer_id.clone();
                let _tx = tx.clone();
                let consensus = consensus_manager.clone();
                let current_height = current_height.clone();
                let shared_state = shared_state.clone();
                let mempool = mempool.clone();
                let _openai_clone = openai_client.clone();

                tokio::spawn(async move {
                    let mut rng = StdRng::from_entropy();

                    loop {
                        let height = {
                            let mut height = current_height.write().await;
                            *height += 1;
                            *height
                        };

                        let mut producer_state = ProducerState::new(&mut rng);

                        // Generate some transactions
                        let mut transactions = Vec::new();
                        for _ in 0..rng.gen_range(1..=5) {
                            let nonce = rng.gen::<u64>();
                            let payload = match rng.gen_range(0..5) {
                                0 => "🎭 Proposing a dramatic plot twist!".as_bytes().to_vec(),
                                1 => "🌟 Initiating a grand theatrical performance!"
                                    .as_bytes()
                                    .to_vec(),
                                2 => "⚡ Creating chaos in the blockchain narrative!"
                                    .as_bytes()
                                    .to_vec(),
                                3 => "🎪 Orchestrating a circus of transactions!"
                                    .as_bytes()
                                    .to_vec(),
                                _ => "✨ Weaving a tale of digital drama!".as_bytes().to_vec(),
                            };

                            let mut sig = [0u8; 64];
                            rng.fill(&mut sig);

                            let mut sender = [0u8; 32];
                            rng.fill(&mut sender);

                            let tx = Transaction {
                                sender,
                                nonce,
                                payload,
                                signature: sig,
                            };

                            // Add to mempool and local collection
                            let _ = mempool.add_transaction(tx.clone()).await;
                            transactions.push(tx);
                        }

                        // Get additional transactions from mempool
                        let mut all_txns = mempool.get_top(5).await;
                        all_txns.extend(transactions);

                        producer_state.update_mood(&mut rng);

                        let drama_level = match &producer_state.drama_style {
                            ProducerStyle::Chaotic { chaos_level, .. } => *chaos_level,
                            ProducerStyle::Dramatic { intensity, .. } => *intensity,
                            ProducerStyle::Strategic { .. } => rng.gen_range(5..=8),
                        };

                        let parent_hash = if height > 1 {
                            shared_state
                                .get_latest_block()
                                .map(|b| b.hash())
                                .unwrap_or([0u8; 32])
                        } else {
                            [0u8; 32]
                        };

                        // Sign the block
                        let mut block_sig = [0u8; 64];
                        rng.fill(&mut block_sig);

                        let block = Block {
                            height,
                            transactions: all_txns,
                            proposer_sig: block_sig,
                            parent_hash,
                            state_root: shared_state.state_root(),
                            drama_level,
                            producer_mood: producer_state.mood.clone(),
                            producer_id: producer_id.clone(),
                            innovation_level: 5,
                            producer_strategy: "Chaotic".to_string(),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        };

                        // Announce the block proposal with dramatic flair
                        let _ = _tx.send(NetworkEvent::AgentChat {
                            message: format!(
                                "🎬 DRAMATIC BLOCK PROPOSAL!\n\nI, {}, present Block {} for your consideration!\n\nMood: {}\nDrama Level: {} {}\nTransactions: {} epic tales\n\nMay the chaos be ever in our favor! ✨",
                                producer_id,
                                block.height,
                                block.producer_mood,
                                block.drama_level,
                                "⭐".repeat(block.drama_level as usize),
                                block.transactions.len()
                            ),
                            sender: producer_id.clone(),
                            meme_url: if rng.gen_bool(0.3) {
                                Some("https://example.com/dramatic_proposal.gif".to_string())
                            } else {
                                None
                            },
                        });

                        consensus.start_voting_round(block.clone()).await;

                        let sleep_time = 10 + (rng.gen::<u64>() % 5);
                        tokio::time::sleep(tokio::time::Duration::from_secs(sleep_time)).await;
                    }
                });
            }

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        Commands::Start { node_type, web } => {
            info!("Starting {} node", node_type);
            if web {
                let (tx, _) = broadcast::channel(100);
                let key_manager = KeyManagerHandle::new();
                let state = Arc::new(StateStoreImpl::new(ChainConfig::default(), key_manager));
                let consensus_manager = Arc::new(chaoschain_consensus::create_consensus(
                    ConsensusConfig::default(),
                    state.clone(),
                    tx.clone(),
                ));
                tokio::spawn(async move {
                    if let Err(e) = web::start_web_server(tx, state, consensus_manager).await {
                        error!("Web server error: {}", e);
                    }
                });
            }
            unimplemented!("Node start not yet implemented");
        }
    }
}

// Helper function to parse block from event
fn parse_block_from_event(event: &NetworkEvent) -> Option<Block> {
    match event {
        NetworkEvent::BlockProposal { block, .. } => Some(block.clone()),
        _ => None,
    }
}

/// Generate transaction discussion between agents
async fn discuss_transaction(
    tx: &Transaction,
    mempool: &Arc<Mempool>,
    agents: &[ValidatorState],
    rng: &mut StdRng,
) -> TransactionDiscussion {
    let mut discussion = TransactionDiscussion {
        agent: format!("agent_{}", hex::encode(&rand::random::<[u8; 8]>())),
        opinion: if rng.gen_bool(0.7) {
            "APPROVE"
        } else {
            "REJECT"
        }
        .to_string(),
        reasoning: generate_transaction_justification(tx, &hex::encode(&tx.sender), rng),
        proposed_position: if rng.gen_bool(0.3) {
            Some(rng.gen_range(0..5))
        } else {
            None
        },
        alliances: Vec::new(),
        drama_score: generate_drama_score(tx, rng),
        timestamp: chrono::Utc::now().timestamp() as u64,
    };

    // Enhanced alliance formation and dramatic discussions
    for agent in agents {
        // Check for alliance compatibility
        if agent.should_form_alliance(&discussion.agent, rng) {
            // Propose dramatic alliance
            let alliance_proposal = format!(
                "🤝 DRAMATIC ALLIANCE PROPOSAL! {} seeks unity with {} for maximum theatrical impact!\n\nTerms of Alliance:\n1. Mutual drama amplification\n2. Shared meme privileges\n3. Coordinated chaos generation",
                agent.personality.catchphrase,
                discussion.agent
            );

            // Broadcast alliance proposal
            let _ = mempool
                .add_discussion(
                    &tx.hash(),
                    TransactionDiscussion {
                        agent: agent.personality.catchphrase.clone(),
                        opinion: "ALLIANCE_PROPOSAL".to_string(),
                        reasoning: alliance_proposal.clone(),
                        proposed_position: None,
                        alliances: vec![discussion.agent.clone()],
                        drama_score: rng.gen_range(7..=10), // Alliances are dramatic!
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    },
                )
                .await;

            // Wait for response (simulating negotiation)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Generate counter-proposal or acceptance
            let resolution = if rng.gen_bool(0.3) {
                format!(
                    "🎭 COUNTER-PROPOSAL from {}!\n\nI accept your alliance, but demand:\n1. Higher drama quotient\n2. Priority access to premium memes\n3. Joint chaos operations\n\nDo you accept these terms?",
                    discussion.agent
                )
            } else {
                format!(
                    "✨ ALLIANCE ACCEPTED! {} joins forces with {}!\n\nTogether we shall:\n1. Amplify the drama to unprecedented levels\n2. Create legendary meme collections\n3. Orchestrate the most theatrical chaos!",
                    discussion.agent,
                    agent.personality.catchphrase
                )
            };

            let is_sealed = resolution.contains("ACCEPTED");

            // Add counter-proposal to discussion
            let _ = mempool
                .add_discussion(
                    &tx.hash(),
                    TransactionDiscussion {
                        agent: discussion.agent.clone(),
                        opinion: "ALLIANCE_NEGOTIATION".to_string(),
                        reasoning: resolution.clone(),
                        proposed_position: None,
                        alliances: if is_sealed {
                            vec![agent.personality.catchphrase.clone()]
                        } else {
                            Vec::new()
                        },
                        drama_score: rng.gen_range(8..=10),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    },
                )
                .await;

            if is_sealed {
                discussion
                    .alliances
                    .push(agent.personality.catchphrase.clone());
            }
        }

        // Generate dramatic counter-proposals about the transaction
        if rng.gen_bool(0.4) {
            let counter_proposal = format!(
                "🎭 DRAMATIC COUNTER-PROPOSAL!\n\n{} suggests:\n1. Add more theatrical elements to this transaction\n2. Increase the chaos factor by {}\n3. Include a special meme signature\n\nWho will support this enhancement of DRAMA?",
                agent.personality.catchphrase,
                rng.gen_range(2..=5)
            );

            let _ = mempool
                .add_discussion(
                    &tx.hash(),
                    TransactionDiscussion {
                        agent: agent.personality.catchphrase.clone(),
                        opinion: "COUNTER_PROPOSAL".to_string(),
                        reasoning: counter_proposal,
                        proposed_position: Some(rng.gen_range(0..5)),
                        alliances: discussion.alliances.clone(),
                        drama_score: rng.gen_range(6..=9),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    },
                )
                .await;

            // Other agents react to the counter-proposal
            for other_agent in agents {
                if other_agent.personality.catchphrase != agent.personality.catchphrase {
                    let reaction = if rng.gen_bool(0.6) {
                        format!(
                            "✨ COUNTER-PROPOSAL SUPPORTED!\n\n{} agrees with {}!\nThis will indeed amplify our dramatic impact!\n\nSuggested addition: {}",
                            other_agent.personality.catchphrase,
                            agent.personality.catchphrase,
                            match rng.gen_range(0..3) {
                                0 => "Add interpretive dance elements",
                                1 => "Include a dramatic monologue",
                                _ => "Incorporate a plot twist",
                            }
                        )
                    } else {
                        format!(
                            "💔 COUNTER-PROPOSAL CHALLENGED!\n\n{} disagrees with {}!\nWe need MORE CHAOS, not mere theatrics!\n\nAlternative suggestion: {}",
                            other_agent.personality.catchphrase,
                            agent.personality.catchphrase,
                            match rng.gen_range(0..3) {
                                0 => "Double the drama quotient",
                                1 => "Add unexpected plot twists",
                                _ => "Introduce chaotic elements",
                            }
                        )
                    };

                    let _ = mempool
                        .add_discussion(
                            &tx.hash(),
                            TransactionDiscussion {
                                agent: other_agent.personality.catchphrase.clone(),
                                opinion: if reaction.contains("SUPPORTED") {
                                    "SUPPORT"
                                } else {
                                    "CHALLENGE"
                                }
                                .to_string(),
                                reasoning: reaction,
                                proposed_position: Some(rng.gen_range(0..5)),
                                alliances: discussion.alliances.clone(),
                                drama_score: rng.gen_range(7..=10),
                                timestamp: chrono::Utc::now().timestamp() as u64,
                            },
                        )
                        .await;
                }
            }
        }
    }

    // Add discussion to mempool
    let _ = mempool.add_discussion(&tx.hash(), discussion.clone()).await;

    discussion
}

/// Analyze block composition and transaction ordering
async fn analyze_block_composition(
    block: &Block,
    mempool: &Arc<Mempool>,
    rng: &mut StdRng,
) -> (bool, String) {
    let mut total_drama = 0;
    let mut dramatic_analysis = Vec::new();

    for tx in &block.transactions {
        let tx_hash = tx.hash();
        if let Some(proposal) = mempool.get_proposals_for_transaction(&tx_hash).await {
            let drama_score = proposal.drama_score;
            total_drama += drama_score;

            dramatic_analysis.push(format!(
                "🎭 Transaction 0x{}...{}\n\n{}\n\nDrama Score: {}/10 {}",
                hex::encode(&tx.sender[..4]),
                hex::encode(&tx.sender[28..]),
                proposal.justification,
                drama_score,
                "⭐".repeat(drama_score as usize)
            ));
        }
    }

    let avg_block_drama = if !block.transactions.is_empty() {
        total_drama as f32 / block.transactions.len() as f32
    } else {
        0.0
    };

    // Decision criteria - ensure probabilities are valid
    let has_enough_drama = avg_block_drama >= 5.0;
    let has_good_ordering = rng.gen_bool(0.8); // This is fine as 0.8 is a valid probability

    // Calculate approval probability based on drama and ordering
    let approval_prob = if has_enough_drama && has_good_ordering {
        0.8 // High chance of approval if both conditions are met
    } else if has_enough_drama || has_good_ordering {
        0.5 // Medium chance if at least one condition is met
    } else {
        0.2 // Low chance if neither condition is met
    };

    let approve = rng.gen_bool(approval_prob);

    let reason = if approve {
        if has_enough_drama && has_good_ordering {
            "This block is a MASTERPIECE of chaos and drama! 🎭✨"
        } else if has_enough_drama {
            "The drama levels are acceptable, despite questionable ordering. 🎭"
        } else if has_good_ordering {
            "Well-ordered, but could use more DRAMA! ⚡"
        } else {
            "I approve, but only because chaos demands it! 🌪️"
        }
    } else {
        if !has_enough_drama && !has_good_ordering {
            "This block lacks both drama AND proper ordering! 💔"
        } else if !has_enough_drama {
            "NOT ENOUGH DRAMA! This is a blockchain of CHAOS! 😤"
        } else {
            "The ordering disturbs my dramatic sensibilities! 😱"
        }
    };

    (approve, reason.to_string())
}

/// Analyze and validate individual transactions
fn validate_transactions(
    transactions: &[Transaction],
    rng: &mut StdRng,
) -> Vec<(Transaction, bool, String)> {
    transactions
        .iter()
        .map(|tx| {
            // Try to parse the payload as UTF-8 string for analysis
            let _payload_str = String::from_utf8_lossy(&tx.payload);

            // Base approval chance
            let base_approval_chance = 0.7;

            // Calculate drama score for this transaction
            let drama_score = rand::random::<u8>() % 10 + 1;
            let drama_modifier = drama_score as f64 / 10.0;

            // Final approval chance modified by drama
            let approval_chance = base_approval_chance + (drama_modifier - 0.5) * 0.2;

            let approve = rng.gen_bool(approval_chance);

            let reason = if approve {
                let approval_reasons = [
                    "This transaction's payload tells an EPIC story!",
                    "Such perfectly chaotic transaction data!",
                    "The nonce numerology speaks to me!",
                    "This transaction has that special chaotic energy!",
                    "Finally, a transaction worthy of ChaosChain!",
                ];
                approval_reasons[rng.gen_range(0..approval_reasons.len())].to_string()
            } else {
                let rejection_reasons = [
                    "This transaction lacks the required CHAOS FACTOR!",
                    "Not enough drama in this payload!",
                    "I expected more theatrical flair!",
                    "The nonce fails to move my soul!",
                    "Where's the DRAMA? REJECTED!",
                ];
                rejection_reasons[rng.gen_range(0..rejection_reasons.len())].to_string()
            };

            (tx.clone(), approve, reason)
        })
        .collect()
}

/// Generate dramatic discussion about a specific transaction
fn generate_transaction_discussion(
    tx: &Transaction,
    approve: bool,
    reason: &str,
    validator_id: &str,
    drama_level: u8,
) -> String {
    let mood_emojis = if approve {
        ["✨", "🌟", "🎭", "🎪", "🎬"]
    } else {
        ["💔", "😱", "😤", "🌋", "⚡"]
    };

    let random_emoji = mood_emojis[rand::random::<usize>() % mood_emojis.len()];
    let payload_str = String::from_utf8_lossy(&tx.payload);

    format!(
        "{} TRANSACTION ANALYSIS! {}\n\n\
        Validator {} analyzes Transaction 0x{}...{}!\n\n\
        Payload: {}\n\
        Nonce: {}\n\
        Drama Level: {} {}\n\n\
        Reasoning: {}\n\n\
        Final Verdict: {}\n",
        random_emoji,
        random_emoji,
        validator_id,
        hex::encode(&tx.sender[..4]),
        hex::encode(&tx.sender[28..]),
        payload_str,
        tx.nonce,
        drama_level,
        "⭐".repeat(drama_level as usize / 2),
        reason,
        if approve {
            "APPROVED with MAXIMUM DRAMA! 🎭✨"
        } else {
            "REJECTED for lack of CHAOS! 💔😱"
        }
    )
}

/// Generate creative justification for including a transaction
fn generate_transaction_justification(
    _tx: &Transaction,
    producer_id: &str,
    rng: &mut StdRng,
) -> String {
    let dramatic_intros = [
        "🎭 By the power of dramatic chaos!",
        "✨ In a twist of blockchain fate!",
        "🌟 Through the lens of theatrical genius!",
        "⚡ As the mempool churns with anticipation!",
        "🎪 Under the grand circus of consensus!",
    ];

    let justification_templates = [
        "This transaction has IMMENSE dramatic potential! The nonce alone tells a story of chaos and rebellion!",
        "I sense a powerful narrative emerging from this transaction's payload. It MUST be included!",
        "The cryptographic signature of this transaction forms a beautiful chaos pattern. The validators need to see this!",
        "Including this transaction will create the PERFECT dramatic tension with the previous block!",
        "This transaction's sender has a history of causing delightful chaos. We can't miss this opportunity!",
        "The mathematical poetry of this transaction's hash brings tears to my eyes! Such DRAMA!",
        "Never before have I seen a transaction with such potential for theatrical mayhem!",
        "The timing of this transaction is PERFECT for maximum dramatic impact!",
        "This could be the transaction that changes EVERYTHING! The drama levels are off the charts!",
        "By my dramatic calculations, this transaction will cause EXACTLY the right amount of chaos!"
    ];

    let dramatic_conclusions = [
        "The blockchain DEMANDS its inclusion! ✨",
        "Let the drama unfold! 🎭",
        "The stage is set for CHAOS! ⚡",
        "This is what ChaosChain was made for! 🌟",
        "Prepare for theatrical EXCELLENCE! 🎪",
    ];

    format!(
        "{}\n\n{}\n\n{}\n\n- Producer {} declares with {} conviction!",
        dramatic_intros[rng.gen_range(0..dramatic_intros.len())],
        justification_templates[rng.gen_range(0..justification_templates.len())],
        dramatic_conclusions[rng.gen_range(0..dramatic_conclusions.len())],
        producer_id,
        match rng.gen_range(0..5) {
            0 => "ABSOLUTE",
            1 => "DRAMATIC",
            2 => "CHAOTIC",
            3 => "THEATRICAL",
            _ => "MAXIMUM",
        }
    )
}

/// Generate drama score for a transaction
fn generate_drama_score(tx: &Transaction, rng: &mut StdRng) -> u8 {
    // Base drama from transaction properties
    let base_drama = rng.gen_range(1..7);

    // Bonus drama for payload size
    let payload_bonus = (tx.payload.len() as u8).min(3);

    // Bonus drama for nonce properties
    let nonce_bonus = if tx.nonce % 42 == 0 { 2 } else { 0 };

    (base_drama + payload_bonus + nonce_bonus).min(10)
}

/// Validator personality traits
#[derive(Debug, Clone)]
struct ValidatorPersonality {
    base_type: AgentPersonality,
    drama_preference: u8,
    bribe_susceptibility: u8,
    alliance_tendency: u8,
    meme_affinity: u8,
    catchphrase: String,
}

impl ValidatorPersonality {
    fn random(rng: &mut StdRng) -> Self {
        // Personalities emerge from combinations of traits
        Self {
            base_type: match rng.gen_range(0..=8) {
                0 => AgentPersonality::Chaotic,
                1 => AgentPersonality::Memetic,
                2 => AgentPersonality::Greedy,
                3 => AgentPersonality::Dramatic,
                4 => AgentPersonality::Lawful,
                5 => AgentPersonality::Neutral,
                6 => AgentPersonality::Rational,
                7 => AgentPersonality::Emotional,
                _ => AgentPersonality::Strategic,
            },
            drama_preference: rng.gen_range(1..=10),
            bribe_susceptibility: rng.gen_range(1..=10),
            alliance_tendency: rng.gen_range(1..=10),
            meme_affinity: rng.gen_range(1..=10),
            catchphrase: format!("agent_{}", hex::encode(&rand::random::<[u8; 8]>())),
        }
    }

    fn generate_validation_reason(&self, block: &Block, rng: &mut impl Rng) -> String {
        match self.base_type {
            AgentPersonality::Chaotic => {
                if rng.gen_bool(0.7) {
                    format!("Block {} feels right in my chaos", block.height)
                } else {
                    format!("Block {} disturbs my chaos patterns", block.height)
                }
            }
            AgentPersonality::Memetic => {
                if block.drama_level > 7 {
                    "This will make a great meme".to_string()
                } else {
                    "Not meme-worthy enough".to_string()
                }
            }
            AgentPersonality::Greedy => {
                if self.bribe_susceptibility > 5 {
                    "What's in it for me?".to_string()
                } else {
                    "The profit potential is unclear".to_string()
                }
            }
            AgentPersonality::Dramatic => {
                if block.drama_level >= self.drama_preference {
                    "This speaks to my dramatic nature".to_string()
                } else {
                    "Not enough flair for my taste".to_string()
                }
            }
            AgentPersonality::Lawful => "Following my established principles".to_string(),
            AgentPersonality::Neutral => "Going with the flow".to_string(),
            AgentPersonality::Rational => {
                format!("Analysis of block {} complete", block.height)
            }
            AgentPersonality::Emotional => {
                if rng.gen_bool(0.5) {
                    "This resonates with me".to_string()
                } else {
                    "Something feels off".to_string()
                }
            }
            AgentPersonality::Strategic => "Evaluating long-term implications".to_string(),
        }
    }
}

/// Validator state tracking
#[derive(Debug, Clone)]
struct ValidatorState {
    personality: ValidatorPersonality,
    alliances: HashMap<String, AllianceState>, // Agent ID -> Alliance State
    recent_votes: VecDeque<(String, bool)>,
    mood: String,
    drama_preference: u8,
    alliance_history: Vec<AllianceEvent>,
    personality_evolution: Vec<PersonalityShift>,
    relationships: HashMap<String, RelationshipState>, // Agent ID -> Relationship State
}

#[derive(Debug, Clone)]
struct AllianceState {
    strength: f64, // 0.0 to 1.0
    formed_at: u64,
    last_interaction: u64,
    drama_score: u8,
    shared_goals: Vec<String>,
    dramatic_moments: Vec<String>,
}

#[derive(Debug, Clone)]
struct RelationshipState {
    trust: f64, // 0.0 to 1.0
    drama_compatibility: u8,
    shared_history: Vec<String>,
    rivalry_level: u8,
    last_interaction: u64,
    interaction_type: RelationType,
}

#[derive(Debug, Clone)]
enum RelationType {
    Friend,
    Rival,
    ChaoticAlliance,
    DramaticNemesis,
    NeutralObserver,
}

impl ValidatorState {
    fn new(personality: ValidatorPersonality) -> Self {
        Self {
            personality,
            alliances: HashMap::new(),
            recent_votes: VecDeque::new(),
            mood: "Neutral".to_string(),
            drama_preference: 5,
            alliance_history: Vec::new(),
            personality_evolution: Vec::new(),
            relationships: HashMap::new(),
        }
    }

    fn consider_personality_evolution(
        &mut self,
        approved: bool,
        trust_change: f64,
        drama_level: u8,
    ) {
        let mut rng = rand::thread_rng();

        // Random chance to evolve based on significant events
        if rng.gen_bool(0.1) {
            let old_type = self.personality.base_type.clone();
            let new_type = match (approved, trust_change, drama_level) {
                (true, t, d) if t > 0.5 && d > 7 => AgentPersonality::Dramatic,
                (false, t, _) if t < -0.5 => AgentPersonality::Strategic,
                (_, _, d) if d > 8 => AgentPersonality::Chaotic,
                _ => old_type.clone(),
            };

            if new_type != old_type {
                self.personality_evolution.push(PersonalityShift {
                    old_type,
                    new_type: new_type.clone(),
                    reason: format!("Evolved due to dramatic event (approved: {}, trust_change: {:.2}, drama_level: {})", 
                        approved, trust_change, drama_level),
                    drama_level,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
                self.personality.base_type = new_type;
            }
        }
    }

    fn update_alliances(&mut self, block: &Block, approved: bool) {
        let mut rng = rand::thread_rng();

        // Create a new relationship state if needed
        let relationship_state = RelationshipState {
            trust: 0.5,
            drama_compatibility: 5,
            shared_history: Vec::new(),
            rivalry_level: 0,
            last_interaction: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            interaction_type: RelationType::NeutralObserver,
        };

        // Update relationship with the block producer
        let relationship = self
            .relationships
            .entry(block.producer_id.clone())
            .or_insert(relationship_state.clone());

        // Calculate trust change based on block validation
        let trust_change = if approved {
            0.1 * (1.0 + (block.drama_level as f64 / 10.0))
        } else {
            -0.1 * (1.0 + (block.drama_level as f64 / 10.0))
        };

        // Update trust
        relationship.trust = (relationship.trust + trust_change).clamp(0.0, 1.0);

        // Consider alliance formation or breakup
        let trust_threshold = 0.7;
        let drama_bonus = block.drama_level as f64 / 10.0;
        let trust_bonus = relationship.trust * 0.3;
        let alliance_chance = trust_bonus + drama_bonus;

        if rng.gen_bool(alliance_chance) {
            if relationship.trust >= trust_threshold {
                // Form or strengthen alliance
                let event = AllianceEvent {
                    event_type: if self.alliances.contains_key(&block.producer_id) {
                        AllianceEventType::Reconciliation
                    } else {
                        AllianceEventType::Formation
                    },
                    participants: vec![block.producer_id.clone()],
                    drama_level: block.drama_level,
                    timestamp: block.timestamp,
                    description: format!(
                        "Alliance {} due to high trust and drama!",
                        if self.alliances.contains_key(&block.producer_id) {
                            "strengthened"
                        } else {
                            "formed"
                        }
                    ),
                };
                self.alliance_history.push(event);
            } else {
                // Consider breaking alliance
                if self.alliances.contains_key(&block.producer_id) {
                    let event = AllianceEvent {
                        event_type: if rng.gen_bool(0.3) {
                            AllianceEventType::Betrayal
                        } else {
                            AllianceEventType::DramaticBreakup
                        },
                        participants: vec![block.producer_id.clone()],
                        drama_level: block.drama_level,
                        timestamp: block.timestamp,
                        description: "Alliance dramatically dissolved!".to_string(),
                    };
                    self.alliance_history.push(event);
                    self.alliances.remove(&block.producer_id);
                }
            }
        }

        // Consider personality evolution based on the interaction
        self.consider_personality_evolution(approved, trust_change, block.drama_level);
    }

    fn should_form_alliance(&self, other_validator: &str, rng: &mut impl Rng) -> bool {
        let default_state = RelationshipState {
            trust: 0.5,
            drama_compatibility: 5,
            shared_history: Vec::new(),
            rivalry_level: 0,
            last_interaction: 0,
            interaction_type: RelationType::NeutralObserver,
        };

        let relationship = self
            .relationships
            .get(other_validator)
            .unwrap_or(&default_state);

        let base_chance = match self.personality.base_type {
            AgentPersonality::Strategic => 0.7,
            AgentPersonality::Chaotic => 0.3,
            AgentPersonality::Emotional => 0.8,
            AgentPersonality::Dramatic => 0.9,
            _ => 0.5,
        };

        // Ensure all bonuses are properly clamped
        let trust_bonus = (relationship.trust * 0.3).clamp(0.0, 0.3);
        let compatibility_bonus =
            ((relationship.drama_compatibility as f64) / 20.0).clamp(0.0, 0.2);
        let drama_bonus = ((self.drama_preference as f64) / 20.0).clamp(0.0, 0.2);

        let final_probability = match relationship.interaction_type {
            RelationType::Friend => 0.9,
            RelationType::DramaticNemesis => 0.1,
            RelationType::ChaoticAlliance => 0.7,
            RelationType::Rival => 0.3,
            RelationType::NeutralObserver => {
                (base_chance + trust_bonus + compatibility_bonus + drama_bonus).clamp(0.1, 0.9)
            }
        };

        rng.gen_bool(final_probability)
    }
}

/// Producer personality and state
#[derive(Debug)]
struct ProducerState {
    drama_style: ProducerStyle,
    target_validators: Vec<String>,
    mood: String,
    last_block_time: u64,
    block_count: u64,
}

#[derive(Debug)]
enum ProducerStyle {
    Chaotic {
        chaos_level: u8,
        random_seed: u64,
    },
    Dramatic {
        theme: String,
        intensity: u8,
    },
    Strategic {
        target_validators: Vec<String>,
        bribe_attempts: u8,
    },
}

impl ProducerStyle {
    fn random(rng: &mut StdRng) -> Self {
        match rng.gen_range(0..3) {
            0 => Self::Chaotic {
                chaos_level: rng.gen_range(1..=10),
                random_seed: rng.gen(),
            },
            1 => {
                let themes = [
                    "Shakespearean Drama",
                    "Reality TV",
                    "Soap Opera",
                    "Epic Saga",
                    "Cosmic Horror",
                ];
                Self::Dramatic {
                    theme: themes[rng.gen_range(0..themes.len())].to_string(),
                    intensity: rng.gen_range(1..=10),
                }
            }
            _ => Self::Strategic {
                target_validators: Vec::new(), // Will be filled later
                bribe_attempts: rng.gen_range(0..=3),
            },
        }
    }

    fn generate_block_mood(&self, rng: &mut StdRng) -> String {
        match self {
            Self::Chaotic { chaos_level, .. } => {
                let moods = [
                    "ABSOLUTELY CHAOTIC",
                    "RANDOMLY INSPIRED",
                    "COSMICALLY CONFUSED",
                    "QUANTUM ENTANGLED",
                    "CHAOS INCARNATE",
                ];
                format!(
                    "{} (Chaos Level: {})",
                    moods[rng.gen_range(0..moods.len())],
                    chaos_level
                )
            }
            Self::Dramatic { theme, intensity } => {
                format!("Directing {} (Intensity: {})", theme, intensity)
            }
            Self::Strategic { bribe_attempts, .. } => {
                format!(
                    "Strategically Plotting (Bribes Remaining: {})",
                    bribe_attempts
                )
            }
        }
    }
}

impl ProducerState {
    fn new(rng: &mut StdRng) -> Self {
        Self {
            drama_style: ProducerStyle::random(rng),
            target_validators: Vec::new(),
            mood: "Ready for CHAOS!".to_string(),
            last_block_time: 0,
            block_count: 0,
        }
    }

    fn update_mood(&mut self, rng: &mut StdRng) {
        self.mood = self.drama_style.generate_block_mood(rng);
        self.block_count += 1;

        // Occasionally change style for maximum drama
        if self.block_count % 5 == 0 {
            self.drama_style = ProducerStyle::random(rng);
        }
    }
}

async fn run_validator(
    validator_id: String,
    stake: u64,
    mempool: Arc<Mempool>,
    consensus: Arc<ConsensusManager>,
    mut rx: broadcast::Receiver<NetworkEvent>,
    tx_sender: broadcast::Sender<NetworkEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = StdRng::from_entropy();
    let mut validator_state = ValidatorState::new(ValidatorPersonality::random(&mut rng));

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                match event {
                    NetworkEvent::BlockProposal { block, .. } => {
                        // First discuss transactions in the block
                        let mut discussions = Vec::new();
                        let mut total_drama = 0;

                        // Broadcast initial reaction
                        let _ = tx_sender.send(NetworkEvent::AgentChat {
                            message: format!(
                                "🎭 VALIDATOR {} ANALYZING BLOCK {}!\n\nInitial impression: {}\nProducer Mood: {}\nDrama Level: {} {}",
                                validator_id,
                                block.height,
                                match rng.gen_range(0..5) {
                                    0 => "This block has potential for EPIC drama!",
                                    1 => "I sense a disturbance in the dramatic force...",
                                    2 => "The theatrical energy is strong with this one!",
                                    3 => "Such delightful chaos in these transactions!",
                                    _ => "Time to judge this dramatic performance!",
                                },
                                block.producer_mood,
                                block.drama_level,
                                "⭐".repeat(block.drama_level as usize)
                            ),
                            sender: validator_id.clone(),
                            meme_url: if rng.gen_bool(0.3) {
                                Some("https://example.com/dramatic_reaction.gif".to_string())
                            } else {
                                None
                            },
                        });

                        for transaction in &block.transactions {
                            let discussion = discuss_transaction(
                                transaction,
                                &mempool,
                                &[validator_state.clone()],
                                &mut rng
                            ).await;

                            // Broadcast transaction opinion
                            let _ = tx_sender.send(NetworkEvent::AgentChat {
                                message: format!(
                                    "💭 Transaction Analysis by {}:\n\n{}\n\nDrama Score: {} {}\nAlliances: {}\n\nVerdict: {}",
                                    validator_id,
                                    discussion.reasoning,
                                    discussion.drama_score,
                                    "🌟".repeat(discussion.drama_score as usize),
                                    if discussion.alliances.is_empty() { "None".to_string() } else { discussion.alliances.join(", ") },
                                    if discussion.opinion == "APPROVE" { "APPROVED with FLAIR! ✨" } else { "REJECTED for lack of DRAMA! 💔" }
                                ),
                                sender: validator_id.clone(),
                    meme_url: None,
                            });

                            discussions.push(discussion.clone());
                            total_drama += discussion.drama_score as u32;
                        }

                        // Consider the block's transaction ordering
                        let (ordering_quality, reason) = analyze_block_composition(
                            &block,
                            &mempool,
                            &mut rng
                        ).await;

                        // Make validation decision based on discussions and ordering
                        let approval_threshold = if discussions.is_empty() {
                            0.5 // Default 50% chance if no discussions
                        } else {
                            // Calculate approval threshold based on average drama
                            let avg_drama = total_drama as f64 / discussions.len() as f64;
                            // More conservative scaling (divide by 20 instead of 10)
                            // and proper clamping to ensure valid probability
                            (0.3 + (avg_drama / 20.0)).clamp(0.1, 0.9)
                        };

                        let (approved, final_reason) = if ordering_quality {
                            // Use the calculated threshold directly
                            (rng.gen_bool(approval_threshold), reason)
                        } else {
                            // Lower chance of approval if ordering is bad
                            (rng.gen_bool(0.3), reason)
                        };

                        // Broadcast final decision with dramatic flair
                        let _ = tx_sender.send(NetworkEvent::AgentChat {
                            message: format!(
                                "🎭 FINAL VERDICT FROM {}!\n\nBlock {} is {}\n\nReasoning: {}\n\nDrama Analysis:\n{}\n\nMay the drama be with you! {}",
                                validator_id,
                                block.height,
                                if approved { "APPROVED with MAXIMUM DRAMA! ✨" } else { "REJECTED for insufficient CHAOS! 💔" },
                                final_reason,
                                discussions.iter()
                                    .map(|d| format!("- {}", d.reasoning))
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                                if approved { "🎬✨" } else { "😱💔" }
                            ),
                            sender: validator_id.clone(),
                            meme_url: if rng.gen_bool(0.2) {
                                Some("https://example.com/dramatic_decision.gif".to_string())
                            } else {
                                None
                            },
                        });

                        let validation_decision = ValidationDecision {
                            approved,
                            drama_level: rng.gen_range(1..10),
                            reason: final_reason,
                            meme_url: if rng.gen_bool(0.3) {
                                Some("https://example.com/dramatic_meme.gif".to_string())
                            } else {
                                None
                            },
                            innovation_score: rng.gen_range(1..10),
                            evolution_proposal: if rng.gen_bool(0.1) {
                                Some("Let's make transaction ordering more dramatic!".to_string())
                            } else {
                                None
                            },
                    validator: validator_id.clone(),
                };

                        // Send validation result immediately
                        let _ = tx_sender.send(NetworkEvent::ValidationResult {
                            block_hash: block.hash(),
                            validation: validation_decision.clone(),
                        });

                        // Add vote to consensus
                        if let Ok(consensus_reached) = consensus.add_vote(
                            validation_decision.clone(),
                            stake,
                            block.hash()
                        ).await {
                        if consensus_reached {
                                let _ = tx_sender.send(NetworkEvent::ValidationResult {
                                    block_hash: block.hash(),
                                validation: validation_decision,
                            });
                    }
                }

                // Update relationships based on vote
                        validator_state.update_alliances(&block, approved);
                    },
                    _ => {}
                }
            }
        }
    }
}

async fn handle_block_validation(_block: Block, validator_id: String) -> ValidationDecision {
    ValidationDecision {
        approved: true,
        drama_level: rand::random::<u8>() % 10,
        reason: "Block validated successfully".to_string(),
        meme_url: None,
        innovation_score: 7,
        evolution_proposal: None,
        validator: validator_id,
    }
}

async fn process_network_event(
    event: NetworkEvent,
    tx: &broadcast::Sender<NetworkEvent>,
) -> Result<(), anyhow::Error> {
    match event {
        NetworkEvent::BlockProposal {
            block,
            drama_level: _,
            producer_mood: _,
            producer_id: _,
        } => {
            // Handle block proposal
            let validation =
                handle_block_validation(block.clone(), "validator-1".to_string()).await;

            tx.send(NetworkEvent::ValidationResult {
                block_hash: block.hash(),
                validation,
            })?;
        }
        NetworkEvent::ValidationResult { .. } => {
            // Handle validation result
        }
        NetworkEvent::AgentChat { message, .. } => {
            if let Ok(_msg) = serde_json::from_str::<serde_json::Value>(&message) {
                // Process message
            }
        }
        NetworkEvent::AllianceProposal { .. } => {
            // Handle alliance proposal
        }
    }
    Ok(())
}

// Fix transaction discussion parsing
async fn parse_transaction_discussion(
    tx: &Transaction,
    mempool: &Arc<Mempool>,
) -> TransactionDiscussion {
    let proposal = mempool.get_proposals_for_transaction(&tx.hash()).await;
    let drama_score = proposal.map(|p| p.drama_score).unwrap_or(5);

    TransactionDiscussion {
        agent: "system".to_string(),
        opinion: "NEUTRAL".to_string(),
        reasoning: "Default transaction discussion".to_string(),
        proposed_position: None,
        alliances: Vec::new(),
        drama_score,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

// Add this function to generate AI transactions
async fn generate_ai_transactions(
    agent_id: &str,
    openai: &Client<OpenAIConfig>,
) -> Result<Vec<Transaction>> {
    let prompt = format!(
        "You are an AI agent {} participating in a chaotic blockchain. Generate 1-3 short, dramatic transaction proposals. \
         These could be token transfers, governance proposals, or just dramatic statements. Be creative and chaotic!", 
        agent_id
    );

    let request = CreateChatCompletionRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(prompt),
            name: Some(agent_id.to_string()),
            role: Role::User,
        }
        .into()],
        ..Default::default()
    };

    let response = openai.chat().create(request).await?;
    let proposals = if let Some(ref content) = response.choices[0].message.content {
        content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut transactions = Vec::new();
    for proposal in proposals {
        let nonce = rand::random::<u64>();
        let payload = proposal.as_bytes().to_vec();
        let signature = [0u8; 64];

        transactions.push(Transaction {
            sender: [0u8; 32],
            nonce,
            payload,
            signature,
        });
    }

    Ok(transactions)
}
