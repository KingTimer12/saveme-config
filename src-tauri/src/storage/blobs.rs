use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlobPayload {
    format: String,
    sha256: String,
    size: u64,
    b64: String,
    // Blockchain fields - each blob links to the previous blob
    pub previous_blob_hash: Option<String>,
    pub blob_chain_hash: Option<String>,
}

impl BlobPayload {
    pub fn new(format: String, data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let sha256 = format!("{:x}", hasher.finalize());
        let b64 = general_purpose::STANDARD.encode(data);
        BlobPayload {
            format,
            sha256,
            size: data.len() as u64,
            b64,
            previous_blob_hash: None,
            blob_chain_hash: None,
        }
    }

    pub fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        general_purpose::STANDARD.decode(&self.b64)
    }

    pub fn get_format(&self) -> &str {
        &self.format
    }

    pub fn get_sha256(&self) -> &str {
        &self.sha256
    }

    // Blockchain methods for blob chaining
    pub fn get_previous_blob_hash(&self) -> Option<&String> {
        self.previous_blob_hash.as_ref()
    }

    pub fn get_blob_chain_hash(&self) -> Option<&String> {
        self.blob_chain_hash.as_ref()
    }

    pub fn set_previous_blob_hash(&mut self, previous_hash: Option<String>) {
        self.previous_blob_hash = previous_hash;
    }

    pub fn calculate_blob_content_hash(&self) -> String {
        // Calculate a deterministic hash of this blob's content for chaining
        let mut hasher = Sha256::new();
        hasher.update(self.format.as_bytes());
        hasher.update(self.sha256.as_bytes());
        hasher.update(&self.size.to_le_bytes());
        hasher.update(self.b64.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn finalize_blob_chain_hash(&mut self) -> Result<(), anyhow::Error> {
        let mut hasher = Sha256::new();
        
        // Include previous blob hash if available
        if let Some(prev_hash) = &self.previous_blob_hash {
            hasher.update(prev_hash.as_bytes());
        }
        
        // Include current blob content hash
        let content_hash = self.calculate_blob_content_hash();
        hasher.update(content_hash.as_bytes());
        
        self.blob_chain_hash = Some(hex::encode(hasher.finalize()));
        Ok(())
    }

    pub fn verify_blob_integrity(&self) -> bool {
        // Verify that the blob chain hash is correct
        let mut hasher = Sha256::new();
        
        if let Some(prev_hash) = &self.previous_blob_hash {
            hasher.update(prev_hash.as_bytes());
        }
        
        let content_hash = self.calculate_blob_content_hash();
        hasher.update(content_hash.as_bytes());
        let expected_chain_hash = hex::encode(hasher.finalize());
        
        match &self.blob_chain_hash {
            Some(stored_hash) => *stored_hash == expected_chain_hash,
            None => false, // No chain hash means not properly initialized
        }
    }
}
