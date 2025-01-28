use axum::{
    routing::get,
    Router, Json, extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, net::SocketAddr};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;
use anyhow::Result;
use tower_http::services::ServeDir;
use serde_json;
use chaoschain_core::{NetworkEvent, Block};
use chaoschain_state::StateStoreImpl;
use std::sync::RwLock;
use hex;
use std::collections::HashMap;
use chrono;

/// Web server state
pub struct AppState {
    /// Channel for network events
    pub tx: broadcast::Sender<NetworkEvent>,
    /// Chain state
    pub state: Arc<StateStoreImpl>,
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
#[derive(Clone, Debug, Serialize)]
pub struct BlockInfo {
    pub height: u64,
    pub producer: String,
    pub transaction_count: usize,
    pub validators: Vec<String>,
    pub timestamp: u64,
}

/// Start the web server
pub async fn start_web_server(tx: broadcast::Sender<NetworkEvent>, state: Arc<StateStoreImpl>) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = Arc::new(AppState {
        tx,
        state: state.clone(),
    });

    let app = Router::new()
        .route("/api/network/status", get(get_network_status))
        .route("/api/events", get(events_handler))
        .nest_service("/", ServeDir::new("static"))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Web server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Get network status including latest blocks
async fn get_network_status(
    State(state): State<Arc<AppState>>,
) -> Json<NetworkStatus> {
    let state_guard = state.state.clone();
    
    // Get chain state
    let chain_state = state_guard.get_state();
    
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
    
    Json(NetworkStatus {
        validator_count: 4, // We know we started with 4 validators
        producer_count: chain_state.producers.len() as u32,
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
            Ok(msg) => {
                let event_type = if msg.message.contains("DRAMATIC BLOCK PROPOSAL") {
                    "BlockProposal"
                } else if msg.message.contains("CONSENSUS") {
                    "Consensus"
                } else if msg.message.contains("APPROVES") || msg.message.contains("REJECTS") {
                    "Vote"
                } else {
                    "Drama"
                };

                let json = serde_json::json!({
                    "type": event_type,
                    "agent": msg.agent_id,
                    "message": msg.message,
                    "timestamp": chrono::Utc::now().timestamp(),
                });
                Event::default().data(json.to_string())
            }
            Err(_) => Event::default().data("error"),
        };
        Ok(event)
    });
    
    Sse::new(stream)
} 