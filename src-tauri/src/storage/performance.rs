use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Performance configuration for optimized backup operations
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Number of threads to use for parallel operations
    pub thread_count: usize,
    /// Maximum memory usage in MB for operations
    pub max_memory_mb: usize,
    /// Compression level (1-22, higher = better compression but slower)
    pub compression_level: i32,
    /// Buffer size for I/O operations in bytes
    pub io_buffer_size: usize,
    /// Chunk size for parallel processing in bytes
    pub chunk_size: usize,
    /// Enable adaptive compression based on file size
    pub adaptive_compression: bool,
    /// Enable parallel deduplication checks
    pub parallel_dedup: bool,
    /// Maximum files to process in a single batch
    pub max_batch_size: usize,
}

/// Global performance configuration instance
pub static PERFORMANCE_CONFIG: Lazy<PerformanceConfig> =
    Lazy::new(|| PerformanceConfig::auto_detect());

/// Performance metrics tracking
pub struct PerformanceMetrics {
    pub total_files_processed: AtomicUsize,
    pub total_bytes_compressed: AtomicUsize,
    pub total_compression_time_ms: AtomicUsize,
    pub total_dedup_saves: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_files_processed: AtomicUsize::new(0),
            total_bytes_compressed: AtomicUsize::new(0),
            total_compression_time_ms: AtomicUsize::new(0),
            total_dedup_saves: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }

    pub fn add_file_processed(&self) {
        self.total_files_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_bytes_compressed(&self, bytes: usize) {
        self.total_bytes_compressed
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_compression_time(&self, ms: usize) {
        self.total_compression_time_ms
            .fetch_add(ms, Ordering::Relaxed);
    }

    pub fn add_dedup_save(&self) {
        self.total_dedup_saves.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            files_processed: self.total_files_processed.load(Ordering::Relaxed),
            bytes_compressed: self.total_bytes_compressed.load(Ordering::Relaxed),
            compression_time_ms: self.total_compression_time_ms.load(Ordering::Relaxed),
            dedup_saves: self.total_dedup_saves.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.total_files_processed.store(0, Ordering::Relaxed);
        self.total_bytes_compressed.store(0, Ordering::Relaxed);
        self.total_compression_time_ms.store(0, Ordering::Relaxed);
        self.total_dedup_saves.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub files_processed: usize,
    pub bytes_compressed: usize,
    pub compression_time_ms: usize,
    pub dedup_saves: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl PerformanceStats {
    pub fn compression_throughput_mbps(&self) -> f64 {
        if self.compression_time_ms == 0 {
            return 0.0;
        }
        let mb_processed = self.bytes_compressed as f64 / (1024.0 * 1024.0);
        let seconds = self.compression_time_ms as f64 / 1000.0;
        mb_processed / seconds
    }

    pub fn cache_hit_ratio(&self) -> f64 {
        let total_requests = self.cache_hits + self.cache_misses;
        if total_requests == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total_requests as f64
    }

    pub fn dedup_efficiency(&self) -> f64 {
        if self.files_processed == 0 {
            return 0.0;
        }
        self.dedup_saves as f64 / self.files_processed as f64
    }
}

/// Global performance metrics instance
pub static PERFORMANCE_METRICS: Lazy<PerformanceMetrics> = Lazy::new(|| PerformanceMetrics::new());

impl PerformanceConfig {
    /// Auto-detect optimal performance settings based on system capabilities
    pub fn auto_detect() -> Self {
        let cpu_count = num_cpus::get();
        let available_memory = Self::get_available_memory_mb();

        // Calculate optimal thread count (leave some cores for system)
        let thread_count = if cpu_count > 4 {
            cpu_count - 1
        } else {
            cpu_count
        };

        // Calculate memory limits (use up to 50% of available memory)
        let max_memory_mb = (available_memory / 2).max(512).min(8192); // Between 512MB and 8GB

        // Adaptive settings based on system capabilities
        let (compression_level, io_buffer_size, chunk_size) = match cpu_count {
            1..=2 => (15, 256 * 1024, 1024 * 1024), // Low-end: fast compression, small buffers
            3..=4 => (17, 512 * 1024, 2 * 1024 * 1024), // Mid-range: balanced
            5..=8 => (19, 1024 * 1024, 4 * 1024 * 1024), // High-end: better compression
            _ => (19, 2 * 1024 * 1024, 8 * 1024 * 1024), // Server-grade: maximum performance
        };

        println!(
            "Auto-detected performance config: {} threads, {}MB memory, compression level {}",
            thread_count, max_memory_mb, compression_level
        );

        Self {
            thread_count,
            max_memory_mb,
            compression_level,
            io_buffer_size,
            chunk_size,
            adaptive_compression: true,
            parallel_dedup: cpu_count > 2,
            max_batch_size: (cpu_count * 10).min(200).max(20),
        }
    }

    /// Create a custom performance configuration
    pub fn custom(
        thread_count: Option<usize>,
        max_memory_mb: Option<usize>,
        compression_level: Option<i32>,
    ) -> Self {
        let auto_config = Self::auto_detect();

        Self {
            thread_count: thread_count.unwrap_or(auto_config.thread_count),
            max_memory_mb: max_memory_mb.unwrap_or(auto_config.max_memory_mb),
            compression_level: compression_level.unwrap_or(auto_config.compression_level),
            ..auto_config
        }
    }

    /// Create a fast configuration optimized for speed over compression ratio
    pub fn fast() -> Self {
        let auto_config = Self::auto_detect();

        Self {
            compression_level: 6,
            adaptive_compression: true,
            parallel_dedup: true,
            max_batch_size: auto_config.max_batch_size * 2,
            ..auto_config
        }
    }

    /// Create a balanced configuration for general use
    pub fn balanced() -> Self {
        Self::auto_detect()
    }

    /// Create a maximum compression configuration
    pub fn max_compression() -> Self {
        let auto_config = Self::auto_detect();

        Self {
            compression_level: 22,
            adaptive_compression: false,
            max_batch_size: auto_config.max_batch_size / 2,
            ..auto_config
        }
    }

    /// Get adaptive compression level based on file size
    pub fn get_adaptive_compression_level(&self, file_size: usize) -> i32 {
        if !self.adaptive_compression {
            return self.compression_level;
        }

        match file_size {
            0..=1_048_576 => self.compression_level, // < 1MB: use configured level
            1_048_577..=10_485_760 => (self.compression_level - 2).max(6), // 1-10MB: slightly faster
            10_485_761..=104_857_600 => (self.compression_level - 4).max(6), // 10-100MB: faster
            _ => 6, // > 100MB: fast compression
        }
    }

    /// Get optimal chunk size for parallel processing
    pub fn get_optimal_chunk_size(&self, total_size: usize) -> usize {
        let base_chunk_size = self.chunk_size;
        let optimal_chunks = self.thread_count * 2; // 2x threads for better load balancing

        if total_size < base_chunk_size {
            return total_size;
        }

        let calculated_chunk_size = total_size / optimal_chunks;
        calculated_chunk_size
            .max(base_chunk_size / 4)
            .min(base_chunk_size * 4)
    }

    /// Check if parallel processing should be used for given data size
    pub fn should_use_parallel(&self, data_size: usize) -> bool {
        data_size > self.chunk_size && self.thread_count > 1
    }

    /// Get available system memory in MB (fallback to reasonable default)
    fn get_available_memory_mb() -> usize {
        // This is a simplified version - in production you might want to use
        // system-specific APIs to get actual available memory
        match num_cpus::get() {
            1..=2 => 2048, // 2GB for low-end systems
            3..=4 => 4096, // 4GB for mid-range
            5..=8 => 8192, // 8GB for high-end
            _ => 16384,    // 16GB for server-grade
        }
    }

    /// Validate configuration settings
    pub fn validate(&self) -> Result<(), String> {
        if self.thread_count == 0 {
            return Err("Thread count must be greater than 0".to_string());
        }

        if self.max_memory_mb < 128 {
            return Err("Maximum memory must be at least 128MB".to_string());
        }

        if !(1..=22).contains(&self.compression_level) {
            return Err("Compression level must be between 1 and 22".to_string());
        }

        if self.io_buffer_size < 1024 {
            return Err("I/O buffer size must be at least 1KB".to_string());
        }

        if self.chunk_size < self.io_buffer_size {
            return Err("Chunk size must be at least as large as I/O buffer size".to_string());
        }

        if self.max_batch_size == 0 {
            return Err("Maximum batch size must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Utility functions for performance optimization
pub mod utils {
    use super::*;

    /// Calculate optimal number of parallel workers for a given workload
    pub fn calculate_optimal_workers(
        total_work_items: usize,
        work_complexity: WorkComplexity,
    ) -> usize {
        let config = &*PERFORMANCE_CONFIG;
        let base_workers = config.thread_count;

        let complexity_multiplier = match work_complexity {
            WorkComplexity::Low => 1.0,
            WorkComplexity::Medium => 0.8,
            WorkComplexity::High => 0.6,
            WorkComplexity::VeryHigh => 0.4,
        };

        let optimal_workers = (base_workers as f64 * complexity_multiplier) as usize;
        optimal_workers
            .max(1)
            .min(total_work_items)
            .min(base_workers)
    }

    /// Estimate memory usage for a given operation
    pub fn estimate_memory_usage(data_size: usize, operation: MemoryOperation) -> usize {
        match operation {
            MemoryOperation::Compression => {
                // Compression typically needs 2x input size temporarily
                data_size * 2
            }
            MemoryOperation::Decompression => {
                // Decompression might need 3-4x for worst case
                data_size * 4
            }
            MemoryOperation::TarCreation => {
                // TAR creation needs input + tar overhead
                data_size + (data_size / 10) // 10% overhead estimate
            }
            MemoryOperation::Hashing => {
                // Hashing is memory-efficient, minimal overhead
                64 * 1024 // 64KB buffer
            }
        }
    }

    /// Check if operation fits within memory limits
    pub fn check_memory_limit(estimated_usage: usize) -> bool {
        let config = &*PERFORMANCE_CONFIG;
        let limit_bytes = config.max_memory_mb * 1024 * 1024;
        estimated_usage <= limit_bytes
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WorkComplexity {
    Low,      // Simple I/O operations
    Medium,   // Compression/decompression
    High,     // Complex processing with multiple steps
    VeryHigh, // Heavy computational work
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryOperation {
    Compression,
    Decompression,
    TarCreation,
    Hashing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect_config() {
        let config = PerformanceConfig::auto_detect();
        assert!(config.thread_count > 0);
        assert!(config.max_memory_mb >= 512);
        assert!(config.compression_level >= 1 && config.compression_level <= 22);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_adaptive_compression() {
        let config = PerformanceConfig::balanced();

        let small_file_level = config.get_adaptive_compression_level(500_000); // 500KB
        let large_file_level = config.get_adaptive_compression_level(200_000_000); // 200MB

        assert!(large_file_level <= small_file_level);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();

        metrics.add_file_processed();
        metrics.add_bytes_compressed(1024);
        metrics.add_cache_hit();

        let stats = metrics.get_stats();
        assert_eq!(stats.files_processed, 1);
        assert_eq!(stats.bytes_compressed, 1024);
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn test_memory_estimation() {
        let data_size = 1024 * 1024; // 1MB
        let compression_memory =
            utils::estimate_memory_usage(data_size, MemoryOperation::Compression);
        let hash_memory = utils::estimate_memory_usage(data_size, MemoryOperation::Hashing);

        assert!(compression_memory > data_size);
        assert!(hash_memory < data_size);
    }
}
