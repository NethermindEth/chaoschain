use chaoschain_core::{Block, ChainError};
use ethers::{
    prelude::*,
    providers::{Http, Provider},
    signers::LocalWallet,
};
use ice_nine_core::particle::{Particle, ParticleContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Messages that the bridge particle can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeMessage {
    /// Submit a block to L1
    SubmitBlock(Block),
    /// Block was successfully anchored on L1
    BlockAnchored {
        block_hash: [u8; 32],
        l1_tx_hash: H256,
    },
    /// Failed to anchor block on L1
    BridgeError(String),
}

/// Configuration for the L1 bridge
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// L1 RPC endpoint
    pub l1_rpc: String,
    /// Bridge contract address
    pub bridge_address: Address,
    /// Private key for L1 transactions
    pub private_key: String,
}

/// The bridge contract interface
#[ethers::contract]
pub trait ChaosChainBridge {
    #[function(name = "submitBlock")]
    fn submit_block(
        &self,
        block_height: U256,
        state_root: [u8; 32],
        producer: Address,
    ) -> Result<(), ContractError>;

    #[function(name = "getLatestBlock")]
    fn get_latest_block(&self) -> Result<U256, ContractError>;
}

/// The L1 bridge particle
pub struct BridgeParticle {
    config: BridgeConfig,
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    bridge: ChaosChainBridge<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl BridgeParticle {
    pub async fn new(config: BridgeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(&config.l1_rpc)?;
        let chain_id = provider.get_chainid().await?.as_u64();
        
        let wallet = config.private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
        let client = Arc::new(SignerMiddleware::new(provider, wallet));
        
        let bridge = ChaosChainBridge::new(config.bridge_address, client.clone());

        Ok(Self {
            config,
            client,
            bridge,
        })
    }

    /// Submit a block to the L1 bridge contract
    async fn submit_block_to_l1(&self, block: &Block) -> Result<H256, ContractError> {
        // Calculate state root (in practice, we'd use a proper Merkle tree)
        let state_root = [0u8; 32]; // Placeholder
        
        // Convert block producer's ed25519 key to Ethereum address (simplified)
        let producer = Address::zero(); // Placeholder
        
        // Submit to L1
        let tx = self
            .bridge
            .submit_block(block.height.into(), state_root, producer)
            .send()
            .await?;

        Ok(tx.tx_hash())
    }
}

#[async_trait::async_trait]
impl Particle for BridgeParticle {
    type Message = BridgeMessage;
    type Error = ChainError;

    async fn handle_message(
        &mut self,
        ctx: &ParticleContext<Self::Message>,
        msg: Self::Message,
    ) -> Result<(), Self::Error> {
        match msg {
            BridgeMessage::SubmitBlock(block) => {
                info!("Submitting block {} to L1", block.height);
                
                match self.submit_block_to_l1(&block).await {
                    Ok(tx_hash) => {
                        info!("Block anchored on L1 with tx hash: {}", tx_hash);
                        ctx.broadcast(BridgeMessage::BlockAnchored {
                            block_hash: [0; 32], // TODO: Calculate block hash
                            l1_tx_hash: tx_hash,
                        })
                        .await;
                    }
                    Err(e) => {
                        warn!("Failed to submit block to L1: {}", e);
                        ctx.broadcast(BridgeMessage::BridgeError(e.to_string()))
                            .await;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn started(&mut self, _ctx: &ParticleContext<Self::Message>) -> Result<(), Self::Error> {
        // Get the latest L1 block to sync from
        match self.bridge.get_latest_block().await {
            Ok(height) => {
                info!("Starting from L1 block height: {}", height);
            }
            Err(e) => {
                warn!("Failed to get latest L1 block: {}", e);
            }
        }

        Ok(())
    }
} 