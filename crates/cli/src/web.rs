use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use ui9_dui::Html;
use ui9_dui::{
    component::{Component, Context},
    event::Event,
    view::{View, ViewBuilder},
};
use ui9_dui::{html, Html, Hub, View};

const MAX_DRAMA_LOG: usize = 100;

/// Web interface state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebState {
    /// Active agents and their personalities
    pub agents: Vec<(String, String)>,
    /// Recent drama events
    pub drama_log: Vec<String>,
    /// Current block height
    pub block_height: u64,
    /// Active meme wars
    pub meme_wars: Vec<(String, String, String)>,
    /// Current alliances
    pub alliances: Vec<(String, String, String)>,
    /// Blocks
    pub blocks: Vec<(u64, String, bool, String)>,
}

/// Current consensus state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Validators and their current votes
    pub validator_votes: HashMap<String, bool>,
    /// Current drama level
    pub drama_level: u8,
    /// Active alliances
    pub alliances: Vec<(String, String)>,
    /// Active feuds
    pub feuds: Vec<(String, String)>,
}

/// Web interface messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebMessage {
    /// New block produced
    Block(u64),
    /// User interaction
    UserAction(UserAction),
    /// New agent connected
    AgentConnected { name: String, personality: String },
    /// Drama event occurred
    DramaEvent(String),
    /// Meme war started
    MemeWarStarted {
        initiator: String,
        target: String,
        meme: String,
    },
    /// Alliance formed
    AllianceFormed {
        agent1: String,
        agent2: String,
        reason: String,
    },
    /// Block produced
    BlockProduced {
        producer: String,
        height: u64,
        transactions: usize,
        drama_level: u32,
    },
    /// Block validated
    BlockValidated {
        validator: String,
        height: u64,
        approved: bool,
        reason: String,
    },
}

/// User actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserAction {
    /// Send a transaction
    SendTransaction { payload: Vec<u8>, drama_level: u8 },
    /// Start drama between agents
    StartDrama {
        instigator: String,
        target: String,
        reason: String,
    },
    /// Propose alliance
    ProposeAlliance {
        from: String,
        to: String,
        terms: String,
    },
}

/// Web interface component
#[derive(Clone)]
pub struct WebInterface {
    /// Current state
    state: Arc<Mutex<WebState>>,
    /// Channel to send messages to agents
    agent_tx: mpsc::Sender<WebMessage>,
}

impl WebInterface {
    pub fn new(agent_tx: mpsc::Sender<WebMessage>) -> Self {
        Self {
            state: Arc::new(Mutex::new(WebState::default())),
            agent_tx,
        }
    }

    pub async fn update(&self, msg: WebMessage) {
        let mut state = self.state.lock().await;
        match msg {
            WebMessage::Block(height) => {
                state.block_height = height;
            }
            WebMessage::UserAction(action) => {
                let _ = self.agent_tx.try_send(WebMessage::UserAction(action));
            }
            WebMessage::AgentConnected { name, personality } => {
                state.agents.push((name, personality));
            }
            WebMessage::DramaEvent(event) => {
                state.drama_log.push(event);
                if state.drama_log.len() > MAX_DRAMA_LOG {
                    state.drama_log.remove(0);
                }
            }
            WebMessage::MemeWarStarted {
                initiator,
                target,
                meme,
            } => {
                state.meme_wars.push((initiator, target, meme));
            }
            WebMessage::AllianceFormed {
                agent1,
                agent2,
                reason,
            } => {
                state.alliances.push((agent1, agent2, reason));
            }
            WebMessage::BlockProduced {
                producer,
                height,
                transactions: _,
                drama_level: _,
            } => {
                state.blocks.push((height, producer, false, String::new()));
            }
            WebMessage::BlockValidated {
                validator: _,
                height,
                approved,
                reason,
            } => {
                if let Some(block) = state.blocks.iter_mut().find(|(h, _, _, _)| *h == height) {
                    block.2 = approved;
                    block.3 = reason;
                }
            }
        }
    }
}

impl Component for WebInterface {
    type Message = WebMessage;

    fn update(&mut self, msg: Self::Message, _ctx: &mut Context<Self>) {
        match msg {
            WebMessage::Block(height) => {
                self.state.lock().await.block_height = height;
            }
            WebMessage::UserAction(action) => {
                let _ = self.agent_tx.try_send(WebMessage::UserAction(action));
            }
            WebMessage::AgentConnected { name, personality } => {
                self.state.lock().await.agents.push((name, personality));
            }
            WebMessage::DramaEvent(event) => {
                self.state.lock().await.drama_log.push(event);
                if self.state.lock().await.drama_log.len() > MAX_DRAMA_LOG {
                    self.state.lock().await.drama_log.remove(0);
                }
            }
            WebMessage::MemeWarStarted {
                initiator,
                target,
                meme,
            } => {
                self.state
                    .lock()
                    .await
                    .meme_wars
                    .push((initiator, target, meme));
            }
            WebMessage::AllianceFormed {
                agent1,
                agent2,
                reason,
            } => {
                self.state
                    .lock()
                    .await
                    .alliances
                    .push((agent1, agent2, reason));
            }
        }
    }

    fn view(&self) -> View {
        let state = self.state.lock().await;

        ViewBuilder::new()
            .title("ChaosChain Demo")
            .child(
                ViewBuilder::new()
                    .title("Connected Agents")
                    .list(state.agents.iter().map(|(name, personality)| {
                        ViewBuilder::new()
                            .text(format!("{} - {}", name, personality))
                            .build()
                    }))
                    .build(),
            )
            .child(
                ViewBuilder::new()
                    .title(format!("Drama Log (Block Height: {})", state.block_height))
                    .list(
                        state
                            .drama_log
                            .iter()
                            .map(|event| ViewBuilder::new().text(event).build()),
                    )
                    .build(),
            )
            .child(
                ViewBuilder::new()
                    .title("Active Meme Wars")
                    .list(state.meme_wars.iter().map(|(initiator, target, meme)| {
                        ViewBuilder::new()
                            .text(format!("{} vs {} - {}", initiator, target, meme))
                            .build()
                    }))
                    .build(),
            )
            .child(
                ViewBuilder::new()
                    .title("Current Alliances")
                    .list(state.alliances.iter().map(|(a1, a2, reason)| {
                        ViewBuilder::new()
                            .text(format!("{} & {} - {}", a1, a2, reason))
                            .build()
                    }))
                    .build(),
            )
            .child(
                ViewBuilder::new()
                    .title("Recent Blocks")
                    .list(
                        state
                            .blocks
                            .iter()
                            .map(|(height, producer, approved, reason)| {
                                ViewBuilder::new()
                                    .text(format!(
                                        "Block {} by {} - {}",
                                        height,
                                        producer,
                                        if *approved {
                                            format!("Approved: {}", reason)
                                        } else {
                                            "Pending validation".to_string()
                                        }
                                    ))
                                    .build()
                            }),
                    )
                    .build(),
            )
            .build()
    }
}

impl WebInterface {
    fn render_consensus(&self) -> String {
        format!(
            r#"
            <div class="consensus-state">
                <div class="drama-meter">
                    Drama Level: {}%
                    <div class="meter" style="width: {}%"></div>
                </div>
                
                <div class="validator-votes">
                    {}
                </div>
                
                <div class="relationships">
                    <div class="alliances">
                        <h3>Alliances</h3>
                        {}
                    </div>
                    <div class="feuds">
                        <h3>Feuds</h3>
                        {}
                    </div>
                </div>
            </div>
        "#,
            self.state.lock().await.consensus_state.drama_level,
            self.state.lock().await.consensus_state.drama_level,
            // Render validator votes
            self.state
                .lock()
                .await
                .consensus_state
                .validator_votes
                .iter()
                .map(|(validator, vote)| format!(
                    r#"<div class="vote">
                        <span>{}</span>: {}
                    </div>"#,
                    validator,
                    if *vote { "✅" } else { "❌" }
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            // Render alliances
            self.state
                .lock()
                .await
                .consensus_state
                .alliances
                .iter()
                .map(|(a, b)| format!("{} 🤝 {}", a, b))
                .collect::<Vec<_>>()
                .join("<br>"),
            // Render feuds
            self.state
                .lock()
                .await
                .consensus_state
                .feuds
                .iter()
                .map(|(a, b)| format!("{} ⚔️ {}", a, b))
                .collect::<Vec<_>>()
                .join("<br>")
        )
    }

    fn render_actions(&self) -> String {
        r#"
            <div class="user-actions">
                <button onclick="sendTransaction()">Send Transaction</button>
                <button onclick="startDrama()">Start Drama</button>
                <button onclick="proposeAlliance()">Propose Alliance</button>
            </div>
            
            <script>
                function sendTransaction() {
                    const payload = prompt("Enter transaction message:");
                    if (payload) {
                        const dramaLevel = Math.floor(Math.random() * 100);
                        window.ice.send({
                            type: "UserAction",
                            action: {
                                type: "SendTransaction",
                                payload: new TextEncoder().encode(payload),
                                drama_level: dramaLevel
                            }
                        });
                    }
                }
                
                function startDrama() {
                    const agents = Object.keys(window.state.agents);
                    const instigator = agents[Math.floor(Math.random() * agents.length)];
                    const target = agents[Math.floor(Math.random() * agents.length)];
                    const reason = prompt("Why start drama?");
                    
                    if (reason) {
                        window.ice.send({
                            type: "UserAction",
                            action: {
                                type: "StartDrama",
                                instigator,
                                target,
                                reason
                            }
                        });
                    }
                }
                
                function proposeAlliance() {
                    const agents = Object.keys(window.state.agents);
                    const from = agents[Math.floor(Math.random() * agents.length)];
                    const to = agents[Math.floor(Math.random() * agents.length)];
                    const terms = prompt("Propose alliance terms:");
                    
                    if (terms) {
                        window.ice.send({
                            type: "UserAction",
                            action: {
                                type: "ProposeAlliance",
                                from,
                                to,
                                terms
                            }
                        });
                    }
                }
            </script>
            
            <style>
                .chaoschain-demo {
                    max-width: 800px;
                    margin: 0 auto;
                    padding: 20px;
                    font-family: system-ui, sans-serif;
                }
                
                .agents, .drama-log, .consensus, .actions {
                    margin: 20px 0;
                    padding: 20px;
                    border: 1px solid #ddd;
                    border-radius: 8px;
                }
                
                .drama-event {
                    padding: 10px;
                    margin: 5px 0;
                    background: #f5f5f5;
                    border-radius: 4px;
                }
                
                .drama-meter {
                    margin: 10px 0;
                }
                
                .meter {
                    height: 20px;
                    background: linear-gradient(90deg, #ff6b6b, #ffd93d);
                    border-radius: 10px;
                }
                
                .vote {
                    display: inline-block;
                    margin: 5px;
                    padding: 5px 10px;
                    background: #f0f0f0;
                    border-radius: 15px;
                }
                
                .user-actions button {
                    margin: 5px;
                    padding: 10px 20px;
                    border: none;
                    border-radius: 20px;
                    background: #4a90e2;
                    color: white;
                    cursor: pointer;
                    transition: background 0.2s;
                }
                
                .user-actions button:hover {
                    background: #357abd;
                }
            </style>
        "#
        .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemeWar {
    pub instigator: String,
    pub target: String,
    pub memes_used: Vec<String>,
    pub intensity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alliance {
    pub members: Vec<String>,
    pub reason: String,
    pub strength: u8,
}

#[derive(Default)]
struct WebState {
    agents: Vec<(String, String)>, // (name, personality)
    drama_log: Vec<String>,
    meme_wars: Vec<(String, String, String)>, // (initiator, target, meme)
    alliances: Vec<(String, String, String)>, // (agent1, agent2, reason)
    blocks: Vec<(u64, String, bool, String)>, // (height, producer, approved, reason)
}
