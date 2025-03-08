use crate::{Error, Transaction};
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

/// A transaction in the mempool with priority
#[derive(Debug, Clone)]
pub struct MempoolTx {
    /// The actual transaction
    pub transaction: Transaction,
    /// Time added to mempool
    pub timestamp: u64,
    /// Priority score (higher = more priority)
    pub priority: u64,
}

impl PartialEq for MempoolTx {
    fn eq(&self, other: &Self) -> bool {
        self.transaction == other.transaction
    }
}

impl Eq for MempoolTx {}

impl PartialOrd for MempoolTx {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MempoolTx {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority comes first
        self.priority.cmp(&other.priority).reverse()
    }
}

/// Thread-safe mempool
#[derive(Clone)]
pub struct Mempool {
    /// Transactions by hash
    txs: Arc<RwLock<HashMap<[u8; 32], MempoolTx>>>,
    /// Priority queue for ordering
    queue: Arc<RwLock<BinaryHeap<MempoolTx>>>,
    /// Maximum number of transactions
    max_size: usize,
}

impl Mempool {
    /// Create a new mempool
    pub fn new(max_size: usize) -> Self {
        Self {
            txs: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            max_size,
        }
    }

    /// Add a transaction to the mempool
    pub fn add_tx(&self, tx: Transaction, priority: u64) -> Result<(), Error> {
        let tx_hash = self.hash_tx(&tx);
        let mempool_tx = MempoolTx {
            transaction: tx,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            priority,
        };

        // Check if we already have this transaction
        let mut txs = self.txs.write();
        if txs.contains_key(&tx_hash) {
            return Ok(());
        }

        // Add to mempool if there's space
        if txs.len() >= self.max_size {
            return Err(Error::StateError("Mempool is full".to_string()));
        }

        txs.insert(tx_hash, mempool_tx.clone());
        self.queue.write().push(mempool_tx);

        Ok(())
    }

    /// Get the top N transactions by priority
    pub fn get_top(&self, n: usize) -> Vec<Transaction> {
        let txs = self.txs.read();
        let queue = self.queue.read();

        queue
            .iter()
            .take(n)
            .filter(|tx| txs.contains_key(&self.hash_tx(&tx.transaction)))
            .map(|tx| tx.transaction.clone())
            .collect()
    }

    /// Remove transactions that are included in a block
    pub fn remove_included(&self, txs: &[Transaction]) {
        let mut mempool_txs = self.txs.write();
        let mut queue = self.queue.write();

        for tx in txs {
            let tx_hash = self.hash_tx(tx);
            mempool_txs.remove(&tx_hash);
            queue.retain(|mempool_tx| mempool_tx.transaction != *tx);
        }
    }

    /// Calculate transaction hash
    fn hash_tx(&self, tx: &Transaction) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&tx.sender);
        hasher.update(&tx.nonce.to_le_bytes());
        hasher.update(&tx.payload);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mempool_ordering() {
        let mempool = Mempool::new(1000);

        // Create transactions with different priorities
        let tx1 = Transaction {
            sender: [1u8; 32],
            nonce: 1,
            payload: vec![1, 2, 3],
            signature: [0u8; 64],
        };

        let tx2 = Transaction {
            sender: [2u8; 32],
            nonce: 2,
            payload: vec![4, 5, 6],
            signature: [0u8; 64],
        };

        // Add transactions
        mempool.add_tx(tx1.clone(), 10).unwrap();
        mempool.add_tx(tx2.clone(), 20).unwrap();

        // Check ordering
        let top_txs = mempool.get_top(2);
        assert_eq!(top_txs.len(), 2);
        assert_eq!(top_txs[0].nonce, 2); // Higher priority first
        assert_eq!(top_txs[1].nonce, 1);
    }
}
