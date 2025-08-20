use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Ok};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::Builder;
use walkdir::WalkDir;
use zstd::encode_all;

use crate::storage::{blobs::BlobPayload, entry::Entry};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub created_at: String,
    pub os_source: String,
    pub entries: Vec<Entry>,
    blobs: HashMap<String, BlobPayload>,
    // Blockchain integrity fields
    pub previous_backup_hash: Option<String>,
    pub backup_chain_hash: Option<String>,
}

impl Manifest {
    pub fn new(name: String, created_at: String, os_source: String) -> Self {
        Self {
            name,
            created_at,
            os_source,
            entries: Vec::new(),
            blobs: HashMap::new(),
            previous_backup_hash: None,
            backup_chain_hash: None,
        }
    }

    pub fn base_storage_dir() -> Result<PathBuf, anyhow::Error> {
        let proj = directories::ProjectDirs::from("com", "you", "saveconfig")
            .ok_or_else(|| anyhow!("cannot get project dir"))?;
        Ok(proj.data_local_dir().to_path_buf())
    }

    pub fn load_from(name: &str) -> Result<Self, anyhow::Error> {
        let manifest_path = Self::base_storage_dir()?.join(name).join("manifest.json");
        let content = fs::read_to_string(manifest_path)?;
        let mut manifest: Manifest = serde_json::from_str(&content)?;
        manifest.ingest_blobs_dir()?;
        Ok(manifest)
    }

    fn backup_dir(&self) -> Result<PathBuf, anyhow::Error> {
        Ok(Self::base_storage_dir()?.join(&self.name))
    }
    
    pub fn save(&mut self) -> Result<(), anyhow::Error> {
        // Finalizar hash da cadeia blockchain antes de salvar
        self.finalize_chain_hash()?;
        
        let backup_dir = self.backup_dir()?;
        fs::create_dir_all(&backup_dir)?;
        let manifest_path = backup_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn find_existing_blob_by_content(&self, content_hash: &str) -> Option<String> {
        // Check if any existing blob has the same content hash
        for (blob_id, blob) in &self.blobs {
            if blob.get_sha256() == content_hash {
                return Some(blob_id.clone());
            }
        }
        None
    }

    pub fn find_existing_blob_across_backups(content_hash: &str) -> Result<Option<(String, String)>, anyhow::Error> {
        // Check across all existing backups for duplicate content
        let storage_dir = Self::base_storage_dir()?;
        if !storage_dir.exists() {
            return Ok(None);
        }

        for entry in fs::read_dir(storage_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            
            let backup_name = entry.file_name().to_string_lossy().into_owned();
            let manifest = Self::load_from(&backup_name)?;
            
            if let Some(blob_id) = manifest.find_existing_blob_by_content(content_hash) {
                return Ok(Some((backup_name, blob_id)));
            }
        }
        
        Ok(None)
    }

    pub fn create_blob_from_file(
        &mut self,
        src: &Path,
        target_hint: &str,
    ) -> Result<(), anyhow::Error> {
        let blob_dir = self.backup_dir()?.join("blobs");
        println!("Creating blob from file");
        fs::create_dir_all(&blob_dir)?;
        println!("Created blob directory in {}", blob_dir.display());

        // Cria TAR na memória
        println!("Creating TAR archive");
        let mut tar_data = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_data);
            let file_name = src.file_name().ok_or_else(|| anyhow!("Invalid file name"))?;
            builder.append_path_with_name(src, file_name)?;
            builder.finish()?;
        }
        println!("Created TAR archive");

        // Comprime com zstd (nivel máximo para melhor compressão)
        println!("Compressing TAR archive with maximum compression");
        let compressed = encode_all(&tar_data[..], 19)?;

        // SHA256 do conteúdo comprimido para verificar duplicação
        println!("Calculating SHA256 hash for deduplication");
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let content_hash = hex::encode(hasher.finalize());

        // Verificar se o blob já existe (deduplicação)
        println!("Checking for existing blob with same content");
        if let Some((existing_backup, existing_blob_id)) = Self::find_existing_blob_across_backups(&content_hash)? {
            println!("Found duplicate content in backup '{}' with blob ID '{}'", existing_backup, existing_blob_id);
            
            // Usar referência do blob existente ao invés de criar novo
            self.entries.push({
                Entry {
                    blob_id: existing_blob_id,
                    target_hint: target_hint.to_string(),
                    logical_path: src.to_string_lossy().into_owned(),
                    tar_member: Some(src.file_name().unwrap().to_string_lossy().into_owned())
                }
            });
            
            println!("Reused existing blob - storage space saved!");
            return Ok(());
        }

        let id = content_hash; // Use content hash as ID for better deduplication

        // Salva no disco
        let blob_path = blob_dir.join(format!("{id}.tar.zst"));
        if !blob_path.exists() {
            fs::write(&blob_path, &compressed)?;
        }
        
        println!("Blob saved to disk");

        // Adicionar blob ao manifest atual
        self.blobs.insert(id.clone(), BlobPayload::new("tar.zst".to_string(), &compressed));

        self.entries.push({
            Entry {
                blob_id: id,
                target_hint: target_hint.to_string(),
                logical_path: src.to_string_lossy().into_owned(),
                tar_member: Some(src.file_name().unwrap().to_string_lossy().into_owned())
            }
        });

        Ok(())
    }

    pub fn ingest_blobs_dir(&mut self) -> Result<(), anyhow::Error> {
        let blob_dir = self.backup_dir()?.join("blobs");
        if !blob_dir.exists() {
            return Ok(());
        }
        for entry in WalkDir::new(blob_dir).min_depth(1).max_depth(1) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let p = entry.path();
            let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            if !(fname.ends_with(".tar") || fname.ends_with(".tar.zst")) {
                continue;
            }
            let bytes = fs::read(p)?;

            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let id = fname.split('.').next().unwrap_or_default().to_string();

            let format = if fname.ends_with(".tar.zst") {
                "tar.zst"
            } else {
                "tar"
            };

            self.blobs
                .insert(id, BlobPayload::new(format.into(), &bytes));
        }
        Ok(())
    }

    pub fn restore_blob_to(&self, entry: &Entry, dest: &Path) -> Result<(), anyhow::Error> {
        let blob = self
            .blobs
            .get(&entry.blob_id)
            .ok_or_else(|| anyhow!("blob_id não encontrado no manifest: {}", entry.blob_id))?;

        let raw = blob
            .decode()
            .context("falha ao decodificar base64 do blob")?;

        let tar_bytes: Vec<u8> = match blob.get_format() {
            "tar" => raw,
            "tar.zst" => {
                zstd::stream::decode_all(&raw[..]).context("falha ao descomprimir zstd")?
            }
            other => return Err(anyhow!("formato de blob desconhecido: {}", other)),
        };

        fs::create_dir_all(
            dest.parent()
                .ok_or_else(|| anyhow!("dest sem parent: {}", dest.display()))?,
        )?;

        let mut ar = tar::Archive::new(&tar_bytes[..]);

        let member_name = entry
            .tar_member
            .as_ref()
            .ok_or_else(|| anyhow!("extract_mode=file requer tar_member"))?;
        // Procurar o membro e extrair para dest exato
        let mut found = false;
        for f in ar.entries()? {
            let mut f = f?;
            let path = f.path()?;
            if path.as_os_str().to_string_lossy() == *member_name {
                // extrair para arquivo temporário e mover para dest
                let tmp = dest.with_extension("tmp.part");
                {
                    let mut out = fs::File::create(&tmp)?;
                    std::io::copy(&mut f, &mut out)?;
                }
                fs::rename(tmp, &dest)?;
                found = true;
                break;
            }
        }
        if !found {
            return Err(anyhow!(
                "membro '{}' não encontrado no TAR do blob {}",
                member_name,
                entry.blob_id
            ));
        }

        Ok(())
    }

    // Blockchain integrity methods
    pub fn calculate_backup_hash(&self) -> Result<String, anyhow::Error> {
        // Create a canonical representation of the backup for hashing
        let mut hasher = Sha256::new();
        
        // Hash the manifest metadata (excluding chain fields to avoid circular dependency)
        hasher.update(self.name.as_bytes());
        hasher.update(self.created_at.as_bytes());
        hasher.update(self.os_source.as_bytes());
        
        // Hash entries in a deterministic order
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by(|a, b| a.blob_id.cmp(&b.blob_id));
        for entry in &sorted_entries {
            hasher.update(entry.blob_id.as_bytes());
            hasher.update(entry.target_hint.as_bytes());
            hasher.update(entry.logical_path.as_bytes());
            if let Some(tar_member) = &entry.tar_member {
                hasher.update(tar_member.as_bytes());
            }
        }
        
        // Hash blobs in a deterministic order
        let mut sorted_blob_ids: Vec<_> = self.blobs.keys().collect();
        sorted_blob_ids.sort();
        for blob_id in sorted_blob_ids {
            let blob = &self.blobs[blob_id];
            hasher.update(blob_id.as_bytes());
            hasher.update(blob.get_format().as_bytes());
            hasher.update(blob.get_sha256().as_bytes());
            hasher.update(&blob.get_size().to_le_bytes());
        }
        
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn set_previous_backup(&mut self, previous_backup_name: &str) -> Result<(), anyhow::Error> {
        // Load the previous backup and get its chain hash
        let previous_manifest = Self::load_from(previous_backup_name)?;
        let previous_hash = previous_manifest.backup_chain_hash
            .ok_or_else(|| anyhow!("Previous backup has no chain hash"))?;
        
        self.previous_backup_hash = Some(previous_hash);
        Ok(())
    }

    pub fn finalize_chain_hash(&mut self) -> Result<(), anyhow::Error> {
        let mut hasher = Sha256::new();
        
        // Include previous backup hash if available
        if let Some(prev_hash) = &self.previous_backup_hash {
            hasher.update(prev_hash.as_bytes());
        }
        
        // Include current backup hash
        let current_hash = self.calculate_backup_hash()?;
        hasher.update(current_hash.as_bytes());
        
        self.backup_chain_hash = Some(hex::encode(hasher.finalize()));
        Ok(())
    }

    pub fn verify_backup_integrity(&self) -> Result<bool, anyhow::Error> {
        // Recalculate the backup hash and compare with stored chain hash
        let calculated_hash = self.calculate_backup_hash()?;
        
        // Verify chain hash
        let mut hasher = Sha256::new();
        if let Some(prev_hash) = &self.previous_backup_hash {
            hasher.update(prev_hash.as_bytes());
        }
        hasher.update(calculated_hash.as_bytes());
        let expected_chain_hash = hex::encode(hasher.finalize());
        
        match &self.backup_chain_hash {
            Some(stored_hash) => Ok(*stored_hash == expected_chain_hash),
            None => Ok(false), // No chain hash means not properly initialized
        }
    }

    pub fn verify_chain_from(&self, start_backup_name: &str) -> Result<bool, anyhow::Error> {
        let mut current_backup_name = start_backup_name.to_string();
        let mut visited = std::collections::HashSet::new();
        
        loop {
            // Prevent infinite loops
            if !visited.insert(current_backup_name.clone()) {
                return Err(anyhow!("Circular reference detected in backup chain"));
            }
            
            // Load current backup
            let manifest = if current_backup_name == self.name {
                self.clone() // Use current instance if it's the same backup
            } else {
                Self::load_from(&current_backup_name)?
            };
            
            // Verify this backup's integrity
            if !manifest.verify_backup_integrity()? {
                return Ok(false);
            }
            
            // Move to next backup in chain
            match &manifest.previous_backup_hash {
                Some(_) => {
                    // Find the backup that has the matching chain hash
                    let storage_dir = Self::base_storage_dir()?;
                    if !storage_dir.exists() {
                        return Ok(true); // No more backups to verify
                    }
                    
                    let mut found_previous = false;
                    for entry in fs::read_dir(storage_dir)? {
                        let entry = entry?;
                        if !entry.file_type()?.is_dir() {
                            continue;
                        }
                        
                        let manifest_path = entry.path().join("manifest.json");
                        if !manifest_path.exists() {
                            continue;
                        }
                        
                        let prev_manifest = Self::load_from(
                            entry.file_name().to_string_lossy().as_ref()
                        )?;
                        
                        if Some(&prev_manifest.backup_chain_hash.unwrap_or_default()) == manifest.previous_backup_hash.as_ref() {
                            current_backup_name = prev_manifest.name.clone();
                            found_previous = true;
                            break;
                        }
                    }
                    
                    if !found_previous {
                        return Err(anyhow!("Broken chain: previous backup not found"));
                    }
                }
                None => break, // Reached the end of the chain
            }
        }
        
        Ok(true)
    }

    pub fn list_all_backups_sorted() -> Result<Vec<String>, anyhow::Error> {
        let storage_dir = Self::base_storage_dir()?;
        let mut backups = Vec::new();
        
        if !storage_dir.exists() {
            return Ok(backups);
        }
        
        for entry in fs::read_dir(storage_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    backups.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        
        // Sort by creation time
        backups.sort_by(|a, b| {
            let manifest_a = Self::load_from(a).unwrap_or_else(|_| Self::new(a.clone(), "".to_string(), "".to_string()));
            let manifest_b = Self::load_from(b).unwrap_or_else(|_| Self::new(b.clone(), "".to_string(), "".to_string()));
            manifest_a.created_at.cmp(&manifest_b.created_at)
        });
        
        Ok(backups)
    }
}
