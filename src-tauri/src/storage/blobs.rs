use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlobPayload {
    format: String,
    sha256: String,
    size: u64,
    b64: String,
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

    pub fn get_size(&self) -> u64 {
        self.size
    }
}
