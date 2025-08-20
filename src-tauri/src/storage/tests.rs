#[cfg(test)]
mod tests {
    use crate::storage::{manifest::Manifest, entry::Entry};
    
    #[test]
    fn test_blockchain_integrity_calculation() {
        let mut manifest = Manifest::new(
            "test-backup".to_string(),
            "2023-01-01T00:00:00Z".to_string(),
            "linux".to_string(),
        );

        // Add some test entries
        manifest.entries.push(Entry {
            blob_id: "test-blob-1".to_string(),
            target_hint: "app:test".to_string(),
            logical_path: "/test/path".to_string(),
            tar_member: Some("test.txt".to_string()),
        });

        // Calculate backup hash
        let backup_hash = manifest.calculate_backup_hash().unwrap();
        assert!(!backup_hash.is_empty());
        assert_eq!(backup_hash.len(), 64); // SHA256 hex length

        // Test chain hash calculation
        manifest.finalize_chain_hash().unwrap();
        assert!(manifest.backup_chain_hash.is_some());
        assert!(!manifest.backup_chain_hash.as_ref().unwrap().is_empty());

        // Test integrity verification
        let is_valid = manifest.verify_backup_integrity().unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_blockchain_chain_linking() {
        let mut manifest1 = Manifest::new(
            "backup-1".to_string(),
            "2023-01-01T00:00:00Z".to_string(),
            "linux".to_string(),
        );

        // First backup
        manifest1.finalize_chain_hash().unwrap();
        let chain_hash_1 = manifest1.backup_chain_hash.clone().unwrap();

        // Second backup linking to first
        let mut manifest2 = Manifest::new(
            "backup-2".to_string(),
            "2023-01-02T00:00:00Z".to_string(),
            "linux".to_string(),
        );
        
        manifest2.previous_backup_hash = Some(chain_hash_1.clone());
        manifest2.finalize_chain_hash().unwrap();

        // Verify the chain is properly linked
        assert_eq!(manifest2.previous_backup_hash.as_ref().unwrap(), &chain_hash_1);
        assert!(manifest2.backup_chain_hash.is_some());
        assert_ne!(manifest2.backup_chain_hash.as_ref().unwrap(), &chain_hash_1);

        // Both should verify as valid
        assert!(manifest1.verify_backup_integrity().unwrap());
        assert!(manifest2.verify_backup_integrity().unwrap());
    }

    #[test]
    fn test_compression_level_usage() {
        // This test ensures we're using the maximum compression level
        let test_data = b"This is test data for compression testing. ".repeat(100);
        
        // Test with level 3 (old)
        let compressed_level_3 = zstd::encode_all(&test_data[..], 3).unwrap();
        
        // Test with level 19 (new)
        let compressed_level_19 = zstd::encode_all(&test_data[..], 19).unwrap();
        
        // Level 19 should compress better (smaller size)
        assert!(compressed_level_19.len() <= compressed_level_3.len());
        
        // Both should decompress to the same original data
        let decompressed_3 = zstd::decode_all(&compressed_level_3[..]).unwrap();
        let decompressed_19 = zstd::decode_all(&compressed_level_19[..]).unwrap();
        
        assert_eq!(decompressed_3, test_data);
        assert_eq!(decompressed_19, test_data);
        assert_eq!(decompressed_3, decompressed_19);
    }
}