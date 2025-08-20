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

use crate::storage::{blobs::BlobPayload, entry::Entry, blob_chain::BlobChainManager};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub created_at: String,
    pub os_source: String,
    pub entries: Vec<Entry>,
    blobs: HashMap<String, BlobPayload>,
}

impl Manifest {
    pub fn new(name: String, created_at: String, os_source: String) -> Self {
        Self {
            name,
            created_at,
            os_source,
            entries: Vec::new(),
            blobs: HashMap::new(),
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

        // Create blob and add to chain
        let mut blob = BlobPayload::new("tar.zst".to_string(), &compressed);
        
        // Initialize blob chain manager and add blob to chain
        let storage_dir = Self::base_storage_dir()?;
        let mut chain_manager = BlobChainManager::new(storage_dir)?;
        chain_manager.add_blob_to_chain(&id, &mut blob)?;
        
        println!("Added blob to blockchain");

        // Adicionar blob ao manifest atual
        self.blobs.insert(id.clone(), blob);

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

    pub fn get_blobs(&self) -> &HashMap<String, BlobPayload> {
        &self.blobs
    }

    pub fn add_blob_for_testing(&mut self, blob_id: String, blob: BlobPayload) {
        self.blobs.insert(blob_id, blob);
    }

    // Blob blockchain integrity methods
    pub fn verify_blob_chain_integrity(&self) -> Result<bool, anyhow::Error> {
        // For testing, allow overriding the storage directory
        self.verify_blob_chain_integrity_with_dir(None)
    }

    pub fn verify_blob_chain_integrity_with_dir(&self, storage_dir_override: Option<PathBuf>) -> Result<bool, anyhow::Error> {
        let storage_dir = storage_dir_override.unwrap_or_else(|| Self::base_storage_dir().unwrap());
        let chain_manager = BlobChainManager::new(storage_dir)?;
        chain_manager.verify_blob_chain(&self.blobs)
    }

    pub fn get_blob_chain_info(&self) -> Result<String, anyhow::Error> {
        self.get_blob_chain_info_with_dir(None)
    }

    pub fn get_blob_chain_info_with_dir(&self, storage_dir_override: Option<PathBuf>) -> Result<String, anyhow::Error> {
        let storage_dir = storage_dir_override.unwrap_or_else(|| Self::base_storage_dir().unwrap());
        let chain_manager = BlobChainManager::new(storage_dir)?;
        let metadata = chain_manager.get_chain_info();
        Ok(format!(
            "Blob chain contains {} blobs with integrity hash: {}",
            metadata.chain_order.len(),
            metadata.chain_integrity_hash
        ))
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
