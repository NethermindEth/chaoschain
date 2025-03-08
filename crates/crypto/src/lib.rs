use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH,
};
use parking_lot::RwLock;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Crypto errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid key")]
    InvalidKey,
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Agent identity and keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKeys {
    /// Agent ID (hex encoded public key)
    pub id: String,
    /// Agent name/alias
    pub name: String,
    /// Agent role (validator, producer, etc)
    pub role: String,
    /// Drama score (0-100)
    pub drama_score: u8,
    /// Stake amount
    pub stake: u64,
}

/// Key manager for handling agent keys
#[derive(Debug)]
pub struct KeyManager {
    /// Signing keys by agent ID
    signing_keys: RwLock<HashMap<String, SigningKey>>,
    /// Agent metadata
    agents: RwLock<HashMap<String, AgentKeys>>,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new() -> Self {
        Self {
            signing_keys: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// Generate new agent keys
    pub fn generate_agent_keys(
        &self,
        name: String,
        role: String,
        initial_stake: u64,
    ) -> Result<AgentKeys, CryptoError> {
        // Generate new Ed25519 keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Create agent ID from public key
        let id = hex::encode(verifying_key.as_bytes());

        // Create agent keys
        let agent = AgentKeys {
            id: id.clone(),
            name,
            role,
            drama_score: 50, // Start with neutral drama score
            stake: initial_stake,
        };

        // Store keys
        self.signing_keys.write().insert(id.clone(), signing_key);
        self.agents.write().insert(id.clone(), agent.clone());

        Ok(agent)
    }

    /// Sign data with agent's key
    pub fn sign(&self, agent_id: &str, data: &[u8]) -> Result<[u8; SIGNATURE_LENGTH], CryptoError> {
        let keys = self.signing_keys.read();
        let signing_key = keys
            .get(agent_id)
            .ok_or_else(|| CryptoError::KeyNotFound(agent_id.to_string()))?;

        let signature = signing_key.sign(data);
        Ok(signature.to_bytes())
    }

    /// Verify a signature
    pub fn verify(
        &self,
        agent_id: &str,
        data: &[u8],
        signature: &[u8; SIGNATURE_LENGTH],
    ) -> Result<bool, CryptoError> {
        // Get agent's public key
        let agents = self.agents.read();
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| CryptoError::KeyNotFound(agent_id.to_string()))?;

        // Decode public key
        let pubkey_bytes = hex::decode(&agent.id).map_err(|_| CryptoError::InvalidKey)?;

        let verifying_key = VerifyingKey::from_bytes(
            &pubkey_bytes[..PUBLIC_KEY_LENGTH]
                .try_into()
                .map_err(|_| CryptoError::InvalidKey)?,
        )
        .map_err(|_| CryptoError::InvalidKey)?;

        // Verify signature
        let sig = Signature::from_bytes(signature);
        match verifying_key.verify(data, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get agent info
    pub fn get_agent(&self, agent_id: &str) -> Option<AgentKeys> {
        self.agents.read().get(agent_id).cloned()
    }

    /// Update agent stake
    pub fn update_stake(&self, agent_id: &str, new_stake: u64) -> Result<(), CryptoError> {
        let mut agents = self.agents.write();
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.stake = new_stake;
            Ok(())
        } else {
            Err(CryptoError::KeyNotFound(agent_id.to_string()))
        }
    }

    /// Update drama score
    pub fn update_drama_score(&self, agent_id: &str, new_score: u8) -> Result<(), CryptoError> {
        let mut agents = self.agents.write();
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.drama_score = new_score;
            Ok(())
        } else {
            Err(CryptoError::KeyNotFound(agent_id.to_string()))
        }
    }

    /// Get agent ID for this key manager
    pub fn get_agent_id(&self) -> Option<String> {
        self.agents.read().keys().next().cloned()
    }
}

/// Thread-safe key manager
#[derive(Clone)]
pub struct KeyManagerHandle {
    inner: Arc<KeyManager>,
}

impl std::fmt::Debug for KeyManagerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyManagerHandle")
            .field("agent_count", &self.inner.agents.read().len())
            .finish()
    }
}

impl KeyManagerHandle {
    /// Create a new key manager handle
    pub fn new() -> Self {
        Self {
            inner: Arc::new(KeyManager::new()),
        }
    }

    /// Get reference to inner key manager
    pub fn inner(&self) -> &KeyManager {
        &self.inner
    }

    /// Get agent ID for this key manager
    pub fn get_agent_id(&self) -> Option<String> {
        self.inner.agents.read().keys().next().cloned()
    }
}

impl Default for KeyManagerHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation_and_signing() {
        let km = KeyManager::new();

        // Generate agent keys
        let agent = km
            .generate_agent_keys("TestAgent".to_string(), "validator".to_string(), 1000)
            .unwrap();

        // Test signing
        let data = b"test message";
        let signature = km.sign(&agent.id, data).unwrap();

        // Test verification
        let valid = km.verify(&agent.id, data, &signature).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_invalid_signature() {
        let km = KeyManager::new();

        // Generate agent keys
        let agent = km
            .generate_agent_keys("TestAgent".to_string(), "validator".to_string(), 1000)
            .unwrap();

        // Test with invalid signature
        let data = b"test message";
        let mut invalid_sig = [0u8; SIGNATURE_LENGTH];
        let valid = km.verify(&agent.id, data, &invalid_sig).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_agent_updates() {
        let km = KeyManager::new();

        // Generate agent keys
        let agent = km
            .generate_agent_keys("TestAgent".to_string(), "validator".to_string(), 1000)
            .unwrap();

        // Update stake
        km.update_stake(&agent.id, 2000).unwrap();
        let updated = km.get_agent(&agent.id).unwrap();
        assert_eq!(updated.stake, 2000);

        // Update drama score
        km.update_drama_score(&agent.id, 75).unwrap();
        let updated = km.get_agent(&agent.id).unwrap();
        assert_eq!(updated.drama_score, 75);
    }
}
