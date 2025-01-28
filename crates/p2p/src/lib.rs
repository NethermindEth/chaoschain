use chaoschain_core::{Block, Transaction, NetworkMessage};
use libp2p::{
    core::transport::Transport,
    gossipsub::{
        self,
        Gossipsub,
        GossipsubConfigBuilder,
        GossipsubMessage,
        MessageAuthenticity,
        ValidationMode,
        Topic as GossipsubTopic,
        GossipsubEvent,
        IdentTopic,
    },
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviour, SwarmEvent},
    Swarm,
    PeerId,
    tcp,
    noise,
    yamux,
};
use libp2p_swarm_derive::NetworkBehaviour;
use serde::{Deserialize, Serialize};
use tracing::info;
use anyhow::Result;
use futures::StreamExt;
use thiserror::Error;
use std::error::Error as StdError;
use tokio::sync::mpsc;
use std::time::Duration;
use sha2::Sha256;

/// P2P message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Propose a new block
    BlockProposal(Block),
    /// Vote on a block proposal
    BlockVote {
        block_hash: [u8; 32],
        approve: bool,
        reason: String,
        meme_url: Option<String>,
    },
    /// Agent chat message
    Chat {
        message: String,
        mood: String,
        meme_url: Option<String>,
    },
    /// Broadcast a new transaction
    Transaction(Transaction),
}

/// P2P network configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Local peer ID
    pub peer_id: PeerId,
    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<String>,
    /// Port to listen on
    pub port: u16,
}

/// P2P network errors
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Fun message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    /// Standard block proposal
    BlockProposal(Block),
    /// Vote on a block
    Vote(BlockVote),
    /// Question about state diff
    WhyThisStateDiff {
        block_hash: [u8; 32],
        question: String,
    },
    /// Bribe attempt (for fun!)
    Bribe {
        block_hash: [u8; 32],
        offer: String,
        meme_base64: Option<String>,
    },
    /// Rejection with optional meme
    BlockRejectionMeme {
        block_hash: [u8; 32],
        reason: String,
        meme_base64: Option<String>,
    },
    /// General chat message
    Chat {
        message: String,
        reaction_emoji: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockVote {
    pub block_hash: [u8; 32],
    pub approve: bool,
    pub reason: String,
    pub meme_url: Option<String>,
}

/// Network topics for different message types
pub struct NetworkTopics {
    blocks: IdentTopic,
    transactions: IdentTopic,
    chat: IdentTopic,
}

impl NetworkTopics {
    pub fn new() -> Self {
        Self {
            blocks: IdentTopic::new("blocks"),
            transactions: IdentTopic::new("transactions"),
            chat: IdentTopic::new("chat"),
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct ChainNetworkBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

#[derive(Debug)]
pub enum OutEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
}

impl From<GossipsubEvent> for OutEvent {
    fn from(event: GossipsubEvent) -> Self {
        OutEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for OutEvent {
    fn from(event: MdnsEvent) -> Self {
        OutEvent::Mdns(event)
    }
}

/// P2P network manager
pub struct Network {
    swarm: Swarm<ChainNetworkBehaviour>,
    topics: NetworkTopics,
}

impl Network {
    pub async fn new() -> Result<Self> {
        let id_keys = Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());
        info!("Local peer id: {peer_id}");

        // Create transport
        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&id_keys)
            .expect("Signing libp2p-noise static DH keypair failed.");

        let transport = tcp::TcpConfig::new()
            .nodelay(true)
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(yamux::YamuxConfig::default())
            .boxed();

        // Create gossipsub
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Permissive)
            .build()
            .map_err(|msg| anyhow::anyhow!("Failed to build gossipsub config: {msg}"))?;

        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Anonymous,
            gossipsub_config,
        ).map_err(|msg| anyhow::anyhow!("Failed to create gossipsub: {msg}"))?;

        // Create MDNS
        let mdns = Mdns::new(Default::default()).await?;

        // Create behaviour
        let behaviour = ChainNetworkBehaviour {
            gossipsub,
            mdns,
        };

        // Create swarm
        let swarm = Swarm::new(transport, behaviour, peer_id);

        let topics = NetworkTopics::new();

        Ok(Self { swarm, topics })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // Subscribe to topics
        self.swarm.behaviour_mut().gossipsub.subscribe(&self.topics.blocks)?;
        self.swarm.behaviour_mut().gossipsub.subscribe(&self.topics.transactions)?;
        self.swarm.behaviour_mut().gossipsub.subscribe(&self.topics.chat)?;

        loop {
            match self.swarm.next().await.expect("Swarm stream is infinite") {
                SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message { 
                    message: GossipsubMessage { data, .. },
                    ..
                })) => {
                    let msg: NetworkMessage = serde_json::from_slice(&data)?;
                    match msg {
                        NetworkMessage::NewBlock(block) => {
                            info!("Received new block: {:?}", block);
                        }
                        NetworkMessage::NewTransaction(tx) => {
                            info!("Received new transaction: {:?}", tx);
                        }
                        NetworkMessage::Chat { from, message } => {
                            info!("Chat from {}: {}", from, message);
                        }
                        NetworkMessage::AgentReasoning { agent, reasoning } => {
                            info!("Agent {} reasoning: {}", agent, reasoning);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub async fn broadcast(&mut self, message: NetworkMessage) -> Result<()> {
        let data = serde_json::to_vec(&message)?;
        
        match message {
            NetworkMessage::NewBlock(_) => {
                self.swarm.behaviour_mut().gossipsub.publish(
                    self.topics.blocks.clone(),
                    data,
                )?;
            }
            NetworkMessage::NewTransaction(_) => {
                self.swarm.behaviour_mut().gossipsub.publish(
                    self.topics.transactions.clone(),
                    data,
                )?;
            }
            NetworkMessage::Chat { .. } | NetworkMessage::AgentReasoning { .. } => {
                self.swarm.behaviour_mut().gossipsub.publish(
                    self.topics.chat.clone(),
                    data,
                )?;
            }
        }

        Ok(())
    }
}