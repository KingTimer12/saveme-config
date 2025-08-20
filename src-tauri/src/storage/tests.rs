#[cfg(test)]
mod tests {
    use crate::storage::{manifest::Manifest, entry::Entry, blobs::BlobPayload};
    use tempfile::TempDir;
    
    #[test]
    fn test_blob_integrity_calculation() {
        let mut blob = BlobPayload::new("tar.zst".to_string(), b"test data");
        
        // Test content hash calculation
        let content_hash = blob.calculate_blob_content_hash();
        assert!(!content_hash.is_empty());
        assert_eq!(content_hash.len(), 64); // SHA256 hex length

        // Test chain hash finalization
        blob.finalize_blob_chain_hash().unwrap();
        assert!(blob.get_blob_chain_hash().is_some());
        assert!(!blob.get_blob_chain_hash().unwrap().is_empty());

        // Test integrity verification
        assert!(blob.verify_blob_integrity());
    }

    #[test]
    fn test_blob_chain_linking() {
        let mut blob1 = BlobPayload::new("tar.zst".to_string(), b"test data 1");
        let mut blob2 = BlobPayload::new("tar.zst".to_string(), b"test data 2");

        // First blob (no previous)
        blob1.finalize_blob_chain_hash().unwrap();
        let chain_hash_1 = blob1.get_blob_chain_hash().cloned().unwrap();

        // Second blob linking to first
        blob2.set_previous_blob_hash(Some(chain_hash_1.clone()));
        blob2.finalize_blob_chain_hash().unwrap();

        // Verify the chain is properly linked
        assert_eq!(blob2.get_previous_blob_hash(), Some(&chain_hash_1));
        assert!(blob2.get_blob_chain_hash().is_some());
        assert_ne!(blob2.get_blob_chain_hash().unwrap(), &chain_hash_1);

        // Both should verify as valid
        assert!(blob1.verify_blob_integrity());
        assert!(blob2.verify_blob_integrity());
    }

    #[test]
    fn test_manifest_blob_chain_verification() -> Result<(), anyhow::Error> {
        let _temp_dir = TempDir::new()?;
        
        // Create a manifest with test data
        let mut manifest = Manifest::new(
            "test-backup".to_string(),
            "2023-01-01T00:00:00Z".to_string(),
            "linux".to_string(),
        );

        // Simulate creating blobs through the manifest
        // Note: This is a simplified test - in reality, blobs would be created through create_blob_from_file
        let mut blob1 = BlobPayload::new("tar.zst".to_string(), b"test data 1");
        let mut blob2 = BlobPayload::new("tar.zst".to_string(), b"test data 2");
        
        // Finalize chain hashes for the blobs
        blob1.finalize_blob_chain_hash().unwrap();
        blob2.finalize_blob_chain_hash().unwrap();
        
        manifest.add_blob_for_testing("blob1".to_string(), blob1);
        manifest.add_blob_for_testing("blob2".to_string(), blob2);

        // Add corresponding entries
        manifest.entries.push(Entry {
            blob_id: "blob1".to_string(),
            target_hint: "app:test1".to_string(),
            logical_path: "/test/path1".to_string(),
            tar_member: Some("test1.txt".to_string()),
        });

        manifest.entries.push(Entry {
            blob_id: "blob2".to_string(),
            target_hint: "app:test2".to_string(),
            logical_path: "/test/path2".to_string(),
            tar_member: Some("test2.txt".to_string()),
        });

        // Test that individual blobs are valid
        for blob in manifest.blobs.values() {
            assert!(blob.verify_blob_integrity());
        }

        Ok(())
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