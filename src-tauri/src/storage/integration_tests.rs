#[cfg(test)]
mod integration_tests {
    use tempfile::{TempDir, NamedTempFile};
    use std::io::Write;
    use crate::storage::{manifest::Manifest, blob_chain::BlobChainManager};

    #[test]
    fn test_complete_blob_chain_workflow() -> Result<(), anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let storage_dir = temp_dir.path();

        // Create test files
        let mut test_file1 = NamedTempFile::new()?;
        let mut test_file2 = NamedTempFile::new()?;
        let mut test_file3 = NamedTempFile::new()?;
        
        test_file1.write_all(b"Test config file 1 content")?;
        test_file2.write_all(b"Test config file 2 content - different")?;
        test_file3.write_all(b"Test config file 3 content - also different")?;
        
        test_file1.flush()?;
        test_file2.flush()?;
        test_file3.flush()?;

        // Create a manifest and add files as blobs (simulating the full process)
        let mut manifest = Manifest::new(
            "integration-test-backup".to_string(),
            chrono::Utc::now().to_rfc3339(),
            "linux".to_string(),
        );

        // This would normally be done by create_blob_from_file, but we simulate it here
        // to avoid filesystem dependencies in the test
        
        // Simulate adding 3 files as blobs with proper blockchain linking
        let blob_data1 = b"compressed_tar_data_file1";
        let blob_data2 = b"compressed_tar_data_file2";
        let blob_data3 = b"compressed_tar_data_file3";

        let mut blob1 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), blob_data1);
        let mut blob2 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), blob_data2);
        let mut blob3 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), blob_data3);

        // Set up blob chain manager for test backup
        let mut chain_manager = BlobChainManager::new(storage_dir.to_path_buf(), "test_backup".to_string())?;

        // Add blobs to the chain in order
        chain_manager.add_blob_to_chain("blob1", &mut blob1)?;
        chain_manager.add_blob_to_chain("blob2", &mut blob2)?;
        chain_manager.add_blob_to_chain("blob3", &mut blob3)?;

        // Add blobs to manifest
        manifest.add_blob_for_testing("blob1".to_string(), blob1);
        manifest.add_blob_for_testing("blob2".to_string(), blob2);
        manifest.add_blob_for_testing("blob3".to_string(), blob3);

        // Add corresponding entries
        manifest.entries.push(crate::storage::entry::Entry {
            blob_id: "blob1".to_string(),
            target_hint: "app:test1".to_string(),
            logical_path: "/test/file1.conf".to_string(),
            tar_member: Some("file1.conf".to_string()),
        });

        manifest.entries.push(crate::storage::entry::Entry {
            blob_id: "blob2".to_string(),
            target_hint: "app:test2".to_string(),
            logical_path: "/test/file2.conf".to_string(),
            tar_member: Some("file2.conf".to_string()),
        });

        manifest.entries.push(crate::storage::entry::Entry {
            blob_id: "blob3".to_string(),
            target_hint: "app:test3".to_string(),
            logical_path: "/test/file3.conf".to_string(),
            tar_member: Some("file3.conf".to_string()),
        });

        // Verify blob chain integrity
        let integrity_check = manifest.verify_blob_chain_integrity_with_dir(Some(storage_dir.to_path_buf()))?;
        assert!(integrity_check, "Blob chain integrity verification should pass");

        // Get blob chain info
        let chain_info = manifest.get_blob_chain_info_with_dir(Some(storage_dir.to_path_buf()))?;
        assert!(chain_info.contains("3 blobs"), "Chain info should mention 3 blobs: {}", chain_info);
        assert!(chain_info.contains("integrity hash"), "Chain info should mention integrity hash: {}", chain_info);

        // Verify that the encrypted metadata file was created
        let metadata_file = storage_dir.join("test_backup_blob_chain.encrypted");
        assert!(metadata_file.exists(), "Encrypted blockchain metadata file should exist");

        // Create a new chain manager to verify persistence
        let chain_manager2 = BlobChainManager::new(storage_dir.to_path_buf(), "test_backup".to_string())?;
        let chain_metadata = chain_manager2.get_chain_info();
        assert_eq!(chain_metadata.chain_order.len(), 3, "Chain should have 3 blobs after loading from disk");
        assert_eq!(chain_metadata.chain_order[0], "blob1", "First blob should be blob1");
        assert_eq!(chain_metadata.chain_order[1], "blob2", "Second blob should be blob2");
        assert_eq!(chain_metadata.chain_order[2], "blob3", "Third blob should be blob3");

        // Verify that the metadata integrity is preserved
        assert!(chain_metadata.verify_integrity(), "Chain metadata integrity should be preserved");

        println!("✅ Complete blob chain workflow test passed!");
        println!("   - Created 3 blobs with proper blockchain linking");
        println!("   - Verified blob chain integrity");
        println!("   - Confirmed encrypted metadata storage");
        println!("   - Verified persistence across manager instances");

        Ok(())
    }

    #[test]
    fn test_blob_chain_security_properties() -> Result<(), anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let storage_dir = temp_dir.path();

        let mut chain_manager = BlobChainManager::new(storage_dir.to_path_buf(), "test_backup".to_string())?;

        // Create and add blobs to chain
        let mut blob1 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"data1");
        let mut blob2 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"data2");
        let mut blob3 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"data3");

        chain_manager.add_blob_to_chain("blob1", &mut blob1)?;
        chain_manager.add_blob_to_chain("blob2", &mut blob2)?;
        chain_manager.add_blob_to_chain("blob3", &mut blob3)?;

        // Create blob map for verification
        let mut blobs = std::collections::HashMap::new();
        blobs.insert("blob1".to_string(), blob1);
        blobs.insert("blob2".to_string(), blob2.clone());
        blobs.insert("blob3".to_string(), blob3);

        // Test 1: Complete chain should verify successfully
        assert!(chain_manager.verify_blob_chain(&blobs)?, "Complete chain should verify");

        // Test 2: Missing blob should break the chain
        blobs.remove("blob2");
        assert!(!chain_manager.verify_blob_chain(&blobs)?, "Chain with missing blob should fail verification");

        // Test 3: Blob with wrong previous hash should break the chain
        blobs.insert("blob2".to_string(), blob2.clone());
        let mut wrong_chain_blob = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"data2");
        // Set a wrong previous hash
        wrong_chain_blob.set_previous_blob_hash(Some("wrong_previous_hash".to_string()));
        wrong_chain_blob.finalize_blob_chain_hash()?;
        
        blobs.insert("blob2".to_string(), wrong_chain_blob);
        assert!(!chain_manager.verify_blob_chain(&blobs)?, "Chain with wrong previous hash should fail verification");

        println!("✅ Blob chain security properties test passed!");
        println!("   - Verified complete chain validation");
        println!("   - Confirmed missing blob detection");
        println!("   - Confirmed wrong chain link detection");

        Ok(())
    }

    #[test]
    fn test_encrypted_storage_security() -> Result<(), anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let storage_dir = temp_dir.path();

        let mut chain_manager = BlobChainManager::new(storage_dir.to_path_buf(), "test_backup".to_string())?;

        // Add some sensitive blob metadata
        let mut blob1 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"sensitive_config_data");
        chain_manager.add_blob_to_chain("sensitive_blob", &mut blob1)?;

        // Verify encrypted file exists
        let metadata_file = storage_dir.join("test_backup_blob_chain.encrypted");
        assert!(metadata_file.exists(), "Encrypted metadata file should exist");

        // Read the raw encrypted data
        let encrypted_data = std::fs::read(&metadata_file)?;
        
        // Verify that the data is actually encrypted (should not contain plaintext)
        let data_string = String::from_utf8_lossy(&encrypted_data);
        assert!(!data_string.contains("sensitive_blob"), "Encrypted data should not contain plaintext blob names");
        assert!(!data_string.contains("chain_order"), "Encrypted data should not contain plaintext field names");
        
        // Verify that a new manager can still decrypt and read the data
        let chain_manager2 = BlobChainManager::new(storage_dir.to_path_buf(), "test_backup".to_string())?;
        let metadata = chain_manager2.get_chain_info();
        assert_eq!(metadata.chain_order[0], "sensitive_blob", "Decrypted data should contain correct blob name");

        println!("✅ Encrypted storage security test passed!");
        println!("   - Verified data is properly encrypted on disk");
        println!("   - Confirmed plaintext is not visible in encrypted file");
        println!("   - Verified decryption works correctly");

        Ok(())
    }

    #[test]
    fn test_per_backup_encryption_isolation() -> Result<(), anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let storage_dir = temp_dir.path();

        // Create two different backup chains
        let mut chain_manager1 = BlobChainManager::new(storage_dir.to_path_buf(), "backup1".to_string())?;
        let mut chain_manager2 = BlobChainManager::new(storage_dir.to_path_buf(), "backup2".to_string())?;

        // Add different blobs to each backup
        let mut blob1_backup1 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"backup1_data");
        let mut blob1_backup2 = crate::storage::blobs::BlobPayload::new("tar.zst".to_string(), b"backup2_data");

        chain_manager1.add_blob_to_chain("blob1", &mut blob1_backup1)?;
        chain_manager2.add_blob_to_chain("blob1", &mut blob1_backup2)?;

        // Verify that separate encrypted files were created
        let metadata_file1 = storage_dir.join("backup1_blob_chain.encrypted");
        let metadata_file2 = storage_dir.join("backup2_blob_chain.encrypted");
        assert!(metadata_file1.exists(), "backup1 encrypted metadata file should exist");
        assert!(metadata_file2.exists(), "backup2 encrypted metadata file should exist");

        // Verify the files have different content (different chains)
        let data1 = std::fs::read(&metadata_file1)?;
        let data2 = std::fs::read(&metadata_file2)?;
        assert_ne!(data1, data2, "Different backup chains should have different encrypted data");

        // Verify each backup can only see its own chain
        let chain_info1 = chain_manager1.get_chain_info();
        let chain_info2 = chain_manager2.get_chain_info();
        
        assert_eq!(chain_info1.chain_order.len(), 1, "backup1 should have 1 blob");
        assert_eq!(chain_info2.chain_order.len(), 1, "backup2 should have 1 blob");

        println!("✅ Per-backup encryption isolation test passed!");
        println!("   - Verified separate encrypted files are created for different backups");
        println!("   - Confirmed backup chains are isolated from each other");
        println!("   - Verified each backup can only access its own metadata");

        Ok(())
    }
}