use crate::performance::{increment_bytes_processed, update_memory_peak, PerformanceConfig};
use bytes::{Bytes, BytesMut};
use std::collections::VecDeque;
use std::io::{BufReader, Read, Seek};

/// Memory-efficient streaming reader for large PDF files
pub struct StreamingPdfReader<R: Read + Seek> {
    reader: BufReader<R>,
    config: PerformanceConfig,
    buffer_pool: BufferPool,
    current_position: u64,
    file_size: u64,
}

impl<R: Read + Seek> StreamingPdfReader<R> {
    pub fn new(mut reader: R, config: PerformanceConfig) -> Result<Self, std::io::Error> {
        // Get file size
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        Ok(Self {
            reader: BufReader::with_capacity(config.stream_chunk_size, reader),
            config,
            buffer_pool: BufferPool::new(),
            current_position: 0,
            file_size,
        })
    }

    /// Read a chunk of data at the specified position
    pub fn read_chunk_at(&mut self, position: u64, size: usize) -> Result<Bytes, std::io::Error> {
        // Seek to position if necessary
        if self.current_position != position {
            self.reader.seek(std::io::SeekFrom::Start(position))?;
            self.current_position = position;
        }

        // Get buffer from pool
        let mut buffer = self.buffer_pool.get_buffer(size);
        buffer.resize(size, 0);

        // Read data
        let bytes_read = self.reader.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        self.current_position += bytes_read as u64;
        increment_bytes_processed(bytes_read as u64);

        // Convert to Bytes and return buffer to pool
        let data = Bytes::from(buffer.clone());
        self.buffer_pool.return_buffer(buffer);

        Ok(data)
    }

    /// Stream through the entire file in chunks
    pub fn stream_chunks(
        &mut self,
    ) -> impl Iterator<Item = Result<(u64, Bytes), std::io::Error>> + '_ {
        StreamingIterator::new(self)
    }

    /// Read a range efficiently, using streaming if it's large
    pub fn read_range(&mut self, start: u64, length: usize) -> Result<Bytes, std::io::Error> {
        // For small ranges, read directly
        if length <= self.config.stream_chunk_size * 2 {
            return self.read_chunk_at(start, length);
        }

        // For large ranges, use streaming approach
        let mut result = BytesMut::with_capacity(length);
        let mut current_pos = start;
        let end_pos = start + length as u64;

        while current_pos < end_pos {
            let chunk_size = std::cmp::min(
                self.config.stream_chunk_size,
                (end_pos - current_pos) as usize,
            );

            let chunk = self.read_chunk_at(current_pos, chunk_size)?;
            result.extend_from_slice(&chunk);
            current_pos += chunk.len() as u64;

            // Check memory usage
            let current_memory_mb = result.len() / (1024 * 1024);
            update_memory_peak(current_memory_mb);

            if current_memory_mb > self.config.max_memory_mb {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::OutOfMemory,
                    "Range too large for memory limit",
                ));
            }
        }

        Ok(result.freeze())
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn current_position(&self) -> u64 {
        self.current_position
    }
}

/// Iterator for streaming through file chunks
struct StreamingIterator<'a, R: Read + Seek> {
    reader: &'a mut StreamingPdfReader<R>,
    position: u64,
    chunk_size: usize,
}

impl<'a, R: Read + Seek> StreamingIterator<'a, R> {
    fn new(reader: &'a mut StreamingPdfReader<R>) -> Self {
        let chunk_size = reader.config.stream_chunk_size;
        Self {
            reader,
            position: 0,
            chunk_size,
        }
    }
}

impl<'a, R: Read + Seek> Iterator for StreamingIterator<'a, R> {
    type Item = Result<(u64, Bytes), std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.reader.file_size {
            return None;
        }

        let remaining = self.reader.file_size - self.position;
        let chunk_size = std::cmp::min(self.chunk_size as u64, remaining) as usize;

        match self.reader.read_chunk_at(self.position, chunk_size) {
            Ok(chunk) => {
                let pos = self.position;
                self.position += chunk.len() as u64;
                Some(Ok((pos, chunk)))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// Buffer pool to reuse byte vectors and reduce allocations
struct BufferPool {
    buffers: VecDeque<Vec<u8>>,
    max_pool_size: usize,
}

impl BufferPool {
    fn new() -> Self {
        Self {
            buffers: VecDeque::new(),
            max_pool_size: 10, // Keep up to 10 buffers in pool
        }
    }

    fn get_buffer(&mut self, min_size: usize) -> Vec<u8> {
        // Try to reuse a buffer from the pool
        while let Some(mut buffer) = self.buffers.pop_front() {
            if buffer.capacity() >= min_size {
                buffer.clear();
                return buffer;
            }
        }

        // Create new buffer if none available
        Vec::with_capacity(min_size.max(8192))
    }

    fn return_buffer(&mut self, buffer: Vec<u8>) {
        if self.buffers.len() < self.max_pool_size && buffer.capacity() >= 1024 {
            self.buffers.push_back(buffer);
        }
        // Otherwise let it be dropped
    }
}

/// Streaming cross-reference table parser
pub struct StreamingXRefParser<R: Read + Seek> {
    reader: StreamingPdfReader<R>,
}

impl<R: Read + Seek> StreamingXRefParser<R> {
    pub fn new(reader: StreamingPdfReader<R>) -> Self {
        Self { reader }
    }

    /// Parse cross-reference table without loading entire file into memory
    pub fn parse_xref_streaming(&mut self) -> Result<Vec<XRefEntry>, std::io::Error> {
        let mut entries = Vec::new();

        // Find xref table position by reading from end of file
        let trailer_pos = self.find_trailer_position()?;

        // Parse xref entries in chunks
        let mut current_pos = trailer_pos;
        loop {
            let chunk = self.reader.read_chunk_at(current_pos, 1024)?;
            if chunk.is_empty() {
                break;
            }

            // Parse xref entries from chunk
            let parsed_entries = self.parse_xref_chunk(&chunk)?;
            entries.extend(parsed_entries);

            current_pos += chunk.len() as u64;

            // Check if we've found the end
            if chunk.windows(9).any(|w| w == b"startxref") {
                break;
            }
        }

        Ok(entries)
    }

    fn find_trailer_position(&mut self) -> Result<u64, std::io::Error> {
        // Read last 1KB to find startxref
        let file_size = self.reader.file_size();
        let search_size = std::cmp::min(1024, file_size) as usize;
        let start_pos = file_size - search_size as u64;

        let chunk = self.reader.read_chunk_at(start_pos, search_size)?;

        // Find "startxref" in reverse
        if let Some(pos) = chunk.windows(9).rposition(|w| w == b"startxref") {
            // Parse the number after startxref
            let after_startxref = &chunk[pos + 9..];
            let xref_pos_str = std::str::from_utf8(after_startxref)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid UTF-8"))?
                .split_whitespace()
                .next()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "No xref position")
                })?;

            let xref_pos: u64 = xref_pos_str.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid xref position")
            })?;

            Ok(xref_pos)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "startxref not found",
            ))
        }
    }

    fn parse_xref_chunk(&self, chunk: &[u8]) -> Result<Vec<XRefEntry>, std::io::Error> {
        // Simplified xref parsing - would need full implementation
        let mut entries = Vec::new();

        let chunk_str = std::str::from_utf8(chunk)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid UTF-8"))?;

        for line in chunk_str.lines() {
            if let Some(entry) = self.parse_xref_line(line)? {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    fn parse_xref_line(&self, line: &str) -> Result<Option<XRefEntry>, std::io::Error> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() == 3 {
            let offset: u64 = parts[0].parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid offset")
            })?;
            let generation: u16 = parts[1].parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid generation")
            })?;
            let status = match parts[2] {
                "n" => XRefEntryType::InUse,
                "f" => XRefEntryType::Free,
                _ => return Ok(None),
            };

            Ok(Some(XRefEntry {
                offset,
                generation,
                entry_type: status,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct XRefEntry {
    pub offset: u64,
    pub generation: u16,
    pub entry_type: XRefEntryType,
}

#[derive(Debug, Clone)]
pub enum XRefEntryType {
    Free,
    InUse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_streaming_reader() {
        let data = b"Hello, World! This is a test PDF content.";
        let cursor = Cursor::new(data.as_slice());
        let config = PerformanceConfig::default();

        let mut reader = StreamingPdfReader::new(cursor, config).unwrap();

        let chunk = reader.read_chunk_at(0, 10).unwrap();
        assert_eq!(&chunk[..], b"Hello, Wor");

        let chunk = reader.read_chunk_at(7, 5).unwrap();
        assert_eq!(&chunk[..], b"World");
    }

    #[test]
    fn test_buffer_pool() {
        let mut pool = BufferPool::new();

        let buffer1 = pool.get_buffer(1024);
        assert!(buffer1.capacity() >= 1024);

        pool.return_buffer(buffer1);

        let buffer2 = pool.get_buffer(512);
        assert!(buffer2.capacity() >= 512);
    }
}
