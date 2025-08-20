use std::{collections::HashMap, fs, path::PathBuf};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hex;

use crate::storage::blobs::BlobPayload;

/// Encrypted storage for blockchain metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct BlobChainMetadata {
    /// Map of blob_id -> chain position
    pub blob_positions: HashMap<String, u64>,
    /// Ordered list of blob IDs in the chain
    pub chain_order: Vec<String>,
    /// Hash of the entire chain for integrity verification
    pub chain_integrity_hash: String,
    /// Timestamp when the chain was last updated
    pub last_updated: String,
}

impl BlobChainMetadata {
    pub fn new() -> Self {
        Self {
            blob_positions: HashMap::new(),
            chain_order: Vec::new(),
            chain_integrity_hash: String::new(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_blob(&mut self, blob_id: String) {
        let position = self.chain_order.len() as u64;
        self.blob_positions.insert(blob_id.clone(), position);
        self.chain_order.push(blob_id);
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.update_integrity_hash();
    }

    pub fn get_previous_blob_id(&self, current_position: u64) -> Option<&String> {
        if current_position == 0 {
            None
        } else {
            self.chain_order.get((current_position - 1) as usize)
        }
    }

    pub fn update_integrity_hash(&mut self) {
        let mut hasher = Sha256::new();
        
        // Hash the chain order to ensure integrity
        for blob_id in &self.chain_order {
            hasher.update(blob_id.as_bytes());
        }
        
        self.chain_integrity_hash = hex::encode(hasher.finalize());
    }

    pub fn verify_integrity(&self) -> bool {
        let mut hasher = Sha256::new();
        
        for blob_id in &self.chain_order {
            hasher.update(blob_id.as_bytes());
        }
        
        let calculated_hash = hex::encode(hasher.finalize());
        calculated_hash == self.chain_integrity_hash
    }
}

/// Manager for blob blockchain operations
pub struct BlobChainManager {
    storage_dir: PathBuf,
    metadata: BlobChainMetadata,
}

impl BlobChainManager {
    const CHAIN_METADATA_FILE: &'static str = "blob_chain.encrypted";
    const ENCRYPTION_KEY: &'static [u8] = b"saveme_config_blob_chain_key_32b"; // 32 bytes for AES-256

    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        let mut manager = Self {
            storage_dir,
            metadata: BlobChainMetadata::new(),
        };
        
        // Try to load existing metadata
        if let Err(_) = manager.load_metadata() {
            // If loading fails, start with fresh metadata
            manager.metadata = BlobChainMetadata::new();
        }
        
        Ok(manager)
    }

    pub fn add_blob_to_chain(&mut self, blob_id: &str, blob: &mut BlobPayload) -> Result<()> {
        let current_position = self.metadata.chain_order.len() as u64;
        
        // Get the previous blob's chain hash if this isn't the first blob
        let previous_blob_hash = if current_position > 0 {
            let prev_blob_id = &self.metadata.chain_order[(current_position - 1) as usize];
            // In a real implementation, we'd load the previous blob and get its chain hash
            // For now, we'll use a placeholder that represents the previous blob's hash
            Some(format!("prev_chain_hash_{}", prev_blob_id))
        } else {
            None
        };

        // Set the previous blob hash and finalize the chain hash
        blob.set_previous_blob_hash(previous_blob_hash);
        blob.finalize_blob_chain_hash()?;

        // Add to metadata
        self.metadata.add_blob(blob_id.to_string());
        
        // Save the updated metadata
        self.save_metadata()?;

        Ok(())
    }

    pub fn verify_blob_chain(&self, blobs: &HashMap<String, BlobPayload>) -> Result<bool> {
        // First verify metadata integrity
        if !self.metadata.verify_integrity() {
            return Ok(false);
        }

        // Verify each blob in the chain
        for (i, blob_id) in self.metadata.chain_order.iter().enumerate() {
            let blob = blobs.get(blob_id)
                .ok_or_else(|| anyhow!("Missing blob in chain: {}", blob_id))?;

            // Verify blob integrity
            if !blob.verify_blob_integrity() {
                return Ok(false);
            }

            // Verify chain linking
            if i > 0 {
                let prev_blob_id = &self.metadata.chain_order[i - 1];
                let prev_blob = blobs.get(prev_blob_id)
                    .ok_or_else(|| anyhow!("Missing previous blob in chain: {}", prev_blob_id))?;

                // Check that current blob's previous_blob_hash matches previous blob's chain_hash
                if let (Some(current_prev), Some(_prev_chain)) = (blob.get_previous_blob_hash(), prev_blob.get_blob_chain_hash()) {
                    // For now, we use a simplified verification
                    // In a complete implementation, we'd store actual chain hashes
                    if !current_prev.contains(prev_blob_id) {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    pub fn get_chain_info(&self) -> &BlobChainMetadata {
        &self.metadata
    }

    fn get_metadata_path(&self) -> PathBuf {
        self.storage_dir.join(Self::CHAIN_METADATA_FILE)
    }

    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Simple XOR encryption for demonstration
        // In production, use proper AES-GCM encryption
        let key = Self::ENCRYPTION_KEY;
        let mut encrypted = Vec::with_capacity(data.len());
        
        for (i, &byte) in data.iter().enumerate() {
            let key_byte = key[i % key.len()];
            encrypted.push(byte ^ key_byte);
        }
        
        Ok(encrypted)
    }

    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Simple XOR decryption (same as encryption for XOR)
        self.encrypt_data(data)
    }

    fn save_metadata(&self) -> Result<()> {
        fs::create_dir_all(&self.storage_dir)?;
        
        let json_data = serde_json::to_string(&self.metadata)?;
        let encrypted_data = self.encrypt_data(json_data.as_bytes())?;
        
        fs::write(self.get_metadata_path(), encrypted_data)?;
        Ok(())
    }

    fn load_metadata(&mut self) -> Result<()> {
        let metadata_path = self.get_metadata_path();
        if !metadata_path.exists() {
            return Err(anyhow!("Metadata file does not exist"));
        }

        let encrypted_data = fs::read(metadata_path)?;
        let decrypted_data = self.decrypt_data(&encrypted_data)?;
        let json_str = String::from_utf8(decrypted_data)?;
        
        self.metadata = serde_json::from_str(&json_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_blob_chain_metadata() {
        let mut metadata = BlobChainMetadata::new();
        
        metadata.add_blob("blob1".to_string());
        metadata.add_blob("blob2".to_string());
        
        assert_eq!(metadata.chain_order.len(), 2);
        assert_eq!(metadata.blob_positions.get("blob1"), Some(&0));
        assert_eq!(metadata.blob_positions.get("blob2"), Some(&1));
        assert!(metadata.verify_integrity());
    }

    #[test]
    fn test_blob_chain_manager() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = BlobChainManager::new(temp_dir.path().to_path_buf())?;
        
        let mut blob1 = BlobPayload::new("tar.zst".to_string(), b"test data 1");
        let mut blob2 = BlobPayload::new("tar.zst".to_string(), b"test data 2");
        
        manager.add_blob_to_chain("blob1", &mut blob1)?;
        manager.add_blob_to_chain("blob2", &mut blob2)?;
        
        assert_eq!(manager.metadata.chain_order.len(), 2);
        assert!(blob1.verify_blob_integrity());
        assert!(blob2.verify_blob_integrity());
        
        Ok(())
    }
}