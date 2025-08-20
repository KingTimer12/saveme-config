use rayon::prelude::*;
use std::io::Write;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{anyhow, Context, Ok};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::Builder;
use walkdir::WalkDir;
use zstd::encode_all;

use crate::storage::{
    blob_chain::BlobChainManager,
    blobs::BlobPayload,
    entry::Entry,
    performance::{MemoryOperation, WorkComplexity, PERFORMANCE_CONFIG, PERFORMANCE_METRICS},
};

/// Thread pool configuration for optimal performance
static THREAD_POOL_INIT: std::sync::Once = std::sync::Once::new();

/// Memory threshold constants for optimization decisions
const SMALL_FILE_THRESHOLD: usize = 1_000_000; // 1MB
const LARGE_FILE_THRESHOLD: usize = 10_000_000; // 10MB
const HUGE_FILE_THRESHOLD: usize = 50_000_000; // 50MB
const PARALLEL_BATCH_SIZE: usize = 100; // Max files per batch
const COMPRESSION_BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub created_at: String,
    pub os_source: String,
    pub entries: Vec<Entry>,
    pub blobs: HashMap<String, BlobPayload>,
}

#[derive(Debug, Clone)]
pub struct EstimatedPerformance {
    pub estimated_time_seconds: f64,
    pub estimated_throughput_mbps: f64,
    pub estimated_dedup_saves: usize,
    pub memory_usage_mb: usize,
}

impl Manifest {
    /// Initialize optimized thread pool for file operations
    fn init_thread_pool() {
        THREAD_POOL_INIT.call_once(|| {
            let config = &*PERFORMANCE_CONFIG;
            let stack_size = 8 * 1024 * 1024; // 8MB stack size for large operations

            rayon::ThreadPoolBuilder::new()
                .num_threads(config.thread_count)
                .stack_size(stack_size)
                .thread_name(|index| format!("saveme-worker-{}", index))
                .build_global()
                .expect("Failed to initialize thread pool");

            println!(
                "Initialized optimized thread pool with {} workers (max memory: {}MB)",
                config.thread_count, config.max_memory_mb
            );
        });
    }

    /// Get optimal chunk size based on data size and configuration
    fn get_optimal_chunk_size(total_size: usize, min_chunk_size: usize) -> usize {
        let config = &*PERFORMANCE_CONFIG;
        let optimal_size = config.get_optimal_chunk_size(total_size);
        optimal_size.max(min_chunk_size).min(total_size / 2)
    }

    /// Memory-efficient compression with adaptive strategy
    fn adaptive_compress(data: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let config = &*PERFORMANCE_CONFIG;
        let size = data.len();
        let level = config.get_adaptive_compression_level(size);

        let strategy = match level {
            1..=6 => "ultra_fast",
            7..=12 => "fast",
            13..=17 => "balanced",
            _ => "max_compression",
        };

        println!(
            "Using {} compression (level {}) for {}MB file",
            strategy,
            level,
            size / 1024 / 1024
        );

        // Track performance metrics
        let start = Instant::now();
        let result = encode_all(data, level).map_err(|e| anyhow!("Compression failed: {}", e))?;

        PERFORMANCE_METRICS.add_bytes_compressed(size);
        PERFORMANCE_METRICS.add_compression_time(start.elapsed().as_millis() as usize);

        Ok(result)
    }

    pub fn new(name: String, created_at: String, os_source: String) -> Self {
        Self::init_thread_pool();
        Self {
            name,
            created_at,
            os_source,
            entries: Vec::new(),
            blobs: HashMap::new(),
        }
    }

    pub fn empty(name: String) -> Self {
        Self {
            name,
            created_at: "".to_string(),
            os_source: "".to_string(),
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
        let manifest = Self::empty(name.to_string());
        manifest.load()
    }

    pub fn load(&self) -> Result<Self, anyhow::Error> {
        let manifest_path = Self::base_storage_dir()?
            .join(&self.name)
            .join("manifest.json");
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

    pub fn find_existing_blob_across_backups(
        content_hash: &str,
    ) -> Result<Option<(String, String)>, anyhow::Error> {
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

    /// Enhanced parallel compression with memory optimization
    fn parallel_compress_worker(data_chunks: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>, anyhow::Error> {
        let config = &*PERFORMANCE_CONFIG;
        let num_threads = config.thread_count.min(data_chunks.len()).max(1);
        println!(
            "Using {} threads for optimized parallel compression",
            num_threads
        );

        // Use different compression levels based on chunk size for optimal performance
        let start = Instant::now();
        let results = data_chunks
            .into_par_iter()
            .enumerate()
            .map(|(index, chunk)| {
                let compression_level = config.get_adaptive_compression_level(chunk.len());

                encode_all(&chunk[..], compression_level)
                    .map_err(|e| anyhow!("Compression failed for chunk {}: {}", index, e))
            })
            .collect();

        // Track metrics
        let total_time = start.elapsed().as_millis() as usize;
        PERFORMANCE_METRICS.add_compression_time(total_time);

        results
    }

    /// Smart memory management for large data processing
    fn process_with_memory_limit<T, F, R>(
        items: Vec<T>,
        max_memory_mb: usize,
        processor: F,
    ) -> Result<Vec<R>, anyhow::Error>
    where
        T: Send + Sync,
        R: Send,
        F: Fn(&T) -> Result<R, anyhow::Error> + Send + Sync,
    {
        let available_memory = max_memory_mb * 1024 * 1024;
        let estimated_item_size = available_memory / (items.len().max(1));
        let batch_size = (available_memory / estimated_item_size.max(1)).min(PARALLEL_BATCH_SIZE);

        println!(
            "Processing {} items in batches of {} (memory limit: {}MB)",
            items.len(),
            batch_size,
            max_memory_mb
        );

        let mut results = Vec::with_capacity(items.len());

        for chunk in items.chunks(batch_size) {
            let chunk_results: Vec<R> = chunk
                .iter()
                .map(|item| processor(item))
                .collect::<Result<Vec<_>, _>>()?;

            results.extend(chunk_results);
        }

        Ok(results)
    }

    /// Enhanced file processing with parallel I/O and compression
    fn process_files_parallel(
        files: Vec<walkdir::DirEntry>,
        src_base: &Path,
    ) -> Result<Vec<(Vec<u8>, PathBuf, PathBuf)>, anyhow::Error> {
        let start_time = Instant::now();
        let config = &*PERFORMANCE_CONFIG;
        let optimal_workers = crate::storage::performance::utils::calculate_optimal_workers(
            files.len(),
            WorkComplexity::Medium,
        );

        println!(
            "Processing {} files with {} workers (memory limit: {}MB)",
            files.len(),
            optimal_workers,
            config.max_memory_mb
        );

        let results: Result<Vec<_>, anyhow::Error> = files
            .into_par_iter()
            .map(
                |entry| -> Result<(Vec<u8>, PathBuf, PathBuf), anyhow::Error> {
                    let path = entry.path().to_path_buf();
                    let relative_path = path.strip_prefix(src_base)?.to_path_buf();

                    // Check memory usage before reading large files
                    let file_size = fs::metadata(&path)?.len() as usize;
                    let estimated_memory =
                        crate::storage::performance::utils::estimate_memory_usage(
                            file_size,
                            MemoryOperation::TarCreation,
                        );

                    if !crate::storage::performance::utils::check_memory_limit(estimated_memory) {
                        return Err(anyhow!(
                            "File too large for memory limit: {}",
                            path.display()
                        ));
                    }

                    // Read file in parallel
                    let data = fs::read(&path)
                        .with_context(|| format!("Failed to read file: {}", path.display()))?;

                    PERFORMANCE_METRICS.add_file_processed();
                    Ok((data, path, relative_path))
                },
            )
            .collect();

        let elapsed = start_time.elapsed();
        println!("File processing completed in {:?}", elapsed);
        results
    }

    /// Batch processing for multiple files with optimal threading
    pub fn create_blobs_from_files_batch(
        &mut self,
        file_paths: Vec<(PathBuf, String)>, // (path, target_hint) pairs
    ) -> Result<Vec<String>, anyhow::Error> {
        let start_time = Instant::now();
        let num_files = file_paths.len();
        println!("Starting batch processing for {} files", num_files);

        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        // Process files in parallel chunks for optimal memory usage
        let chunk_size = (num_files / num_cpus::get()).max(1).min(50); // Max 50 files per chunk
        let chunks: Vec<_> = file_paths.chunks(chunk_size).collect();

        // Since we can't borrow self mutably in parallel closure, process sequentially for now
        // TODO: Refactor to use message passing or other concurrent pattern for true parallelization
        let mut all_blob_ids = Vec::new();

        for chunk in chunks {
            let mut chunk_blob_ids = Vec::new();

            for (path, target_hint) in chunk {
                let blob_id = self.create_single_file_blob_optimized(&path, &target_hint)?;
                chunk_blob_ids.push(blob_id);
            }

            all_blob_ids.extend(chunk_blob_ids);
        }

        let blob_ids = Ok(vec![all_blob_ids]);

        let all_blob_ids: Vec<String> = blob_ids?.into_iter().flatten().collect();

        let total_time = start_time.elapsed();
        println!(
            "Batch processing completed: {} files in {:?} ({:.2} files/sec)",
            num_files,
            total_time,
            num_files as f64 / total_time.as_secs_f64()
        );

        Ok(all_blob_ids)
    }

    /// Optimized single file blob creation for batch processing
    fn create_single_file_blob_optimized(
        &mut self,
        src: &Path,
        target_hint: &str,
    ) -> Result<String, anyhow::Error> {
        let blob_dir = self.backup_dir()?.join("blobs");
        fs::create_dir_all(&blob_dir)?;

        // Read and create TAR in memory with optimized buffer
        let file_size = fs::metadata(src)?.len();
        let mut tar_data = Vec::with_capacity((file_size * 110 / 100) as usize); // 10% overhead estimate

        {
            let mut builder = Builder::new(&mut tar_data);
            let file_name = src
                .file_name()
                .ok_or_else(|| anyhow!("Invalid file name"))?;
            builder.append_path_with_name(src, file_name)?;
            builder.finish()?;
        }

        // Optimized compression based on file size
        let compressed = if tar_data.len() > 5_000_000 {
            // 5MB threshold for batch processing
            encode_all(&tar_data[..], 15)? // Faster compression for batch
        } else {
            encode_all(&tar_data[..], 19)? // Max compression for small files
        };

        // Quick hash calculation
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let content_hash = hex::encode(hasher.finalize());

        // Check for duplicates (optimized for batch)
        if let Some((_, existing_blob_id)) = Self::find_existing_blob_across_backups(&content_hash)?
        {
            self.entries.push(Entry {
                blob_id: existing_blob_id.clone(),
                target_hint: target_hint.to_string(),
                logical_path: src.to_string_lossy().into_owned(),
                tar_member: Some(src.file_name().unwrap().to_string_lossy().into_owned()),
            });
            return Ok(existing_blob_id);
        }

        let id = content_hash;

        // Write blob to disk
        let blob_path = blob_dir.join(format!("{id}.tar.zst"));
        if !blob_path.exists() {
            fs::write(&blob_path, &compressed)?;
        }

        // Create and chain blob
        let mut blob = BlobPayload::new("tar.zst".to_string(), &compressed);
        let storage_dir = Self::base_storage_dir()?;
        let mut chain_manager = BlobChainManager::new(storage_dir, self.name.clone())?;

        let chain_info = chain_manager.get_chain_info();
        if let Some(latest_id) = chain_info.chain_order.last() {
            blob.set_previous_blob_hash(Some(latest_id.clone()));
        }

        chain_manager.add_blob_to_chain(&id, &mut blob)?;
        self.add_blob_for_testing(id.clone(), blob);

        self.entries.push(Entry {
            blob_id: id.clone(),
            target_hint: target_hint.to_string(),
            logical_path: src.to_string_lossy().into_owned(),
            tar_member: Some(src.file_name().unwrap().to_string_lossy().into_owned()),
        });

        Ok(id)
    }

    /// Batch restore with parallel processing
    pub fn restore_blobs_batch(
        &self,
        entries_with_dest: Vec<(&Entry, PathBuf)>,
    ) -> Result<(), anyhow::Error> {
        let start_time = Instant::now();
        let num_entries = entries_with_dest.len();
        println!("Starting batch restore for {} entries", num_entries);

        if entries_with_dest.is_empty() {
            return Ok(());
        }

        // Process restores in parallel
        let results: Result<Vec<_>, anyhow::Error> = entries_with_dest
            .into_par_iter()
            .map(|(entry, dest)| -> Result<(), anyhow::Error> {
                self.restore_blob_to(entry, &dest)
            })
            .collect();

        results?;

        let total_time = start_time.elapsed();
        println!(
            "Batch restore completed: {} entries in {:?} ({:.2} entries/sec)",
            num_entries,
            total_time,
            num_entries as f64 / total_time.as_secs_f64()
        );

        Ok(())
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
            let file_name = src
                .file_name()
                .ok_or_else(|| anyhow!("Invalid file name"))?;
            builder.append_path_with_name(src, file_name)?;
            builder.finish()?;
        }
        println!("Created TAR archive");

        // Use adaptive compression strategy based on configuration
        println!("Compressing TAR archive with adaptive strategy");
        let start_time = Instant::now();
        let config = &*PERFORMANCE_CONFIG;

        let compressed = if config.should_use_parallel(tar_data.len()) {
            // For large files, use parallel chunk compression
            let chunk_size = Self::get_optimal_chunk_size(tar_data.len(), COMPRESSION_BUFFER_SIZE);
            let chunks: Vec<Vec<u8>> = tar_data
                .chunks(chunk_size)
                .map(|chunk| chunk.to_vec())
                .collect();

            println!(
                "Large file detected ({}MB), using {} optimized parallel compression chunks",
                tar_data.len() / 1024 / 1024,
                chunks.len()
            );

            let compressed_chunks = Self::parallel_compress_worker(chunks)?;
            compressed_chunks.into_iter().flatten().collect()
        } else {
            // For smaller files, use adaptive single-thread compression
            Self::adaptive_compress(&tar_data)?
        };

        let compression_time = start_time.elapsed();
        let compression_ratio = tar_data.len() as f64 / compressed.len() as f64;
        println!(
            "Compression completed in {:?}, ratio: {:.2}x",
            compression_time, compression_ratio
        );

        // SHA256 do conteúdo comprimido para verificar duplicação
        println!("Calculating SHA256 hash for deduplication");
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let content_hash = hex::encode(hasher.finalize());

        // Verificar se o blob já existe (deduplicação)
        println!("Checking for existing blob with same content");
        if let Some((existing_backup, existing_blob_id)) =
            Self::find_existing_blob_across_backups(&content_hash)?
        {
            println!(
                "Found duplicate content in backup '{}' with blob ID '{}'",
                existing_backup, existing_blob_id
            );

            // Usar referência do blob existente ao invés de criar novo
            self.entries.push({
                Entry {
                    blob_id: existing_blob_id,
                    target_hint: target_hint.to_string(),
                    logical_path: src.to_string_lossy().into_owned(),
                    tar_member: Some(src.file_name().unwrap().to_string_lossy().into_owned()),
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

        // Create blob and determine previous blob hash
        let mut blob = BlobPayload::new("tar.zst".to_string(), &compressed);

        // Initialize blob chain manager and add blob to chain
        let storage_dir = Self::base_storage_dir()?;
        let mut chain_manager = BlobChainManager::new(storage_dir, self.name.clone())?;

        let chain_info = chain_manager.get_chain_info();
        if let Some(latest_id) = chain_info.chain_order.last() {
            println!(
                "Setting previous_blob_hash to latest chain id: {}",
                latest_id
            );
            blob.set_previous_blob_hash(Some(latest_id.clone()));
        } else {
            // no chain yet — leave previous as None (genesis)
            println!("No existing chain found; this blob will be genesis");
        }

        chain_manager.add_blob_to_chain(&id, &mut blob)?;

        println!("Added blob to blockchain");

        // Adicionar blob ao manifest atual
        self.add_blob_for_testing(id.clone(), blob);

        self.entries.push({
            Entry {
                blob_id: id,
                target_hint: target_hint.to_string(),
                logical_path: src.to_string_lossy().into_owned(),
                tar_member: Some(src.file_name().unwrap().to_string_lossy().into_owned()),
            }
        });

        Ok(())
    }

    pub fn create_blob_from_directory(
        &mut self,
        src: &Path,
        target_hint: &str,
    ) -> Result<(), anyhow::Error> {
        let blob_dir = self.backup_dir()?.join("blobs");
        println!("Creating blob from directory");
        fs::create_dir_all(&blob_dir)?;
        println!("Created blob directory in {}", blob_dir.display());

        // Cria TAR na memória
        println!("Creating TAR archive from directory");
        let mut tar_data = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_data);

            // Collect all entries first
            let entries: Result<Vec<_>, _> = WalkDir::new(src).into_iter().collect();
            let entries = entries?;

            // Separate files and directories for different processing
            let (files, dirs): (Vec<_>, Vec<_>) = entries
                .into_iter()
                .partition(|entry| entry.file_type().is_file());

            // Process directories first (são rápidos)
            for entry in dirs {
                let path = entry.path();
                let relative_path = path.strip_prefix(src)?;

                if entry.file_type().is_dir() && relative_path != Path::new("") {
                    builder.append_dir(relative_path, path)?;
                }
            }

            // Processa arquivos em paralelo (leitura e preparação)
            let mut sorted_files = Self::process_files_parallel(files, src)?;
            // Ordena para balanceamento
            sorted_files.sort_by_key(|(data, _, _)| std::cmp::Reverse(data.len()));

            println!("Adding {} files to TAR archive", sorted_files.len());

            // Escreve os arquivos sequencialmente no TAR
            for (file_data, _, relative_path) in sorted_files {
                let mut header = tar::Header::new_gnu();
                header.set_size(file_data.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();

                builder.append_data(&mut header, &relative_path, &file_data[..])?;
            }

            // Finaliza TAR
            builder.finish()?;
        }
        println!("Created TAR archive from directory");

        // Use enhanced adaptive compression for directories
        println!("Compressing directory TAR with enhanced adaptive strategy");
        let start_time = Instant::now();
        let config = &*PERFORMANCE_CONFIG;

        let compressed = if config.should_use_parallel(tar_data.len()) {
            // For huge directories, use optimized parallel compression
            let chunk_size =
                Self::get_optimal_chunk_size(tar_data.len(), COMPRESSION_BUFFER_SIZE * 4);
            let chunks: Vec<Vec<u8>> = tar_data
                .chunks(chunk_size)
                .map(|chunk| chunk.to_vec())
                .collect();

            println!(
                "Large directory detected ({}MB), using {} adaptive parallel compression chunks",
                tar_data.len() / 1024 / 1024,
                chunks.len()
            );

            let compressed_chunks = Self::parallel_compress_worker(chunks)?;
            compressed_chunks.into_iter().flatten().collect()
        } else {
            // For smaller directories, use adaptive compression
            Self::adaptive_compress(&tar_data)?
        };

        let compression_time = start_time.elapsed();
        let compression_ratio = tar_data.len() as f64 / compressed.len() as f64;
        let throughput = tar_data.len() as f64 / (1024.0 * 1024.0) / compression_time.as_secs_f64();

        println!(
            "Directory compression completed in {:?}, ratio: {:.2}x, throughput: {:.2} MB/s",
            compression_time, compression_ratio, throughput
        );

        // SHA256 do conteúdo comprimido para verificar duplicação
        println!("Calculating SHA256 hash for deduplication");
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let content_hash = hex::encode(hasher.finalize());

        // Verificar se o blob já existe (deduplicação)
        println!("Checking for existing blob with same content");
        if let Some((existing_backup, existing_blob_id)) =
            Self::find_existing_blob_across_backups(&content_hash)?
        {
            println!(
                "Found duplicate content in backup '{}' with blob ID '{}'",
                existing_backup, existing_blob_id
            );

            // Usar referência do blob existente ao invés de criar novo
            self.entries.push({
                Entry {
                    blob_id: existing_blob_id,
                    target_hint: target_hint.to_string(),
                    logical_path: src.to_string_lossy().into_owned(),
                    tar_member: None, // Para diretórios, não há membro específico
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

        // Create blob and determine previous blob hash
        let mut blob = BlobPayload::new("tar.zst".to_string(), &compressed);

        // Initialize blob chain manager and add blob to chain
        let storage_dir = Self::base_storage_dir()?;
        let mut chain_manager = BlobChainManager::new(storage_dir, self.name.clone())?;

        let chain_info = chain_manager.get_chain_info();
        if let Some(latest_id) = chain_info.chain_order.last() {
            println!(
                "Setting previous_blob_hash to latest chain id: {}",
                latest_id
            );
            blob.set_previous_blob_hash(Some(latest_id.clone()));
        } else {
            // no chain yet — leave previous as None (genesis)
            println!("No existing chain found; this blob will be genesis");
        }

        chain_manager.add_blob_to_chain(&id, &mut blob)?;

        println!("Added blob to blockchain");

        // Adicionar blob ao manifest atual
        self.add_blob_for_testing(id.clone(), blob);

        self.entries.push({
            Entry {
                blob_id: id,
                target_hint: target_hint.to_string(),
                logical_path: src.to_string_lossy().into_owned(),
                tar_member: None, // Para diretórios, não há membro específico
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
            // let id = fname.split('.').next().unwrap_or_default().to_string();

            // let format = if fname.ends_with(".tar.zst") {
            //     "tar.zst"
            // } else {
            //     "tar"
            // };

            // self.add_blob_for_testing(id, BlobPayload::new(format.into(), &bytes));
        }
        Ok(())
    }

    pub fn restore_blob_to(&self, entry: &Entry, dest: &Path) -> Result<(), anyhow::Error> {
        let start_time = Instant::now();
        let blob = self
            .blobs
            .get(&entry.blob_id)
            .ok_or_else(|| anyhow!("blob_id não encontrado no manifest: {}", entry.blob_id))?;

        let raw = blob
            .decode()
            .context("falha ao decodificar base64 do blob")?;

        println!("Starting decompression for blob: {}", entry.blob_id);

        let tar_bytes: Vec<u8> = match blob.get_format() {
            "tar" => raw,
            "tar.zst" => {
                // Use parallel decompression for large compressed data
                if raw.len() > 20_000_000 {
                    // 20MB threshold
                    println!("Large compressed blob detected, using optimized decompression");

                    // For very large files, use streaming decompression with buffer optimization
                    let mut decoder = zstd::stream::Decoder::new(&raw[..])?;
                    let mut decompressed = Vec::new();

                    // Use larger buffer for better I/O performance
                    let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
                    loop {
                        match decoder.read(&mut buffer) {
                            std::result::Result::Ok(0) => break, // EOF
                            std::result::Result::Ok(n) => {
                                decompressed.extend_from_slice(&buffer[..n])
                            }
                            std::result::Result::Err(e) => {
                                return Err(anyhow!("Decompression error: {}", e))
                            }
                        }
                    }
                    decompressed
                } else {
                    zstd::stream::decode_all(&raw[..]).context("falha ao descomprimir zstd")?
                }
            }
            other => return Err(anyhow!("formato de blob desconhecido: {}", other)),
        };

        let decompression_time = start_time.elapsed();
        println!("Decompression completed in {:?}", decompression_time);

        fs::create_dir_all(
            dest.parent()
                .ok_or_else(|| anyhow!("dest sem parent: {}", dest.display()))?,
        )?;

        let mut ar = tar::Archive::new(&tar_bytes[..]);

        let member_name = entry
            .tar_member
            .as_ref()
            .ok_or_else(|| anyhow!("extract_mode=file requer tar_member"))?;

        // Optimized member search with early exit
        println!("Searching for member: {}", member_name);
        let mut found = false;
        for f in ar.entries()? {
            let mut f = f?;
            let path = f.path()?;
            if path.as_os_str().to_string_lossy() == *member_name {
                // Use parallel I/O for large files during extraction
                let tmp = dest.with_extension("tmp.part");

                let file_size = f.header().size().unwrap_or(0);
                if file_size > 10_000_000 {
                    // 10MB threshold
                    println!("Large file extraction detected, using optimized I/O");

                    // Use buffered writing for better performance
                    let mut out = std::io::BufWriter::with_capacity(
                        1024 * 1024, // 1MB buffer
                        fs::File::create(&tmp)?,
                    );
                    std::io::copy(&mut f, &mut out)?;
                    out.flush()?;
                } else {
                    let mut out = fs::File::create(&tmp)?;
                    std::io::copy(&mut f, &mut out)?;
                }

                fs::rename(tmp, &dest)?;
                found = true;
                break;
            }
        }

        /// Performance estimation result
        #[derive(Debug, Clone)]
        pub struct EstimatedPerformance {
            pub estimated_time_seconds: f64,
            pub estimated_throughput_mbps: f64,
            pub estimated_dedup_saves: usize,
            pub memory_usage_mb: usize,
        }

        impl EstimatedPerformance {
            pub fn print_estimate(&self) {
                println!("\n=== Performance Estimate ===");
                println!(
                    "  Estimated time: {:.1} seconds",
                    self.estimated_time_seconds
                );
                println!(
                    "  Expected throughput: {:.1} MB/s",
                    self.estimated_throughput_mbps
                );
                println!("  Estimated dedup saves: {}", self.estimated_dedup_saves);
                println!("  Memory usage: {}MB", self.memory_usage_mb);
                println!("=============================\n");
            }
        }

        if !found {
            return Err(anyhow!(
                "membro '{}' não encontrado no TAR do blob {}",
                member_name,
                entry.blob_id
            ));
        }

        let total_time = start_time.elapsed();
        println!("Restore completed in {:?}", total_time);
        Ok(())
    }

    /// Get current performance statistics
    pub fn get_performance_stats(&self) -> crate::storage::performance::PerformanceStats {
        PERFORMANCE_METRICS.get_stats()
    }

    /// Print detailed performance report
    pub fn print_performance_report(&self) {
        let stats = self.get_performance_stats();
        let config = &*PERFORMANCE_CONFIG;

        println!("\n=== SaveMe Performance Report ===");
        println!("Configuration:");
        println!("  Thread count: {}", config.thread_count);
        println!("  Max memory: {}MB", config.max_memory_mb);
        println!("  Compression level: {}", config.compression_level);
        println!("  Adaptive compression: {}", config.adaptive_compression);
        println!("  Parallel dedup: {}", config.parallel_dedup);

        println!("\nStatistics:");
        println!("  Files processed: {}", stats.files_processed);
        println!(
            "  Bytes compressed: {:.2}MB",
            stats.bytes_compressed as f64 / (1024.0 * 1024.0)
        );
        println!(
            "  Compression time: {:.2}s",
            stats.compression_time_ms as f64 / 1000.0
        );
        println!(
            "  Throughput: {:.2} MB/s",
            stats.compression_throughput_mbps()
        );
        println!("  Deduplication saves: {}", stats.dedup_saves);
        println!(
            "  Dedup efficiency: {:.1}%",
            stats.dedup_efficiency() * 100.0
        );
        println!("  Cache hit ratio: {:.1}%", stats.cache_hit_ratio() * 100.0);
        println!("================================\n");
    }

    /// Reset performance metrics
    pub fn reset_performance_metrics(&self) {
        PERFORMANCE_METRICS.reset();
        println!("Performance metrics reset");
    }

    /// Get estimated performance for a given workload
    pub fn estimate_performance(
        &self,
        total_files: usize,
        total_size_mb: f64,
    ) -> EstimatedPerformance {
        let config = &*PERFORMANCE_CONFIG;
        let stats = self.get_performance_stats();

        // Base estimates on current performance if available
        let base_throughput = if stats.compression_throughput_mbps() > 0.0 {
            stats.compression_throughput_mbps()
        } else {
            // Default estimates based on configuration
            match config.compression_level {
                1..=6 => 50.0,   // Fast compression
                7..=12 => 30.0,  // Balanced
                13..=17 => 20.0, // Good compression
                _ => 10.0,       // Maximum compression
            }
        };

        let estimated_time_seconds = total_size_mb / base_throughput;
        let estimated_dedup_saves = (total_files as f64 * stats.dedup_efficiency()) as usize;

        EstimatedPerformance {
            estimated_time_seconds,
            estimated_throughput_mbps: base_throughput,
            estimated_dedup_saves,
            memory_usage_mb: (total_size_mb * 0.2) as usize, // Estimate 20% of data size
        }
    }

    pub fn add_blob_for_testing(&mut self, blob_id: String, blob: BlobPayload) {
        self.blobs.insert(blob_id, blob);
    }

    // Blob blockchain integrity methods
    pub fn verify_blob_chain_integrity(&self) -> Result<bool, anyhow::Error> {
        // For testing, allow overriding the storage directory
        self.verify_blob_chain_integrity_with_dir(None)
    }

    pub fn verify_blob_chain_integrity_with_dir(
        &self,
        storage_dir_override: Option<PathBuf>,
    ) -> Result<bool, anyhow::Error> {
        let storage_dir = storage_dir_override.unwrap_or_else(|| Self::base_storage_dir().unwrap());
        let chain_manager = BlobChainManager::new(storage_dir, self.name.clone())?;
        chain_manager.verify_blob_chain(&self.blobs)
    }

    pub fn get_blob_chain_info(&self) -> Result<String, anyhow::Error> {
        self.get_blob_chain_info_with_dir(None)
    }

    pub fn get_blob_chain_info_with_dir(
        &self,
        storage_dir_override: Option<PathBuf>,
    ) -> Result<String, anyhow::Error> {
        let storage_dir = storage_dir_override.unwrap_or_else(|| Self::base_storage_dir().unwrap());
        let chain_manager = BlobChainManager::new(storage_dir, self.name.clone())?;
        let metadata = chain_manager.get_chain_info();
        Ok(format!(
            "Blob chain contains {} blobs with integrity hash: {}",
            metadata.chain_order.len(),
            metadata.chain_integrity_hash
        ))
    }
}
