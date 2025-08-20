use std::{collections::HashMap, fs, path::PathBuf};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hex;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use rand::RngCore;

use crate::storage::blobs::BlobPayload;

/// Encrypted storage for blockchain metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct BlobChainMetadata {
    /// Map of blob_id -> chain position
    pub blob_positions: HashMap<String, u64>,
    /// Ordered list of blob IDs in the chain
    pub chain_order: Vec<String>,
    /// Map of blob_id -> blob_chain_hash for efficient lookup
    pub blob_chain_hashes: HashMap<String, String>,
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
            blob_chain_hashes: HashMap::new(),
            chain_integrity_hash: String::new(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_blob(&mut self, blob_id: String, blob_chain_hash: String) {
        let position = self.chain_order.len() as u64;
        self.blob_positions.insert(blob_id.clone(), position);
        self.chain_order.push(blob_id.clone());
        self.blob_chain_hashes.insert(blob_id, blob_chain_hash);
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.update_integrity_hash();
    }
    
    pub fn update_integrity_hash(&mut self) {
        let mut hasher = Sha256::new();
        
        // Hash the chain order to ensure integrity
        for blob_id in &self.chain_order {
            hasher.update(blob_id.as_bytes());
        }
        
        self.chain_integrity_hash = hex::encode(hasher.finalize());
    }

    pub fn get_previous_blob_chain_hash(&self, position: u64) -> Option<String> {
        if position == 0 {
            return None;
        }
        
        let prev_position = position - 1;
        let prev_blob_id = &self.chain_order[prev_position as usize];
        self.blob_chain_hashes.get(prev_blob_id).cloned()
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
    backup_name: String,
    metadata: BlobChainMetadata,
}

impl BlobChainManager {
    fn get_encryption_key() -> [u8; 32] {
        // In a production environment, this key should be derived from:
        // 1. User password + salt
        // 2. Hardware-specific information
        // 3. Application-specific secret
        // For now, we use a deterministic key for the demo
        let mut hasher = Sha256::new();
        hasher.update(b"saveme_config_blob_chain_master_key");
        hasher.update(b"application_specific_salt_2024");
        let hash = hasher.finalize();
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash[..32]);
        key
    }

    pub fn new(storage_dir: PathBuf, backup_name: String) -> Result<Self> {
        let mut manager = Self {
            storage_dir,
            backup_name,
            metadata: BlobChainMetadata::new(),
        };
        
        // Try to load existing metadata
        if let Err(_) = manager.load_metadata() {
            // If loading fails, start with fresh metadata
            manager.metadata = BlobChainMetadata::new();
        }
        
        println!("Metadata loaded successfully for backup: {}", manager.backup_name);
        println!("Metadata: {:?}", manager.metadata);
        
        Ok(manager)
    }

    pub fn add_blob_to_chain(&mut self, blob_id: &str, blob: &mut BlobPayload) -> Result<()> {
        let current_position = self.metadata.chain_order.len() as u64;
        
        // Get the actual previous blob's chain hash if this isn't the first blob
        let previous_blob_hash = self.metadata.get_previous_blob_chain_hash(current_position);

        // Set the previous blob hash and finalize the chain hash
        blob.set_previous_blob_hash(previous_blob_hash);
        blob.finalize_blob_chain_hash()?;

        // Get the finalized blob chain hash to store in metadata
        let blob_chain_hash = blob.get_blob_chain_hash()
            .ok_or_else(|| anyhow!("Blob chain hash not finalized"))?
            .clone();

        // Add to metadata with the actual blob chain hash
        self.metadata.add_blob(blob_id.to_string(), blob_chain_hash);
        
        // Save the updated metadata
        self.save_metadata()?;

        Ok(())
    }

    pub fn verify_blob_chain(&self, blobs: &HashMap<String, BlobPayload>) -> Result<bool> {
        // First verify metadata integrity
        if !self.metadata.verify_integrity() {
            println!("Blob chain metadata integrity check failed");
            return Ok(false);
        }

        // Check that all blobs in the chain actually exist
        for blob_id in &self.metadata.chain_order {
            if !blobs.contains_key(blob_id) {
                println!("Missing blob in chain: {}", blob_id);
                return Ok(false);
            }
        }

        // Verify each blob in the chain and check consistency with metadata
        let mut expected_chain_hashes = Vec::new();
        
        for (i, blob_id) in self.metadata.chain_order.iter().enumerate() {
            let blob = blobs.get(blob_id)
                .ok_or_else(|| anyhow!("Missing blob in chain: {}", blob_id))?;
            
            println!("Verifying blob: {}", blob_id);

            // Verify blob internal integrity
            if !blob.verify_blob_integrity() {
                println!("Blob integrity check failed for: {}", blob_id);
                return Ok(false);
            }

            // Get what the previous hash should be for this position
            let expected_prev_hash = self.metadata.get_previous_blob_chain_hash(i as u64);

            // Check that the blob has the expected previous hash
            match (blob.get_previous_blob_hash(), &expected_prev_hash) {
                (Some(actual), Some(expected)) => {
                    if actual != expected {
                        println!("Chain link verification failed for blob {}: expected previous hash {}, got {}", 
                                 blob_id, expected, actual);
                        return Ok(false);
                    }
                }
                (None, None) => {
                    // First blob - OK
                }
                (Some(actual), None) => {
                    println!("First blob {} should not have previous hash but has {}", blob_id, actual);
                    return Ok(false);
                }
                (None, Some(expected)) => {
                    println!("Blob {} should have previous hash {} but doesn't", blob_id, expected);
                    return Ok(false);
                }
            }

            // Calculate and store what this blob's chain hash should be
            let mut expected_blob = BlobPayload::new(blob.get_format().to_string(), &blob.decode().unwrap_or_default());
            expected_blob.set_previous_blob_hash(expected_prev_hash.clone());
            expected_blob.finalize_blob_chain_hash()?;
            expected_chain_hashes.push(expected_blob.get_blob_chain_hash().cloned().unwrap());

            // Verify the actual chain hash matches what we expect
            if blob.get_blob_chain_hash() != Some(&expected_chain_hashes[i]) {
                println!("Chain hash mismatch for blob {}: expected {}, got {:?}", 
                         blob_id, expected_chain_hashes[i], blob.get_blob_chain_hash());
                return Ok(false);
            }

            // Also verify that the stored metadata has the correct chain hash
            if let Some(stored_hash) = self.metadata.blob_chain_hashes.get(blob_id) {
                if stored_hash != &expected_chain_hashes[i] {
                    println!("Metadata chain hash mismatch for blob {}: expected {}, stored {}", 
                             blob_id, expected_chain_hashes[i], stored_hash);
                    return Ok(false);
                }
            } else {
                println!("Missing chain hash in metadata for blob: {}", blob_id);
                return Ok(false);
            }
        }

        println!("Blob chain verification successful: {} blobs verified", self.metadata.chain_order.len());
        Ok(true)
    }

    pub fn get_chain_info(&self) -> &BlobChainMetadata {
        &self.metadata
    }

    fn get_metadata_path(&self) -> PathBuf {
        let filename = format!("{}_blob_chain.encrypted", self.backup_name);
        self.storage_dir.join(filename)
    }

    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let key = Self::get_encryption_key();
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        
        // Generate a random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt the data
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext for storage
        let mut encrypted_data = nonce_bytes.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);
        
        Ok(encrypted_data)
    }

    fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data: too short"));
        }
        
        let key = Self::get_encryption_key();
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        
        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt the data
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
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
        let mut manager = BlobChainManager::new(temp_dir.path().to_path_buf(), "test_backup".to_string())?;
        
        let mut blob1 = BlobPayload::new("tar.zst".to_string(), b"test data 1");
        let mut blob2 = BlobPayload::new("tar.zst".to_string(), b"test data 2");
        
        manager.add_blob_to_chain("blob1", &mut blob1)?;
        manager.add_blob_to_chain("blob2", &mut blob2)?;
        
        assert_eq!(manager.metadata.chain_order.len(), 2);
        assert!(blob1.verify_blob_integrity());
        assert!(blob2.verify_blob_integrity());
        
        // Verify that blob2 has a reference to blob1's actual chain hash
        assert!(blob2.get_previous_blob_hash().is_some());
        assert_eq!(blob2.get_previous_blob_hash(), blob1.get_blob_chain_hash());
        
        Ok(())
    }

    #[test]
    fn test_encrypted_metadata_storage() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = BlobChainManager::new(temp_dir.path().to_path_buf(), "test_backup".to_string())?;
        
        // Add some blobs
        let mut blob1 = BlobPayload::new("tar.zst".to_string(), b"test data 1");
        let mut blob2 = BlobPayload::new("tar.zst".to_string(), b"test data 2");
        
        manager.add_blob_to_chain("blob1", &mut blob1)?;
        manager.add_blob_to_chain("blob2", &mut blob2)?;
        
        // Verify the encrypted file was created with backup-specific name
        let metadata_file = temp_dir.path().join("test_backup_blob_chain.encrypted");
        assert!(metadata_file.exists());
        
        // Create a new manager and verify it can load the encrypted data
        let manager2 = BlobChainManager::new(temp_dir.path().to_path_buf(), "test_backup".to_string())?;
        assert_eq!(manager2.metadata.chain_order.len(), 2);
        assert_eq!(manager2.metadata.chain_order[0], "blob1");
        assert_eq!(manager2.metadata.chain_order[1], "blob2");
        
        // Verify the metadata integrity is preserved
        assert!(manager2.metadata.verify_integrity());
        
        Ok(())
    }

    #[test]
    fn test_complete_blob_chain_verification() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = BlobChainManager::new(temp_dir.path().to_path_buf(), "test_backup".to_string())?;
        
        // Create a chain of 3 blobs
        let mut blob1 = BlobPayload::new("tar.zst".to_string(), b"test data 1");
        let mut blob2 = BlobPayload::new("tar.zst".to_string(), b"test data 2");
        let mut blob3 = BlobPayload::new("tar.zst".to_string(), b"test data 3");
        
        manager.add_blob_to_chain("blob1", &mut blob1)?;
        manager.add_blob_to_chain("blob2", &mut blob2)?;
        manager.add_blob_to_chain("blob3", &mut blob3)?;
        
        // Create blob map for verification
        let mut blobs = HashMap::new();
        blobs.insert("blob1".to_string(), blob1);
        blobs.insert("blob2".to_string(), blob2);
        blobs.insert("blob3".to_string(), blob3);
        
        // Verify the complete chain
        assert!(manager.verify_blob_chain(&blobs)?);
        
        // Test that removing a blob breaks the chain
        blobs.remove("blob2");
        assert!(!manager.verify_blob_chain(&blobs)?);
        
        Ok(())
    }
}