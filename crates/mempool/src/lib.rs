use chaoschain_core::Transaction;
use hex::encode;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct TransactionProposal {
    pub transaction: Transaction,
    pub proposer: String,
    pub justification: String,
    pub drama_score: u8,
    pub timestamp: u64,
    pub discussions: Vec<TransactionDiscussion>,
    pub proposed_order: Option<usize>,
    pub alliances_in_favor: Vec<String>,
    pub alliances_against: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionDiscussion {
    pub agent: String,
    pub opinion: String,
    pub reasoning: String,
    pub proposed_position: Option<usize>,
    pub alliances: Vec<String>,
    pub drama_score: u8,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct Mempool {
    transactions: Arc<RwLock<HashMap<[u8; 32], TransactionProposal>>>,
    ordering_discussions: Arc<RwLock<Vec<OrderingDiscussion>>>,
    max_size: usize,
}

#[derive(Debug, Clone)]
pub struct OrderingDiscussion {
    pub agent: String,
    pub proposed_order: Vec<[u8; 32]>, // Transaction hashes in proposed order
    pub reasoning: String,
    pub drama_score: u8,
    pub timestamp: u64,
    pub supporters: Vec<String>,
    pub opposers: Vec<String>,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            ordering_discussions: Arc::new(RwLock::new(Vec::new())),
            max_size,
        }
    }

    pub async fn add_transaction(&self, tx: Transaction) -> bool {
        let mut transactions = self.transactions.write().await;
        if transactions.len() >= self.max_size {
            return false;
        }

        let hash = tx.hash();
        let mut rng = rand::thread_rng();

        // Generate random drama score between 1 and 10
        let drama_score = rng.gen_range(1..=10);

        // Generate dramatic justification based on transaction properties
        let justification = match drama_score {
            1..=3 => format!("A whisper of chaos in the mempool! Transaction 0x{} arrives with subtle dramatic undertones...", 
                encode(&tx.hash()[..4])),
            4..=6 => format!("The mempool stirs with intrigue! Transaction 0x{} brings a tale of moderate drama!", 
                encode(&tx.hash()[..4])),
            7..=9 => format!("DRAMATIC ENTRANCE! Transaction 0x{} crashes into the mempool with theatrical flair!", 
                encode(&tx.hash()[..4])),
            _ => format!("🎭 MAXIMUM DRAMA ALERT! Transaction 0x{} has arrived to revolutionize the entire blockchain narrative!", 
                encode(&tx.hash()[..4])),
        };

        let proposal = TransactionProposal {
            transaction: tx,
            proposer: format!("agent_{}", encode(&rand::random::<[u8; 8]>())),
            justification,
            drama_score,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            discussions: Vec::new(),
            proposed_order: None,
            alliances_in_favor: Vec::new(),
            alliances_against: Vec::new(),
        };

        transactions.insert(hash, proposal);
        true
    }

    pub async fn propose_transaction(&self, proposal: TransactionProposal) -> bool {
        let mut transactions = self.transactions.write().await;
        if transactions.len() >= self.max_size {
            return false;
        }

        let hash = proposal.transaction.hash();
        transactions.insert(hash, proposal);
        true
    }

    pub async fn add_discussion(
        &self,
        tx_hash: &[u8; 32],
        discussion: TransactionDiscussion,
    ) -> bool {
        let mut transactions = self.transactions.write().await;
        if let Some(proposal) = transactions.get_mut(tx_hash) {
            proposal.discussions.push(discussion);
            true
        } else {
            false
        }
    }

    pub async fn propose_ordering(&self, discussion: OrderingDiscussion) {
        let mut ordering_discussions = self.ordering_discussions.write().await;
        ordering_discussions.push(discussion);
    }

    pub async fn get_top(&self, limit: usize) -> Vec<Transaction> {
        let transactions = self.transactions.read().await;
        let mut proposals: Vec<_> = transactions.values().cloned().collect();

        // Sort by drama score and support
        proposals.sort_by(|a, b| {
            let a_score = a.drama_score as f32 + (a.alliances_in_favor.len() as f32 * 0.5);
            let b_score = b.drama_score as f32 + (b.alliances_in_favor.len() as f32 * 0.5);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        proposals
            .into_iter()
            .take(limit)
            .map(|p| p.transaction)
            .collect()
    }

    pub async fn get_proposals_for_transaction(
        &self,
        tx_hash: &[u8; 32],
    ) -> Option<TransactionProposal> {
        let transactions = self.transactions.read().await;
        transactions.get(tx_hash).cloned()
    }

    pub async fn get_ordering_discussions(&self) -> Vec<OrderingDiscussion> {
        let discussions = self.ordering_discussions.read().await;
        discussions.clone()
    }

    pub async fn get_mempool_stats(&self) -> MempoolStats {
        let transactions = self.transactions.read().await;
        let _discussions = self.ordering_discussions.read().await; // Prefix with _ since we're not using it yet

        let total_transactions = transactions.len();
        let avg_drama_score = if total_transactions > 0 {
            transactions
                .values()
                .map(|p| p.drama_score as f32)
                .sum::<f32>()
                / total_transactions as f32
        } else {
            0.0
        };

        // Find hot topics based on discussion volume
        let mut topic_counts = HashMap::new();
        for proposal in transactions.values() {
            for discussion in &proposal.discussions {
                *topic_counts
                    .entry(discussion.reasoning.clone())
                    .or_insert(0) += 1;
            }
        }

        let mut hot_topics: Vec<_> = topic_counts.into_iter().collect();
        hot_topics.sort_by(|a, b| b.1.cmp(&a.1));
        let hot_topics = hot_topics
            .into_iter()
            .take(5)
            .map(|(topic, _)| topic)
            .collect();

        MempoolStats {
            total_transactions,
            avg_drama_score,
            hot_topics,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolStats {
    pub total_transactions: usize,
    pub avg_drama_score: f32,
    pub hot_topics: Vec<String>,
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
