use chaoschain_core::{Block, Transaction, ChainError};
use ice_nine_core::particle::{Particle, ParticleContext};
use libp2p::{
    gossipsub::{self, Gossipsub, GossipsubEvent, MessageAuthenticity, ValidationMode},
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Topics for different message types
#[derive(Clone)]
pub struct NetworkTopics {
    blocks: gossipsub::Topic,
    transactions: gossipsub::Topic,
    chat: gossipsub::Topic,
}

impl NetworkTopics {
    pub fn new() -> Self {
        Self {
            blocks: gossipsub::Topic::new("blocks"),
            transactions: gossipsub::Topic::new("transactions"),
            chat: gossipsub::Topic::new("chat"),
        }
    }
}

/// Messages that can be sent over the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// New block announcement
    NewBlock(Block),
    /// New transaction announcement
    NewTransaction(Transaction),
    /// Chat message between agents
    Chat {
        from: String,
        message: String,
        /// Optional meme/image in base64
        attachment: Option<String>,
    },
    /// Agent's reasoning for a decision
    Reasoning {
        block_hash: [u8; 32],
        reason: String,
        confidence: f64,
    },
}

/// Combined network behavior
#[derive(NetworkBehaviour)]
struct ChainNetworkBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

/// The network particle
pub struct NetworkParticle {
    swarm: libp2p::Swarm<ChainNetworkBehaviour>,
    topics: NetworkTopics,
}

impl NetworkParticle {
    pub async fn new(keypair: Keypair) -> Result<Self, Box<dyn Error>> {
        let peer_id = PeerId::from(keypair.public());
        info!("Local peer id: {peer_id}");

        // Set up gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .validation_mode(ValidationMode::Permissive) // Be chaotic!
            .build()
            .map_err(|e| format!("Failed to build gossipsub config: {e}"))?;

        let mut behaviour = ChainNetworkBehaviour {
            gossipsub: Gossipsub::new(MessageAuthenticity::Signed(keypair.clone()), gossipsub_config)?,
            mdns: Mdns::new(Default::default()).await?,
        };

        // Subscribe to topics
        let topics = NetworkTopics::new();
        behaviour.gossipsub.subscribe(&topics.blocks)?;
        behaviour.gossipsub.subscribe(&topics.transactions)?;
        behaviour.gossipsub.subscribe(&topics.chat)?;

        let config = libp2p::SwarmConfig::default();
        let swarm = libp2p::Swarm::new(behaviour, config);

        Ok(Self { swarm, topics })
    }

    /// Broadcast a message to the appropriate topic
    async fn broadcast(&mut self, message: NetworkMessage) -> Result<(), Box<dyn Error>> {
        let (topic, encoded) = match &message {
            NetworkMessage::NewBlock(_) => (&self.topics.blocks, serde_json::to_string(&message)?),
            NetworkMessage::NewTransaction(_) => (
                &self.topics.transactions,
                serde_json::to_string(&message)?,
            ),
            _ => (&self.topics.chat, serde_json::to_string(&message)?),
        };

        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), encoded.as_bytes())?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Particle for NetworkParticle {
    type Message = NetworkMessage;
    type Error = ChainError;

    async fn handle_message(
        &mut self,
        ctx: &ParticleContext<Self::Message>,
        msg: Self::Message,
    ) -> Result<(), Self::Error> {
        // Broadcast the message to the network
        if let Err(e) = self.broadcast(msg.clone()).await {
            warn!("Failed to broadcast message: {}", e);
            return Ok(());
        }

        // Handle incoming network events
        match msg {
            NetworkMessage::NewBlock(block) => {
                info!("Broadcasting new block at height {}", block.height);
            }
            NetworkMessage::NewTransaction(tx) => {
                info!("Broadcasting new transaction");
            }
            NetworkMessage::Chat { from, message, .. } => {
                info!("Chat message from {}: {}", from, message);
            }
            NetworkMessage::Reasoning {
                block_hash,
                reason,
                confidence,
            } => {
                info!(
                    "Agent reasoning for block {}: {} (confidence: {})",
                    hex::encode(block_hash),
                    reason,
                    confidence
                );
            }
        }

        Ok(())
    }

    async fn started(&mut self, ctx: &ParticleContext<Self::Message>) -> Result<(), Self::Error> {
        // Start network event loop
        tokio::spawn({
            let mut swarm = self.swarm.clone();
            let ctx = ctx.clone();
            async move {
                loop {
                    match swarm.next_event().await {
                        SwarmEvent::Behaviour(behaviour) => match behaviour {
                            ChainNetworkBehaviourEvent::Gossipsub(GossipsubEvent::Message {
                                message,
                                ..
                            }) => {
                                if let Ok(msg) = serde_json::from_slice::<NetworkMessage>(&message.data)
                                {
                                    if let Err(e) = ctx.send(msg).await {
                                        warn!("Failed to forward network message: {}", e);
                                    }
                                }
                            }
                            ChainNetworkBehaviourEvent::Mdns(MdnsEvent::Discovered(peers)) => {
                                for (peer, _) in peers {
                                    info!("Discovered peer via mDNS: {peer}");
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        });

        Ok(())
    }
} 