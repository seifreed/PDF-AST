use crate::performance::{increment_lazy_loads, PerformanceConfig};
use crate::types::StreamReference;
use bytes::Bytes;
use parking_lot::Mutex;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

/// Trait that combines Read and Seek for PDF stream reading
pub trait PdfReader: Read + Seek + Send + std::fmt::Debug {}
impl<T: Read + Seek + Send + std::fmt::Debug> PdfReader for T {}

/// Lazy-loaded stream that reads data on demand
#[derive(Debug, Clone)]
pub struct LazyStream {
    reference: StreamReference,
    reader: Arc<Mutex<Box<dyn PdfReader>>>,
    config: PerformanceConfig,
    cached_data: Arc<Mutex<Option<Bytes>>>,
}

impl LazyStream {
    pub fn new(
        reference: StreamReference,
        reader: Arc<Mutex<Box<dyn PdfReader>>>,
        config: PerformanceConfig,
    ) -> Self {
        Self {
            reference,
            reader,
            config,
            cached_data: Arc::new(Mutex::new(None)),
        }
    }

    /// Load stream data on demand
    pub fn load_data(&self) -> Result<Bytes, String> {
        // Check cache first
        {
            let cached = self.cached_data.lock();
            if let Some(ref data) = *cached {
                return Ok(data.clone());
            }
        }

        // Load data from reader
        let data = self.load_from_reader()?;

        // Cache if under memory limit
        let data_size_mb = data.len() / (1024 * 1024);
        if data_size_mb < self.config.max_memory_mb / 4 {
            // Use 25% of limit for single stream
            let mut cached = self.cached_data.lock();
            *cached = Some(data.clone());
        }

        increment_lazy_loads();
        Ok(data)
    }

    fn load_from_reader(&self) -> Result<Bytes, String> {
        let mut reader = self.reader.lock();

        // Seek to stream position
        reader
            .seek(SeekFrom::Start(self.reference.offset))
            .map_err(|e| format!("Failed to seek to stream: {}", e))?;

        // Read stream data in chunks to avoid large allocations
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; self.config.stream_chunk_size];
        let mut total_read = 0;

        while total_read < self.reference.length {
            let to_read = std::cmp::min(
                self.config.stream_chunk_size,
                self.reference.length - total_read,
            );

            let bytes_read = reader
                .read(&mut chunk[..to_read])
                .map_err(|e| format!("Failed to read stream chunk: {}", e))?;

            if bytes_read == 0 {
                break; // EOF
            }

            buffer.extend_from_slice(&chunk[..bytes_read]);
            total_read += bytes_read;
        }

        Ok(Bytes::from(buffer))
    }

    /// Check if data is currently cached
    pub fn is_cached(&self) -> bool {
        self.cached_data.lock().is_some()
    }

    /// Clear cached data to free memory
    pub fn clear_cache(&self) {
        let mut cached = self.cached_data.lock();
        *cached = None;
    }

    /// Get stream size without loading data
    pub fn size(&self) -> usize {
        self.reference.length
    }
}

/// Manager for lazy-loaded streams
#[derive(Debug)]
pub struct LazyStreamManager {
    streams: dashmap::DashMap<u64, LazyStream>,
    config: PerformanceConfig,
    total_cached_size: Arc<Mutex<usize>>,
}

impl LazyStreamManager {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            streams: dashmap::DashMap::new(),
            config,
            total_cached_size: Arc::new(Mutex::new(0)),
        }
    }

    pub fn register_stream(&self, stream_id: u64, stream: LazyStream) {
        self.streams.insert(stream_id, stream);
    }

    pub fn get_stream(&self, stream_id: u64) -> Option<LazyStream> {
        self.streams.get(&stream_id).map(|entry| entry.clone())
    }

    pub fn load_stream_data(&self, stream_id: u64) -> Result<Bytes, String> {
        let stream = self.streams.get(&stream_id).ok_or("Stream not found")?;

        let data = stream.load_data()?;

        // Update total cached size
        if stream.is_cached() {
            let mut total_size = self.total_cached_size.lock();
            *total_size += data.len();

            // Check memory limits
            let total_mb = *total_size / (1024 * 1024);
            if total_mb > self.config.max_memory_mb {
                self.evict_cached_streams();
            }
        }

        Ok(data)
    }

    /// Evict cached streams when memory limit is exceeded
    fn evict_cached_streams(&self) {
        let mut streams_to_evict = Vec::new();

        // Collect streams that are cached (simple LRU-like eviction)
        for entry in self.streams.iter() {
            if entry.value().is_cached() {
                streams_to_evict.push(*entry.key());
            }
        }

        // Evict half of the cached streams
        let evict_count = streams_to_evict.len() / 2;
        for &stream_id in streams_to_evict.iter().take(evict_count) {
            if let Some(stream) = self.streams.get(&stream_id) {
                let size = stream.size();
                stream.clear_cache();

                let mut total_size = self.total_cached_size.lock();
                *total_size = total_size.saturating_sub(size);
            }
        }

        log::info!("Evicted {} cached streams to free memory", evict_count);
    }

    pub fn clear_all_caches(&self) {
        for entry in self.streams.iter() {
            entry.value().clear_cache();
        }
        *self.total_cached_size.lock() = 0;
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        let cached_count = self
            .streams
            .iter()
            .filter(|entry| entry.value().is_cached())
            .count();
        let total_cached_mb = *self.total_cached_size.lock() / (1024 * 1024);

        (cached_count, total_cached_mb)
    }
}

#[cfg(feature = "async")]
mod async_support {
    use super::*;
    use async_trait::async_trait;

    #[allow(dead_code)]
    #[async_trait]
    pub trait AsyncLazyStream {
        async fn load_data_async(&self) -> Result<Bytes, String>;
    }

    #[async_trait]
    impl AsyncLazyStream for LazyStream {
        async fn load_data_async(&self) -> Result<Bytes, String> {
            // Check cache first
            {
                let cached = self.cached_data.lock();
                if let Some(ref data) = *cached {
                    return Ok(data.clone());
                }
            }

            // For async implementation, we would need AsyncRead + AsyncSeek
            // This is a simplified version - real implementation would use tokio::fs::File
            tokio::task::block_in_place(|| self.load_data())
        }
    }
}
