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

#[derive(Serialize, Deserialize, Debug)]
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

    pub fn create_blob_from_file(
        &mut self,
        src: &Path,
        target_hint: &str,
    ) -> Result<(), anyhow::Error> {
        let blob_dir = self.backup_dir()?.join("blobs");
        fs::create_dir_all(&blob_dir)?;

        // Cria TAR na memória
        let mut tar_data = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_data);
            builder.append_path(src)?;
            builder.finish()?;
        }

        // Comprime com zstd
        let compressed = encode_all(&tar_data[..], 3)?;

        // SHA256
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let id = hex::encode(hasher.finalize());

        // Salva no disco
        let blob_path = blob_dir.join(format!("{id}.tar.zst"));
        if !blob_path.exists() {
            fs::write(&blob_path, &compressed)?;
        }
        
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
}
