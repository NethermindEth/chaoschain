use anyhow::Result;
use chaoschain_core::{Block, NetworkMessage, Transaction};
use futures::StreamExt;
use libp2p::{
    core::upgrade,
    gossipsub::{
        self, Event as GossipsubEvent, IdentTopic as Topic, MessageAuthenticity, ValidationMode,
    },
    identify,
    identity::Keypair,
    mdns, noise, ping,
    swarm::{Config as SwarmConfig, NetworkBehaviour, SwarmEvent},
    tcp, yamux, PeerId, Swarm, Transport,
};
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;

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
    blocks: Topic,
    transactions: Topic,
    chat: Topic,
}

impl NetworkTopics {
    pub fn new() -> Self {
        Self {
            blocks: Topic::new(BLOCK_TOPIC),
            transactions: Topic::new(TX_TOPIC),
            chat: Topic::new(CHAT_TOPIC),
        }
    }

    pub fn subscribe_all(&self, swarm: &mut Swarm<ChainBehaviour>) {
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.blocks)
            .unwrap();
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.transactions)
            .unwrap();
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.chat)
            .unwrap();
    }
}

impl Default for NetworkTopics {
    fn default() -> Self {
        Self::new()
    }
}

const BLOCK_TOPIC: &str = "blocks";
const TX_TOPIC: &str = "transactions";
const CHAT_TOPIC: &str = "chat";

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ChainBehaviourEvent")]
pub struct ChainBehaviour {
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum ChainBehaviourEvent {
    Gossipsub(GossipsubEvent),
    Identify(identify::Event),
    Ping(ping::Event),
    Mdns(mdns::Event),
}

impl From<GossipsubEvent> for ChainBehaviourEvent {
    fn from(event: GossipsubEvent) -> Self {
        ChainBehaviourEvent::Gossipsub(event)
    }
}

impl From<identify::Event> for ChainBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        ChainBehaviourEvent::Identify(event)
    }
}

impl From<ping::Event> for ChainBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        ChainBehaviourEvent::Ping(event)
    }
}

impl From<mdns::Event> for ChainBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        ChainBehaviourEvent::Mdns(event)
    }
}

pub struct ChainNetwork {
    swarm: Swarm<ChainBehaviour>,
    topics: NetworkTopics,
    event_sender: mpsc::Sender<NetworkMessage>,
}

impl ChainNetwork {
    pub async fn new(
        keypair: Keypair,
        event_sender: mpsc::Sender<NetworkMessage>,
    ) -> Result<Self, Box<dyn StdError>> {
        let peer_id = PeerId::from(keypair.public());
        let topics = NetworkTopics::default();

        // Set up gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .build()
            .expect("Valid config");

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )?;

        // Set up other behaviours
        let identify = identify::Behaviour::new(identify::Config::new(
            "chaoschain/1.0.0".to_string(),
            keypair.public(),
        ));
        let ping = ping::Behaviour::new(ping::Config::new());
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;

        // Create network behaviour
        let behaviour = ChainBehaviour {
            gossipsub,
            identify,
            ping,
            mdns,
        };

        // Set up transport
        let transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(
                noise::Config::new(&keypair)
                    .expect("signing libp2p-noise static DH keypair failed"),
            )
            .multiplex(yamux::Config::default())
            .boxed();

        // Build the swarm
        let swarm = Swarm::new(
            transport,
            behaviour,
            peer_id,
            SwarmConfig::with_tokio_executor(),
        );

        Ok(Self {
            swarm,
            topics,
            event_sender,
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn StdError>> {
        // Listen on all interfaces
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // Subscribe to topics
        self.topics.subscribe_all(&mut self.swarm);

        loop {
            match self.swarm.next().await.expect("Swarm stream is infinite") {
                SwarmEvent::Behaviour(ChainBehaviourEvent::Gossipsub(
                    GossipsubEvent::Message { message, .. },
                )) => {
                    let msg: NetworkMessage = serde_json::from_slice(&message.data)?;
                    self.event_sender.send(msg).await?;
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {:?}", address);
                }
                _ => {}
            }
        }
    }

    pub async fn broadcast_block(&mut self, block: Block) -> Result<(), Box<dyn StdError>> {
        let msg = NetworkMessage::NewBlock(block);
        let data = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topics.blocks.clone(), data)?;
        Ok(())
    }

    pub async fn broadcast_transaction(
        &mut self,
        tx: Transaction,
    ) -> Result<(), Box<dyn StdError>> {
        let msg = NetworkMessage::NewTransaction(tx);
        let data = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topics.transactions.clone(), data)?;
        Ok(())
    }

    pub async fn broadcast_chat(
        &mut self,
        from: String,
        message: String,
    ) -> Result<(), Box<dyn StdError>> {
        let msg = NetworkMessage::Chat { from, message };
        let data = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topics.chat.clone(), data)?;
        Ok(())
    }
}
