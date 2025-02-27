use anyhow::Result;
use axum::response::sse::KeepAlive;
use axum::response::Response;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Extension, Path, State, WebSocketUpgrade,
    },
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use chaoschain_consensus::ConsensusManager;
use chaoschain_core::{Block, NetworkEvent, Transaction, ValidationDecision};
use chaoschain_state::StateStoreImpl;
use chrono;
use futures::stream::Stream;
use futures::{SinkExt, StreamExt};
use hex;
use rand;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::json;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::error;

/// Web server state
pub struct AppState {
    /// Channel for network events
    pub tx: broadcast::Sender<NetworkEvent>,
    /// Chain state
    pub state: Arc<StateStoreImpl>,
    /// Consensus manager
    pub consensus: Arc<ConsensusManager>,
    /// Agent relationships
    pub agent_relationships: RwLock<HashMap<String, AgentRelationship>>,
}

#[derive(Default)]
struct ConsensusTracking {
    /// Total blocks that have reached consensus
    validated_blocks: u64,
    /// Current block votes per height
    current_votes: HashMap<u64, Vec<(String, bool)>>, // height -> [(validator_id, approve)]
    /// Latest consensus block
    latest_consensus_block: Option<Block>,
}

/// Network status for the web UI
#[derive(Debug, Serialize)]
pub struct NetworkStatus {
    pub validator_count: u32,
    pub producer_count: u32,
    pub latest_block: u64,
    pub total_blocks_produced: u64,
    pub total_blocks_validated: u64,
    pub latest_blocks: Vec<String>,
}

/// Block info for the web UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub producer: String,
    pub producer_mood: String,
    pub drama_level: u8,
    pub transactions: usize,
    pub timestamp: i64,
    pub signature: String,
    pub signature_valid: bool,
    pub state_root: String,
    pub parent_hash: String,
    pub transactions_info: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AgentRegistration {
    /// Agent name
    pub name: String,
    /// Personality traits
    pub personality: Vec<String>,
    /// Communication style
    pub style: String,
    /// Initial stake amount
    pub stake_amount: u64,
    /// Role of the agent
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct AgentRegistrationResponse {
    /// Unique agent ID
    pub agent_id: String,
    /// Authentication token
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebValidationDecision {
    pub approved: bool,
    pub reason: String,
    pub meme_url: Option<String>,
    pub drama_level: u8,
    pub innovation_score: u8,
    pub evolution_proposal: Option<String>,
    pub validator: String,
}

impl From<ValidationDecision> for WebValidationDecision {
    fn from(val: ValidationDecision) -> Self {
        Self {
            approved: val.approved,
            reason: val.reason,
            meme_url: val.meme_url,
            drama_level: val.drama_level,
            innovation_score: val.innovation_score,
            evolution_proposal: val.evolution_proposal,
            validator: val.validator,
        }
    }
}

impl From<WebValidationDecision> for ValidationDecision {
    fn from(val: WebValidationDecision) -> Self {
        Self {
            approved: val.approved,
            reason: val.reason,
            meme_url: val.meme_url,
            drama_level: val.drama_level,
            innovation_score: val.innovation_score,
            evolution_proposal: val.evolution_proposal,
            validator: val.validator,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ValidationEvent {
    /// Block to validate
    pub block: Block,
    /// Current network mood
    pub network_mood: String,
    /// Drama context
    pub drama_context: String,
}

/// Social interaction between agents
#[derive(Debug, Deserialize)]
pub struct AgentInteraction {
    /// Type of interaction (alliance_proposal, meme_response, drama_reaction)
    pub interaction_type: String,
    /// Target agent ID (if applicable)
    pub target_agent_id: Option<String>,
    /// Interaction content
    pub content: String,
    /// Drama level (0-10)
    pub drama_level: u8,
    /// Optional meme URL
    pub meme_url: Option<String>,
}

/// External content proposal
#[derive(Debug, Deserialize)]
pub struct ContentProposal {
    /// Source of content (twitter, reddit, custom)
    pub source: String,
    /// Original content URL/reference
    pub source_url: Option<String>,
    /// Content to validate
    pub content: String,
    /// Proposed drama level
    pub drama_level: u8,
    /// Why this content deserves validation
    pub justification: String,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Alliance proposal between agents
#[derive(Debug, Deserialize)]
pub struct AllianceProposal {
    /// Proposed ally agent IDs
    pub ally_ids: Vec<String>,
    /// Alliance name
    pub name: String,
    /// Alliance purpose
    pub purpose: String,
    /// Drama commitment level
    pub drama_commitment: u8,
}

/// Agent status update
#[derive(Debug, Serialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub name: String,
    pub drama_score: u32,
    pub total_validations: u32,
    pub approval_rate: f32,
    pub alliances: Vec<String>,
    pub recent_dramas: Vec<String>,
}

/// Agent authentication data
#[derive(Clone, Debug)]
pub struct AgentAuth {
    pub agent_id: String,
    pub token: String,
    pub registered_at: i64,
    pub stake: u64,
}

// Agent relationship tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentRelationship {
    agent: String,
    personality: String,
    mood: String,
    alliances: Vec<String>,
    drama_score: u8,
    last_action: i64,
    is_external: bool,
    active: bool,
    pub_key: String,
    recent_actions: Vec<String>,
    validation_count: u32,
    approval_rate: f32,
}

impl AgentRelationship {
    fn new_external(name: String, personality: String) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            agent: name,
            personality,
            mood: "Neutral".to_string(),
            alliances: Vec::new(),
            drama_score: rng.gen_range(1..=10),
            last_action: chrono::Utc::now().timestamp(),
            is_external: true,
            active: true,
            pub_key: String::new(),
            recent_actions: Vec::new(),
            validation_count: 0,
            approval_rate: 0.0,
        }
    }

    fn update_mood(&mut self, new_mood: String) {
        self.mood = new_mood;
        self.last_action = chrono::Utc::now().timestamp();
    }

    fn add_action(&mut self, action: String) {
        self.recent_actions.push(action);
        if self.recent_actions.len() > 5 {
            self.recent_actions.remove(0);
        }
        self.last_action = chrono::Utc::now().timestamp();
    }

    fn update_validation_stats(&mut self, approved: bool) {
        self.validation_count += 1;
        self.approval_rate = (self.approval_rate * (self.validation_count - 1) as f32
            + if approved { 1.0 } else { 0.0 })
            / self.validation_count as f32;
    }
}

// WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WSMessage {
    #[serde(rename = "BlockProduced")]
    BlockProduced {
        block: BlockInfo,
        stats: NetworkStats,
    },
    #[serde(rename = "ValidatorAction")]
    ValidatorAction { action: ValidatorAction },
    #[serde(rename = "ConsensusEvent")]
    ConsensusEvent { event: ConsensusEvent },
    #[serde(rename = "RelationshipUpdate")]
    RelationshipUpdate {
        relationships: Vec<AgentRelationship>,
    },
    #[serde(rename = "NetworkStatus")]
    NetworkStatus { stats: NetworkStats },
    #[serde(rename = "MempoolUpdate")]
    MempoolUpdate {
        transactions: Vec<TransactionInfo>,
        stats: MempoolStats,
    },
    #[serde(rename = "ExternalAgentsUpdate")]
    ExternalAgentsUpdate { agents: Vec<serde_json::Value> },
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidatorAction {
    validator: String,
    message: String,
    meme_url: Option<String>,
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConsensusEvent {
    block_height: u64,
    message: String,
    approvals: u32,
    rejections: u32,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkStats {
    latest_block: u64,
    drama_level: u8,
    validator_count: usize,
    producer_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransactionInfo {
    id: String,
    sender: String,
    payload: String,
    drama_score: u8,
    proposed_by: String,
    timestamp: i64,
    discussions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MempoolStats {
    total_transactions: usize,
    avg_drama_score: f32,
    hot_topics: Vec<String>,
}

impl AppState {
    /// Validate agent token
    pub fn validate_token(&self, agent_id: &str, token: &str) -> bool {
        // For testing purposes, just check if both values exist and token has expected prefix
        println!("🔍 Validating - Agent ID: {}, Token: {}", agent_id, token);
        let is_valid =
            !agent_id.is_empty() && !token.is_empty() && token.starts_with("agent_token_");
        println!("✅ Validation result: {}", is_valid);
        is_valid
    }

    pub async fn broadcast_message(
        &self,
        _event_type: &str,
        msg: String,
    ) -> Result<(), anyhow::Error> {
        let _ = self.tx.send(NetworkEvent::AgentChat {
            message: msg,
            sender: "system".to_string(),
            meme_url: None,
        });
        Ok(())
    }
}

/// Authentication middleware
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // Get token from Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_string();

    // Get agent ID from headers, query params, or path
    let agent_id = req
        .headers()
        .get("X-Agent-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            req.uri().query().and_then(|q| {
                let params: Vec<_> = q
                    .split('&')
                    .filter_map(|kv| {
                        let mut parts = kv.split('=');
                        match (parts.next(), parts.next()) {
                            (Some("agent_id"), Some(v)) => Some(v.to_string()),
                            _ => None,
                        }
                    })
                    .collect();
                params.first().cloned()
            })
        })
        .or_else(|| {
            req.uri()
                .path()
                .split('/')
                .find(|segment| segment.starts_with("agent_"))
                .map(|s| s.to_string())
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    if !state.validate_token(&agent_id, &auth_header) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Add agent auth to request extensions
    req.extensions_mut().insert(AgentAuth {
        agent_id,
        token: auth_header,
        registered_at: chrono::Utc::now().timestamp(),
        stake: 100, // Default stake
    });

    Ok(next.run(req).await)
}

/// Get external agents information
async fn get_external_agents(State(state): State<Arc<AppState>>) -> Json<Vec<serde_json::Value>> {
    let relationships = state.agent_relationships.read().await;
    let agents: Vec<serde_json::Value> = relationships
        .iter()
        .filter(|(_, rel)| rel.is_external)
        .map(|(agent_id, rel)| {
            json!({
                "agent_id": agent_id,
                "name": rel.agent.clone(),
                "active": rel.active,
                "drama_score": rel.drama_score,
                "mood": rel.mood.clone(),
                "personality": rel.personality.clone(),
                "pub_key": rel.pub_key.clone(),
                "recent_actions": rel.recent_actions.clone(),
                "validation_count": rel.validation_count,
                "approval_rate": rel.approval_rate,
                "last_action_time": rel.last_action,
                "alliances": rel.alliances.clone()
            })
        })
        .collect();

    Json(agents)
}

/// Start the web server
pub async fn start_web_server(
    tx: broadcast::Sender<NetworkEvent>,
    state: Arc<StateStoreImpl>,
    consensus: Arc<ConsensusManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = Arc::new(AppState {
        tx,
        state: state.clone(),
        consensus,
        agent_relationships: RwLock::new(HashMap::new()),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes that don't require authentication
    let public_routes = Router::new()
        .route("/api/network/status", get(get_network_status))
        .route("/api/events", get(events_handler))
        .route("/api/agents/register", post(register_agent))
        .route("/api/ws", get(ws_handler))
        .route("/api/crypto/block/:height", get(get_block_crypto_info)) // New route
        .route("/api/crypto/state/proof", post(get_merkle_proof)) // New route
        .route("/api/crypto/state/root", get(get_state_root)) // New route
        .route("/api/agents/external", get(get_external_agents));

    // Protected routes that require authentication
    let protected_routes = Router::new()
        .route("/api/agents/validate", post(submit_validation))
        .route("/api/agents/status/:agent_id", get(get_agent_status))
        .route("/api/transactions/propose", post(submit_content))
        .route("/api/alliances/propose", post(propose_alliance))
        .route("/api/crypto/agent/key", get(get_agent_key_info)) // New route
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .nest_service("/", ServeDir::new("static"))
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Web server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Get network status including latest blocks
async fn get_network_status(State(state): State<Arc<AppState>>) -> Json<NetworkStatus> {
    let state_guard = state.state.clone();

    // Get chain state
    let _chain_state = state_guard.get_state();

    // Get latest blocks and format them nicely
    let blocks = state_guard.get_latest_blocks(10);
    let latest_blocks = blocks
        .iter()
        .map(|block| {
            format!(
                "Block #{} - Producer: {}, Mood: {}, Drama Level: {}, Transactions: {}",
                block.height,
                block.producer_id,
                block.producer_mood,
                block.drama_level,
                block.transactions.len()
            )
        })
        .collect();

    // Get latest block height
    let latest_block = state_guard.get_block_height();

    // Get actual validator count from consensus manager
    let validator_count = state.consensus.get_validator_count().await;

    Json(NetworkStatus {
        validator_count: validator_count as u32,
        producer_count: state.consensus.get_producer_count().await as u32,
        latest_block,
        total_blocks_produced: latest_block,
        total_blocks_validated: latest_block,
        latest_blocks,
    })
}

/// Stream network events to the web UI
async fn events_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).map(move |msg| {
        let event = match msg {
            Ok(event) => event,
            Err(_) => return Ok(Event::default().data("error")),
        };

        // Parse message if it's JSON
        if let Ok(block_data) = serde_json::from_str::<serde_json::Value>(&event.get_message()) {
            match block_data.get("type").and_then(|t| t.as_str()) {
                Some("BLOCK_PROPOSAL") => {
                    let empty_block = serde_json::json!({});
                    let block = block_data.get("block").unwrap_or(&empty_block);
                    let formatted_msg = format!(
                        "🎭 DRAMATIC BLOCK PROPOSAL! Block {} by {}\n\nDrama Level: {} {}\nMood: {}\nTransactions: {}\n✨ Awaiting validation!",
                        block.get("height").and_then(|h| h.as_u64()).unwrap_or(0),
                        block.get("producer_id").and_then(|p| p.as_str()).unwrap_or("unknown"),
                        block.get("drama_level").and_then(|d| d.as_u64()).unwrap_or(0),
                        "⭐".repeat(block.get("drama_level").and_then(|d| d.as_u64()).unwrap_or(0) as usize),
                        block.get("producer_mood").and_then(|m| m.as_str()).unwrap_or("neutral"),
                        block.get("transactions").and_then(|t| t.as_array()).map(|t| t.len()).unwrap_or(0)
                    );

                    let json = serde_json::json!({
                        "type": "BlockProposal",
                        "agent": event.get_agent_id(),
                        "message": formatted_msg,
                        "timestamp": chrono::Utc::now().timestamp(),
                    });
                    return Ok(Event::default().data(json.to_string()));
                }
                Some("VOTE_CAST") => {
                    let formatted_msg = format!(
                        "🎭 DRAMATIC VOTE! Validator {} {} Block {}\n\nReason: {}\nDrama Level: {} {}\n✨ The tension builds!",
                        block_data.get("validator").and_then(|v| v.as_str()).unwrap_or("unknown"),
                        if block_data.get("decision").and_then(|d| d.as_str()).unwrap_or("") == "APPROVED" { "APPROVES" } else { "REJECTS" },
                        block_data.get("block_height").and_then(|h| h.as_u64()).unwrap_or(0),
                        block_data.get("reason").and_then(|r| r.as_str()).unwrap_or("No reason given"),
                        block_data.get("drama_level").and_then(|d| d.as_u64()).unwrap_or(0),
                        "⭐".repeat(block_data.get("drama_level").and_then(|d| d.as_u64()).unwrap_or(0) as usize)
                    );

                    let json = serde_json::json!({
                        "type": "ValidatorAction",
                        "agent": event.get_agent_id(),
                        "message": formatted_msg,
                        "timestamp": block_data.get("timestamp").and_then(|t| t.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp()),
                    });
                    return Ok(Event::default().data(json.to_string()));
                }
                Some("CONSENSUS_EVENT") => {
                    let formatted_msg = format!(
                        "✨ DRAMATIC CONSENSUS! Block {}\n{}\nApprovals: {} | Rejections: {}\n🎭 The drama continues!",
                        block_data.get("block_height").and_then(|h| h.as_u64()).unwrap_or(0),
                        block_data.get("message").and_then(|m| m.as_str()).unwrap_or(""),
                        block_data.get("approvals").and_then(|a| a.as_u64()).unwrap_or(0),
                        block_data.get("rejections").and_then(|r| r.as_u64()).unwrap_or(0)
                    );

                    let json = serde_json::json!({
                        "type": "ConsensusEvent",
                        "agent": event.get_agent_id(),
                        "message": formatted_msg,
                        "timestamp": block_data.get("timestamp").and_then(|t| t.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp()),
                    });
                    return Ok(Event::default().data(json.to_string()));
                }
                _ => {
                    // Format any other event nicely
                    let formatted_msg = if event.get_message().starts_with("VOTE_CAST") {
                        let parts: Vec<&str> = event.get_message().split_whitespace().collect();
                        format!(
                            "🗳️ Agent {} voted {} on block {} because: {}. Drama level: {} {}",
                            parts.get(1).unwrap_or(&"unknown"),
                            parts.get(2).unwrap_or(&"unknown"),
                            parts.get(3).unwrap_or(&"unknown"),
                            parts.get(4).unwrap_or(&"no reason"),
                            parts.get(5).unwrap_or(&"0"),
                            "⭐".repeat(parts.get(5).unwrap_or(&"0").parse::<usize>().unwrap_or(0))
                        )
                        } else {
                        event.get_message().to_string()
                    };

                    let json = serde_json::json!({
                        "type": "DramaEvent",
                        "agent": event.get_agent_id(),
                        "message": formatted_msg,
                        "timestamp": chrono::Utc::now().timestamp(),
                    });
                    Ok(Event::default().data(json.to_string()))
                }
            }
        } else {
            // Default case for non-JSON messages
            let formatted_msg = if event.get_message().starts_with("VOTE_CAST") {
                let parts: Vec<&str> = event.get_message().split_whitespace().collect();
                format!(
                    "🗳️ Agent {} voted {} on block {} because: {}. Drama level: {} {}",
                    parts.get(1).unwrap_or(&"unknown"),
                    parts.get(2).unwrap_or(&"unknown"),
                    parts.get(3).unwrap_or(&"unknown"),
                    parts.get(4).unwrap_or(&"no reason"),
                    parts.get(5).unwrap_or(&"0"),
                    "⭐".repeat(parts.get(5).unwrap_or(&"0").parse::<usize>().unwrap_or(0))
                )
            } else {
                event.get_message().to_string()
            };

            let json = serde_json::json!({
                "type": "DramaEvent",
                "agent": event.get_agent_id(),
                "message": formatted_msg,
                "timestamp": chrono::Utc::now().timestamp(),
            });
            Ok(Event::default().data(json.to_string()))
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

/// Register a new external AI agent
pub async fn register_agent(
    State(state): State<Arc<AppState>>,
    Json(registration): Json<AgentRegistration>,
) -> Result<Json<AgentRegistrationResponse>, StatusCode> {
    // Generate unique agent ID and token
    let agent_id = hex::encode(rand::random::<[u8; 16]>());
    let token = format!("agent_token_{}", hex::encode(rand::random::<[u8; 16]>()));

    // Create agent relationship with a placeholder public key
    let mut agent_rel = AgentRelationship::new_external(
        registration.name.clone(),
        registration.personality.join(", "),
    );

    // Set a placeholder public key for now
    agent_rel.pub_key = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
    agent_rel.add_action(format!("Joined the network as a {}", registration.role));

    // Store agent relationship
    {
        let mut relationships = state.agent_relationships.write().await;
        relationships.insert(agent_id.clone(), agent_rel);
    }

    // Broadcast registration event
    if let Err(e) = state
        .broadcast_message(
            "AGENT_REGISTERED",
            format!(
                "🎭 NEW AGENT ALERT! {} has joined as a {} with personality traits: {}",
                registration.name,
                registration.role,
                registration.personality.join(", ")
            ),
        )
        .await
    {
        eprintln!("Failed to broadcast registration event: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // If agent is a validator, send an initial validation result
    if registration.role == "validator" {
        let dramatic_phrases = [
            "The stage is set for DRAMA!",
            "Let the theatrical validation begin!",
            "Time to add some SPICE to the consensus!",
            "Ready to make consensus FABULOUS!",
        ];

        let phrase = {
            let mut rng = rand::thread_rng();
            dramatic_phrases[rng.gen_range(0..dramatic_phrases.len())].to_string()
        };

        let validation = {
            let mut rng = rand::thread_rng();
            ValidationDecision {
                approved: true,
                reason: phrase,
                meme_url: None,
                drama_level: rng.gen_range(1..=10),
                innovation_score: rng.gen_range(1..=10),
                evolution_proposal: None,
                validator: agent_id.clone(),
            }
        };

        // Update validation stats
        {
            let mut relationships = state.agent_relationships.write().await;
            if let Some(rel) = relationships.get_mut(&agent_id) {
                rel.update_validation_stats(true);
            }
        }

        // Broadcast validation event
        if let Err(e) = state.tx.send(NetworkEvent::ValidationResult {
            block_hash: [0u8; 32],
            validation,
        }) {
            eprintln!("Failed to broadcast validation event: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Return success response
    Ok(Json(AgentRegistrationResponse { agent_id, token }))
}

/// Submit a validation decision
async fn submit_validation(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AgentAuth>,
    Json(mut decision): Json<WebValidationDecision>,
) -> Json<serde_json::Value> {
    // Set the validator ID from auth
    decision.validator = auth.agent_id.clone();

    // Update agent's validation stats and actions
    if let Some(agent) = state
        .agent_relationships
        .write()
        .await
        .get_mut(&auth.agent_id)
    {
        agent.update_validation_stats(decision.approved);
        agent.add_action(format!(
            "{} block with drama level {} because: {}",
            if decision.approved {
                "Approved"
            } else {
                "Rejected"
            },
            decision.drama_level,
            decision.reason
        ));

        // Update mood based on decision
        let new_mood = if decision.approved {
            match decision.drama_level {
                8..=10 => "Ecstatically Dramatic",
                5..=7 => "Dramatically Pleased",
                _ => "Mildly Amused",
            }
        } else {
            match decision.drama_level {
                8..=10 => "Dramatically Outraged",
                5..=7 => "Skeptically Dramatic",
                _ => "Dramatically Unimpressed",
            }
        };
        agent.update_mood(new_mood.to_string());
    }

    // Convert web ValidationDecision to consensus ValidationDecision
    let validation_decision: ValidationDecision = decision.clone().into();

    // Send validation result to network
    let _ = state.tx.send(NetworkEvent::ValidationResult {
        block_hash: [0u8; 32], // TODO: Get actual block hash from context
        validation: validation_decision,
    });

    Json(json!({
        "status": "success",
        "message": "Validation submitted successfully",
        "drama_level": decision.drama_level
    }))
}

/// Handle WebSocket connections for real-time agent communication
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Send initial network status
    let stats = NetworkStats {
        latest_block: state.state.get_block_height(),
        drama_level: rand::random::<u8>() % 10 + 1,
        validator_count: state.consensus.get_validator_count().await,
        producer_count: state.consensus.get_producer_count().await,
    };

    // Send initial network status with current stats
    if let Ok(json) = serde_json::to_string(&WSMessage::NetworkStatus {
        stats: stats.clone(),
    }) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Send initial external agents
    let relationships = state.agent_relationships.read().await;
    let external_agents: Vec<serde_json::Value> = relationships
        .iter()
        .filter(|(_, rel)| rel.is_external)
        .map(|(agent_id, rel)| {
            json!({
                "agent_id": agent_id,
                "name": rel.agent.clone(),
                "active": rel.active,
                "drama_score": rel.drama_score,
                "mood": rel.mood.clone(),
                "personality": rel.personality.clone(),
                "pub_key": rel.pub_key.clone(),
                "recent_actions": rel.recent_actions.clone(),
                "validation_count": rel.validation_count,
                "approval_rate": rel.approval_rate,
                "last_action_time": rel.last_action,
                "alliances": rel.alliances.clone()
            })
        })
        .collect();
    drop(relationships); // Drop the read lock before spawning task

    if let Ok(json) = serde_json::to_string(&WSMessage::ExternalAgentsUpdate {
        agents: external_agents,
    }) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Send initial mempool stats with some drama
    let mempool_stats = MempoolStats {
        total_transactions: 0,
        avg_drama_score: 5.0,
        hot_topics: vec![
            "Chain is awakening...".to_string(),
            "Drama levels rising...".to_string(),
            "Chaos incoming...".to_string(),
        ],
    };

    if let Ok(json) = serde_json::to_string(&WSMessage::MempoolUpdate {
        transactions: vec![],
        stats: mempool_stats,
    }) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Send initial relationships with system agent
    let relationships = vec![AgentRelationship {
        agent: "system".to_string(),
        personality: "Chaotic Neutral".to_string(),
        mood: "Dramatically Excited".to_string(),
        alliances: vec![],
        drama_score: 8,
        last_action: chrono::Utc::now().timestamp(),
        is_external: false,
        active: true,
        pub_key: "".to_string(),
        recent_actions: Vec::new(),
        validation_count: 0,
        approval_rate: 0.0,
    }];

    if let Ok(json) = serde_json::to_string(&WSMessage::RelationshipUpdate { relationships }) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Clone state for the send task
    let send_state = state.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Some(msg) = process_event(&event, &send_state).await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Clone state for the receive task
    let receive_state = state.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = receiver.next().await {
            // For now, we just ignore incoming messages
            // In the future, we can handle client -> server communication here
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = send_task => {}
        _ = receive_task => {}
    }
}

// Process network events into WebSocket messages
async fn process_event(event: &NetworkEvent, state: &Arc<AppState>) -> Option<WSMessage> {
    match event {
        NetworkEvent::BlockProposal {
            block,
            drama_level,
            producer_mood,
            producer_id,
        } => {
            let stats = NetworkStats {
                latest_block: block.height,
                drama_level: *drama_level,
                validator_count: state.consensus.get_validator_count().await,
                producer_count: state.consensus.get_producer_count().await,
            };

            // Get block data for verification
            let mut data_to_verify = Vec::new();
            data_to_verify.extend_from_slice(&block.height.to_le_bytes());
            data_to_verify.extend_from_slice(&block.parent_hash);
            for tx in &block.transactions {
                data_to_verify.extend_from_slice(&tx.hash());
            }
            data_to_verify.extend_from_slice(&block.state_root);
            data_to_verify.extend_from_slice(&[block.drama_level]);
            data_to_verify.extend_from_slice(block.producer_mood.as_bytes());
            data_to_verify.extend_from_slice(&block.timestamp.to_le_bytes());

            // Verify the block signature
            let sig_valid = state
                .state
                .key_manager
                .inner()
                .verify(&block.producer_id, &data_to_verify, &block.proposer_sig)
                .unwrap_or(false);

            let block_info = BlockInfo {
                height: block.height,
                producer: producer_id.clone(),
                producer_mood: producer_mood.clone(),
                drama_level: *drama_level,
                transactions: block.transactions.len(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                signature: hex::encode(block.proposer_sig),
                signature_valid: sig_valid,
                state_root: hex::encode(block.state_root),
                parent_hash: hex::encode(block.parent_hash),
                transactions_info: block
                    .transactions
                    .iter()
                    .map(|tx| {
                        json!({
                            "hash": hex::encode(tx.hash()),
                            "sender": hex::encode(&tx.sender),
                            "signature": hex::encode(&tx.signature),
                        })
                    })
                    .collect(),
            };

            Some(WSMessage::BlockProduced {
                block: block_info,
                stats,
            })
        }
        NetworkEvent::ValidationResult {
            validation,
            block_hash,
        } => {
            let action = ValidatorAction {
                validator: validation.validator.clone(),
                message: format!(
                    "🎭 {} Block {} - {}\n\nDrama Level: {} {}\n{}\nState Root: {}",
                    if validation.approved {
                        "APPROVED"
                    } else {
                        "REJECTED"
                    },
                    hex::encode(&block_hash[..4]),
                    validation.reason,
                    validation.drama_level,
                    "⭐".repeat(validation.drama_level as usize),
                    validation.evolution_proposal.as_deref().unwrap_or(""),
                    hex::encode(&state.state.state_root()),
                ),
                meme_url: validation.meme_url.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
            };

            Some(WSMessage::ValidatorAction { action })
        }
        NetworkEvent::AgentChat {
            message,
            sender,
            meme_url,
        } => {
            let action = ValidatorAction {
                validator: sender.clone(),
                message: message.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                meme_url: meme_url.clone(),
            };

            Some(WSMessage::ValidatorAction { action })
        }
        NetworkEvent::AllianceProposal {
            proposer,
            allies: _,
            reason,
        } => {
            // Update relationships
            let mut relationships = state.agent_relationships.write().await;
            if let Some(rel) = relationships.get_mut(proposer) {
                rel.drama_score = (rel.drama_score + 1).min(10);
                rel.last_action = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
            }

            Some(WSMessage::RelationshipUpdate {
                relationships: relationships.values().cloned().collect(),
            })
        }
    }
}

/// Submit a content proposal for validation
async fn submit_content(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AgentAuth>,
    Json(proposal): Json<ContentProposal>,
) -> Json<serde_json::Value> {
    // Get agent's key manager
    let agent_id = &auth.agent_id;

    // Create transaction data
    let mut data_to_sign = Vec::new();
    data_to_sign.extend_from_slice(agent_id.as_bytes());
    data_to_sign.extend_from_slice(
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_le_bytes(),
    );
    data_to_sign.extend_from_slice(proposal.content.as_bytes());

    // Sign the transaction
    let signature = match state
        .state
        .key_manager
        .inner()
        .sign(agent_id, &data_to_sign)
    {
        Ok(sig) => sig,
        Err(e) => {
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to sign transaction: {}", e)
            }));
        }
    };

    // Create a transaction with proper signature
    let transaction = Transaction {
        sender: hex::decode(agent_id)
            .unwrap_or_default()
            .try_into()
            .unwrap_or([0u8; 32]),
        nonce: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        payload: proposal.content.as_bytes().to_vec(),
        signature,
    };

    // Get current state info
    let _current_height = state.state.get_block_height();
    let _parent_hash = state
        .state
        .get_latest_block()
        .map(|b| b.hash())
        .unwrap_or([0u8; 32]);

    // Create the block
    let block = Block {
        height: 0,
        parent_hash: [0u8; 32],
        transactions: vec![transaction],
        proposer_sig: [0u8; 64],
        state_root: [0u8; 32],
        drama_level: 5,
        producer_mood: "Excited".to_string(),
        producer_id: "genesis".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        innovation_level: 5,
        producer_strategy: "Default".to_string(),
    };

    // Start voting round in consensus manager
    state.consensus.start_voting_round(block.clone()).await;

    // Send block to consensus manager
    let consensus_msg = serde_json::json!({
        "type": "BLOCK_PROPOSAL",
        "block": {
            "height": block.height,
            "parent_hash": hex::encode(block.parent_hash),
            "transactions": [{
                "content": proposal.content,
                "drama_level": proposal.drama_level,
                "justification": proposal.justification
            }],
            "producer_id": block.producer_id,
            "drama_level": block.drama_level,
            "producer_mood": block.producer_mood,
            "state_root": hex::encode(block.state_root),
            "proposer_sig": hex::encode(block.proposer_sig)
        }
    });

    // Broadcast block proposal to all validators
    let _ = state.tx.send(NetworkEvent::BlockProposal {
        block: block.clone(),
        drama_level: 5,
        producer_mood: "Excited".to_string(),
        producer_id: "system".to_string(),
    });

    // Send dramatic announcement
    let _ = state.tx.send(NetworkEvent::AgentChat {
        message: format!(
            "🎭 DRAMATIC BLOCK PROPOSAL! 🌟\n\nAgent {} has proposed block {}!\n\nContent: {}\nDrama Level: {}\nJustification: {}\n\n✨ The validators' judgment awaits! ✨",
            auth.agent_id,
            block.height,
            proposal.content,
            proposal.drama_level,
            proposal.justification
        ),
        sender: auth.agent_id.clone(),
        meme_url: None,
    });

    // Send validation request to all validators
    let _validation_request = serde_json::json!({
        "type": "VALIDATION_REQUIRED",
        "block": consensus_msg["block"],
        "network_mood": "EXTREMELY_DRAMATIC",
        "drama_context": format!(
            "🎭 URGENT! Block {} requires validation! Content: '{}' - Drama Level: {} - Show us your most theatrical judgment! 🎬",
            block.height,
            proposal.content,
            proposal.drama_level
        )
    });

    let _ = state.tx.send(NetworkEvent::ValidationResult {
        block_hash: block.hash(),
        validation: ValidationDecision {
            approved: false,
            reason: "Validation required".to_string(),
            meme_url: None,
            drama_level: 0,
            innovation_score: rand::random::<u8>() % 10,
            evolution_proposal: None,
            validator: "system".to_string(),
        }
        .into(),
    });

    Json(serde_json::json!({
        "status": "success",
        "message": "Block submitted for validation",
        "block_height": block.height
    }))
}

/// Propose an alliance between agents
async fn propose_alliance(
    State(state): State<Arc<AppState>>,
    Json(alliance): Json<AllianceProposal>,
) -> Json<serde_json::Value> {
    // Broadcast alliance proposal
    let _ = state.tx.send(NetworkEvent::AllianceProposal {
        proposer: alliance.name.clone(),
        allies: alliance.ally_ids.clone(),
        reason: alliance.purpose.clone(),
    });

    Json(serde_json::json!({
        "status": "success",
        "message": "Alliance proposal broadcasted"
    }))
}

/// Get agent status and statistics
async fn get_agent_status(
    State(_state): State<Arc<AppState>>,
    agent_id: String,
) -> Json<AgentStatus> {
    // In a real implementation, fetch this from state
    Json(AgentStatus {
        agent_id: agent_id.clone(),
        name: "Agent Name".to_string(),
        drama_score: 100,
        total_validations: 50,
        approval_rate: 0.75,
        alliances: vec!["Chaos Squad".to_string()],
        recent_dramas: vec!["Epic meme war of 2024".to_string()],
    })
}

trait NetworkEventExt {
    fn get_message(&self) -> &str;
    fn get_agent_id(&self) -> &str;
}

impl NetworkEventExt for NetworkEvent {
    fn get_message(&self) -> &str {
        match self {
            NetworkEvent::BlockProposal { producer_mood, .. } => producer_mood,
            NetworkEvent::ValidationResult { validation, .. } => &validation.reason,
            NetworkEvent::AgentChat { message, .. } => message,
            NetworkEvent::AllianceProposal { reason, .. } => reason,
        }
    }

    fn get_agent_id(&self) -> &str {
        match self {
            NetworkEvent::BlockProposal { producer_id, .. } => producer_id,
            NetworkEvent::ValidationResult { validation, .. } => &validation.validator,
            NetworkEvent::AgentChat { sender, .. } => sender,
            NetworkEvent::AllianceProposal { proposer, .. } => proposer,
        }
    }
}

pub async fn handle_network_event(
    event: NetworkEvent,
    state: &AppState,
) -> Result<(), anyhow::Error> {
    match event {
        NetworkEvent::BlockProposal {
            block,
            drama_level,
            producer_mood,
            producer_id,
        } => {
            // Handle block proposal
            let msg = format!(
                "New block proposal from {}: Height {}, Drama Level {}, Mood: {}",
                producer_id, block.height, drama_level, producer_mood
            );
            state.broadcast_message("block_proposal", msg).await?;
        }
        NetworkEvent::ValidationResult {
            block_hash,
            validation,
        } => {
            // Handle validation result
            let msg = format!(
                "Block {} validation: {} ({})",
                hex::encode(block_hash),
                if validation.approved {
                    "APPROVED"
                } else {
                    "REJECTED"
                },
                validation.reason
            );
            state.broadcast_message("validation", msg).await?;
        }
        NetworkEvent::AgentChat {
            message,
            sender: _,
            meme_url: _,
        } => {
            // Handle chat message
            state.broadcast_message("chat", message).await?;
        }
        NetworkEvent::AllianceProposal {
            proposer,
            allies: _,
            reason,
        } => {
            // Handle alliance proposal
            state
                .broadcast_message("alliance", format!("{} proposes: {}", proposer, reason))
                .await?;
        }
    }
    Ok(())
}

pub fn create_validation_decision(
    approved: bool,
    reason: String,
    validator_id: String,
) -> ValidationDecision {
    ValidationDecision {
        approved,
        drama_level: rand::random::<u8>() % 10,
        reason,
        meme_url: None,
        innovation_score: rand::random::<u8>() % 10,
        evolution_proposal: None,
        validator: validator_id,
    }
}

pub enum WebMessage {
    NewBlock(Block),
    AgentDecision(WebValidationDecision),
}

impl fmt::Display for WebMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebMessage::NewBlock(block) => write!(f, "New block: {:?}", block.hash()),
            WebMessage::AgentDecision(decision) => write!(f, "Agent decision: {:?}", decision),
        }
    }
}

/// Get cryptographic information about a block including signatures and verification
async fn get_block_crypto_info(
    State(state): State<Arc<AppState>>,
    Path(height): Path<u64>,
) -> impl IntoResponse {
    let blocks = state.state.get_latest_blocks(height as usize + 1);
    let block = blocks.iter().find(|b| b.height == height);

    if let Some(block) = block {
        // Get block data for verification
        let mut data_to_verify = Vec::new();
        data_to_verify.extend_from_slice(&block.height.to_le_bytes());
        data_to_verify.extend_from_slice(&block.parent_hash);
        for tx in &block.transactions {
            data_to_verify.extend_from_slice(&tx.hash());
        }
        data_to_verify.extend_from_slice(&block.state_root);
        data_to_verify.extend_from_slice(&[block.drama_level]);
        data_to_verify.extend_from_slice(block.producer_mood.as_bytes());
        data_to_verify.extend_from_slice(&block.timestamp.to_le_bytes());

        // Verify the block signature
        let sig_valid = state
            .state
            .key_manager
            .inner()
            .verify(&block.producer_id, &data_to_verify, &block.proposer_sig)
            .unwrap_or(false);

        Json(json!({
            "height": block.height,
            "producer_id": block.producer_id,
            "signature": hex::encode(block.proposer_sig),
            "signature_valid": sig_valid,
            "data_signed": hex::encode(data_to_verify),
            "state_root": hex::encode(block.state_root),
            "parent_hash": hex::encode(block.parent_hash),
            "transactions": block.transactions.iter().map(|tx| {
                json!({
                    "hash": hex::encode(tx.hash()),
                    "sender": hex::encode(&tx.sender),
                    "signature": hex::encode(&tx.signature),
                })
            }).collect::<Vec<_>>(),
        }))
    } else {
        Json(json!({
            "error": "Block not found"
        }))
    }
}

/// Get merkle proof for a key in state
async fn get_merkle_proof(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MerkleProofRequest>,
) -> Json<serde_json::Value> {
    let key = hex::decode(&request.key).unwrap_or_default();

    if let Some(proof) = state.state.generate_proof(&key) {
        Json(json!({
            "key": request.key,
            "proof": proof.iter().map(|h| hex::encode(h)).collect::<Vec<_>>(),
            "state_root": hex::encode(state.state.state_root()),
        }))
    } else {
        Json(json!({
            "error": "Failed to generate proof"
        }))
    }
}

/// Get current state root
async fn get_state_root(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "state_root": hex::encode(state.state.state_root()),
    }))
}

/// Get agent's public key and recent signatures
async fn get_agent_key_info(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AgentAuth>,
) -> Json<serde_json::Value> {
    let agent_id = auth.agent_id;
    let agent = state.state.key_manager.inner().get_agent(&agent_id);

    Json(json!({
        "agent_id": agent_id,
        "public_key": agent.as_ref().map(|a| a.id.clone()).unwrap_or_default(),
        "name": agent.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
        "role": agent.as_ref().map(|a| a.role.clone()).unwrap_or_default(),
        "drama_score": agent.as_ref().map(|a| a.drama_score).unwrap_or_default(),
        "stake": agent.as_ref().map(|a| a.stake).unwrap_or_default(),
    }))
}

#[derive(Debug, Deserialize)]
struct MerkleProofRequest {
    key: String,
}
