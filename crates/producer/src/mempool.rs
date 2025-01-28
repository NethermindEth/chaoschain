use chaoschain_core::{Transaction, ChainError};
use ice_nine_core::particle::{Particle, ParticleContext};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Messages that the mempool particle can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MempoolMessage {
    /// Submit a new transaction
    SubmitTransaction(Transaction),
    /// Request best transactions for a block
    RequestTransactions {
        max_count: usize,
        max_size: usize,
    },
    /// Transactions were included in a block
    TransactionsIncluded(Vec<Transaction>),
}

/// The mempool particle
pub struct MempoolParticle {
    /// The actual mempool
    mempool: chaoschain_core::mempool::Mempool,
}

impl MempoolParticle {
    pub fn new(max_size: usize) -> Self {
        Self {
            mempool: chaoschain_core::mempool::Mempool::new(max_size),
        }
    }
}

#[async_trait::async_trait]
impl Particle for MempoolParticle {
    type Message = MempoolMessage;
    type Error = ChainError;

    async fn handle_message(
        &mut self,
        ctx: &ParticleContext<Self::Message>,
        msg: Self::Message,
    ) -> Result<(), Self::Error> {
        match msg {
            MempoolMessage::SubmitTransaction(tx) => {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if let Err(e) = self.mempool.add_transaction(tx.clone(), current_time) {
                    warn!("Failed to add transaction to mempool: {}", e);
                } else {
                    info!("Added transaction to mempool");
                }
            }
            MempoolMessage::RequestTransactions { max_count, max_size: _ } => {
                // For now, we ignore max_size and just return max_count transactions
                let transactions = self.mempool.get_top_transactions(max_count);
                info!("Returning {} transactions from mempool", transactions.len());
                
                // TODO: Send these transactions to the requesting particle
            }
            MempoolMessage::TransactionsIncluded(txs) => {
                self.mempool.remove_transactions(&txs);
                info!("Removed {} included transactions from mempool", txs.len());
            }
        }

        Ok(())
    }
} 