use crate::ast::{AstNode, AstResult, NodeId, NodeType};
use std::collections::HashMap;

/// Chunk-based processing for large PDF documents
pub struct ChunkProcessor {
    chunk_size: usize,
    overlap_size: usize,
    processed_chunks: HashMap<u64, ProcessedChunkInfo>,
    chunk_cache: HashMap<u64, Vec<u8>>,
    max_cache_size: usize,
}

/// Information about a processed chunk
#[derive(Debug, Clone)]
pub struct ProcessedChunkInfo {
    pub offset: u64,
    pub size: usize,
    pub node_count: usize,
    pub processing_time_ms: u64,
    pub error_count: usize,
    pub checksum: u32,
}

/// Configuration for chunk processing
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub chunk_size: usize,
    pub overlap_size: usize,
    pub max_cache_size: usize,
    pub enable_checksums: bool,
    pub parallel_processing: bool,
    pub error_tolerance: f64, // Percentage of errors allowed
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024,  // 64KB
            overlap_size: 4 * 1024, // 4KB overlap
            max_cache_size: 100,    // Cache 100 chunks max
            enable_checksums: true,
            parallel_processing: false,
            error_tolerance: 0.1, // Allow 10% errors
        }
    }
}

impl ChunkProcessor {
    /// Create a new chunk processor
    pub fn new(config: ChunkConfig) -> Self {
        Self {
            chunk_size: config.chunk_size,
            overlap_size: config.overlap_size,
            processed_chunks: HashMap::new(),
            chunk_cache: HashMap::new(),
            max_cache_size: config.max_cache_size,
        }
    }

    /// Process data in chunks
    pub fn process_chunks<R, F>(
        &mut self,
        mut reader: R,
        mut processor: F,
    ) -> AstResult<ChunkProcessingResult>
    where
        R: std::io::Read + std::io::Seek,
        F: FnMut(&[u8], u64) -> AstResult<Vec<AstNode>>,
    {
        let mut result = ChunkProcessingResult::new();
        let mut current_offset = 0u64;
        let mut buffer = vec![0u8; self.chunk_size];
        let mut overlap_buffer = Vec::new();

        loop {
            // Seek to current position
            reader.seek(std::io::SeekFrom::Start(current_offset))?;

            // Read chunk with overlap
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF
            }

            // Combine with overlap from previous chunk
            let mut chunk_data = overlap_buffer.clone();
            chunk_data.extend_from_slice(&buffer[..bytes_read]);

            // Process the chunk
            let start_time = std::time::Instant::now();
            let chunk_result = self.process_single_chunk(
                &chunk_data,
                current_offset.saturating_sub(overlap_buffer.len() as u64),
                &mut processor,
            )?;
            let processing_time = start_time.elapsed().as_millis() as u64;

            // Store chunk information
            let chunk_info = ProcessedChunkInfo {
                offset: current_offset,
                size: chunk_data.len(),
                node_count: chunk_result.nodes.len(),
                processing_time_ms: processing_time,
                error_count: chunk_result.errors.len(),
                checksum: self.calculate_checksum(&chunk_data),
            };

            self.processed_chunks
                .insert(current_offset, chunk_info.clone());
            result.add_chunk_result(chunk_result, chunk_info);

            // Cache the chunk if enabled
            if self.chunk_cache.len() < self.max_cache_size {
                self.chunk_cache.insert(current_offset, chunk_data.clone());
            }

            // Prepare overlap for next chunk
            if bytes_read >= self.overlap_size {
                overlap_buffer = buffer[bytes_read - self.overlap_size..bytes_read].to_vec();
            } else {
                overlap_buffer = buffer[..bytes_read].to_vec();
            }

            // Move to next chunk
            current_offset += bytes_read as u64 - self.overlap_size as u64;

            // Stop if we didn't read a full chunk (probably EOF)
            if bytes_read < self.chunk_size {
                break;
            }
        }

        result.finalize();
        Ok(result)
    }

    /// Process a single chunk of data
    fn process_single_chunk<F>(
        &self,
        data: &[u8],
        offset: u64,
        processor: &mut F,
    ) -> AstResult<SingleChunkResult>
    where
        F: FnMut(&[u8], u64) -> AstResult<Vec<AstNode>>,
    {
        let mut result = SingleChunkResult {
            nodes: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Try to process the entire chunk
        match processor(data, offset) {
            Ok(nodes) => {
                result.nodes = nodes;
            }
            Err(e) => {
                result.errors.push(ChunkError {
                    offset,
                    error_type: ChunkErrorType::ProcessingError,
                    message: format!("Failed to process chunk: {:?}", e),
                });

                // Try to process smaller sub-chunks on error
                if data.len() > 1024 {
                    result.errors.push(ChunkError {
                        offset,
                        error_type: ChunkErrorType::FallbackProcessing,
                        message: "Attempting sub-chunk processing".to_string(),
                    });

                    self.process_sub_chunks(data, offset, processor, &mut result)?;
                }
            }
        }

        Ok(result)
    }

    /// Process smaller sub-chunks when main processing fails
    fn process_sub_chunks<F>(
        &self,
        data: &[u8],
        base_offset: u64,
        processor: &mut F,
        result: &mut SingleChunkResult,
    ) -> AstResult<()>
    where
        F: FnMut(&[u8], u64) -> AstResult<Vec<AstNode>>,
    {
        let sub_chunk_size = 1024; // 1KB sub-chunks
        let mut offset = 0;

        while offset < data.len() {
            let end = (offset + sub_chunk_size).min(data.len());
            let sub_chunk = &data[offset..end];
            let sub_offset = base_offset + offset as u64;

            match processor(sub_chunk, sub_offset) {
                Ok(mut nodes) => {
                    result.nodes.append(&mut nodes);
                }
                Err(e) => {
                    result.warnings.push(ChunkWarning {
                        offset: sub_offset,
                        warning_type: ChunkWarningType::SubChunkError,
                        message: format!("Sub-chunk processing failed: {:?}", e),
                    });
                }
            }

            offset += sub_chunk_size;
        }

        Ok(())
    }

    /// Get chunk by offset
    pub fn get_chunk(&self, offset: u64) -> Option<&Vec<u8>> {
        self.chunk_cache.get(&offset)
    }

    /// Get chunk information
    pub fn get_chunk_info(&self, offset: u64) -> Option<&ProcessedChunkInfo> {
        self.processed_chunks.get(&offset)
    }

    /// Get all processed chunk offsets
    pub fn get_processed_offsets(&self) -> Vec<u64> {
        let mut offsets: Vec<_> = self.processed_chunks.keys().cloned().collect();
        offsets.sort();
        offsets
    }

    /// Calculate a simple checksum for data integrity
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        data.iter().fold(0u32, |acc, &byte| {
            acc.wrapping_mul(31).wrapping_add(byte as u32)
        })
    }

    /// Verify chunk integrity
    pub fn verify_chunk(&self, offset: u64) -> AstResult<bool> {
        if let Some(chunk) = self.chunk_cache.get(&offset) {
            if let Some(info) = self.processed_chunks.get(&offset) {
                let current_checksum = self.calculate_checksum(chunk);
                return Ok(current_checksum == info.checksum);
            }
        }
        Ok(false)
    }

    /// Clear old chunks from cache to manage memory
    pub fn cleanup_cache(&mut self, keep_recent: usize) {
        if self.chunk_cache.len() > keep_recent {
            let mut offsets: Vec<_> = self.chunk_cache.keys().cloned().collect();
            offsets.sort();

            // Remove oldest chunks
            let to_remove = offsets.len() - keep_recent;
            for &offset in offsets.iter().take(to_remove) {
                self.chunk_cache.remove(&offset);
            }
        }
    }
}

/// Result of processing a single chunk
#[derive(Debug, Clone)]
pub struct SingleChunkResult {
    pub nodes: Vec<AstNode>,
    pub errors: Vec<ChunkError>,
    pub warnings: Vec<ChunkWarning>,
}

/// Result of processing multiple chunks
#[derive(Debug, Clone)]
pub struct ChunkProcessingResult {
    pub total_chunks: usize,
    pub total_nodes: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub total_bytes: u64,
    pub total_processing_time_ms: u64,
    pub chunk_results: Vec<(SingleChunkResult, ProcessedChunkInfo)>,
}

impl ChunkProcessingResult {
    pub fn new() -> Self {
        Self {
            total_chunks: 0,
            total_nodes: 0,
            total_errors: 0,
            total_warnings: 0,
            total_bytes: 0,
            total_processing_time_ms: 0,
            chunk_results: Vec::new(),
        }
    }

    pub fn add_chunk_result(&mut self, result: SingleChunkResult, info: ProcessedChunkInfo) {
        self.total_chunks += 1;
        self.total_nodes += result.nodes.len();
        self.total_errors += result.errors.len();
        self.total_warnings += result.warnings.len();
        self.total_bytes += info.size as u64;
        self.total_processing_time_ms += info.processing_time_ms;

        self.chunk_results.push((result, info));
    }

    pub fn finalize(&mut self) {
        // Calculate final statistics or perform cleanup
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_chunks == 0 {
            return 1.0;
        }

        let successful_chunks = self.total_chunks
            - self
                .chunk_results
                .iter()
                .filter(|(result, _)| !result.errors.is_empty())
                .count();

        successful_chunks as f64 / self.total_chunks as f64
    }

    pub fn processing_rate_mb_per_sec(&self) -> f64 {
        if self.total_processing_time_ms == 0 {
            return 0.0;
        }

        let mb_processed = self.total_bytes as f64 / (1024.0 * 1024.0);
        let seconds = self.total_processing_time_ms as f64 / 1000.0;

        mb_processed / seconds
    }
}

impl Default for ChunkProcessingResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Error that occurred during chunk processing
#[derive(Debug, Clone)]
pub struct ChunkError {
    pub offset: u64,
    pub error_type: ChunkErrorType,
    pub message: String,
}

/// Type of chunk processing error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkErrorType {
    ProcessingError,
    ParseError,
    MemoryError,
    IOError,
    FallbackProcessing,
}

/// Warning generated during chunk processing
#[derive(Debug, Clone)]
pub struct ChunkWarning {
    pub offset: u64,
    pub warning_type: ChunkWarningType,
    pub message: String,
}

/// Type of chunk processing warning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkWarningType {
    PartialProcessing,
    DataIncomplete,
    SubChunkError,
    PerformanceWarning,
}

/// Helper function to create a simple chunk processor
pub fn create_simple_chunk_processor() -> ChunkProcessor {
    ChunkProcessor::new(ChunkConfig::default())
}

/// Process a file in chunks with a simple node extraction function
pub fn process_file_in_chunks<P: AsRef<std::path::Path>>(
    path: P,
    chunk_size: usize,
) -> AstResult<ChunkProcessingResult> {
    let file = std::fs::File::open(path)?;
    let config = ChunkConfig {
        chunk_size,
        ..ChunkConfig::default()
    };
    let mut processor = ChunkProcessor::new(config);

    processor.process_chunks(file, |data, offset| {
        // Simple processing: try to find PDF objects in the chunk
        let parser = crate::parser::PdfParser::new();
        match parser.parse_objects(data) {
            Ok(pdf_values) => {
                let mut nodes = Vec::new();
                for (i, pdf_value) in pdf_values.into_iter().enumerate() {
                    let node_id = NodeId::new(offset as usize + i);
                    let node = AstNode::new(node_id, NodeType::Unknown, pdf_value);
                    nodes.push(node);
                }
                Ok(nodes)
            }
            Err(_) => Ok(Vec::new()), // Return empty vec on parse errors
        }
    })
}
