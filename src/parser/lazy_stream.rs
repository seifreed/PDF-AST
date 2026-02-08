use crate::types::{ObjectId, PdfDictionary, PdfStream};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

/// Lazy stream that loads data on-demand
#[derive(Debug, Clone)]
pub struct LazyStream {
    dict: PdfDictionary,
    loader: StreamLoader,
    cache: Arc<Mutex<Option<Vec<u8>>>>,
}

#[derive(Debug, Clone)]
pub enum StreamLoader {
    /// Stream data stored inline
    Inline(Vec<u8>),

    /// Stream data to be loaded from file
    File {
        offset: u64,
        length: usize,
        file_handle: Arc<Mutex<Box<dyn StreamSource>>>,
    },

    /// Stream data from object stream
    ObjectStream {
        stream_obj: ObjectId,
        index: u32,
        parent_loader: Box<StreamLoader>,
    },
}

pub trait StreamSource: Send + Sync {
    fn read_at(&mut self, offset: u64, length: usize) -> std::io::Result<Vec<u8>>;
    fn clone_source(&self) -> Box<dyn StreamSource>;
}

impl std::fmt::Debug for dyn StreamSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StreamSource")
    }
}

impl LazyStream {
    pub fn new_inline(dict: PdfDictionary, data: Vec<u8>) -> Self {
        LazyStream {
            dict,
            loader: StreamLoader::Inline(data),
            cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_file(
        dict: PdfDictionary,
        offset: u64,
        length: usize,
        source: Box<dyn StreamSource>,
    ) -> Self {
        LazyStream {
            dict,
            loader: StreamLoader::File {
                offset,
                length,
                file_handle: Arc::new(Mutex::new(source)),
            },
            cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_object_stream(
        dict: PdfDictionary,
        stream_obj: ObjectId,
        index: u32,
        parent: StreamLoader,
    ) -> Self {
        LazyStream {
            dict,
            loader: StreamLoader::ObjectStream {
                stream_obj,
                index,
                parent_loader: Box::new(parent),
            },
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Load stream data on-demand
    pub fn load(&self) -> Result<Vec<u8>, String> {
        // Check cache first
        if let Ok(cache) = self.cache.lock() {
            if let Some(ref data) = *cache {
                return Ok(data.clone());
            }
        }

        // Load data based on loader type
        let data = match &self.loader {
            StreamLoader::Inline(data) => data.clone(),

            StreamLoader::File {
                offset,
                length,
                file_handle,
            } => {
                let mut handle = file_handle
                    .lock()
                    .map_err(|e| format!("Failed to lock file handle: {}", e))?;

                handle
                    .read_at(*offset, *length)
                    .map_err(|e| format!("Failed to read stream data: {}", e))?
            }

            StreamLoader::ObjectStream {
                stream_obj: _,
                index,
                parent_loader,
            } => {
                // Load parent stream first
                let parent_data = self.load_parent_stream(parent_loader)?;

                // Parse object stream to extract specific object
                self.extract_from_object_stream(&parent_data, *index)?
            }
        };

        // Cache the loaded data
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(data.clone());
        }

        Ok(data)
    }

    fn load_parent_stream(&self, parent: &StreamLoader) -> Result<Vec<u8>, String> {
        match parent {
            StreamLoader::Inline(data) => Ok(data.clone()),

            StreamLoader::File {
                offset,
                length,
                file_handle,
            } => {
                let mut handle = file_handle
                    .lock()
                    .map_err(|e| format!("Failed to lock parent file handle: {}", e))?;

                handle
                    .read_at(*offset, *length)
                    .map_err(|e| format!("Failed to read parent stream: {}", e))
            }

            StreamLoader::ObjectStream { .. } => {
                Err("Nested object streams not supported".to_string())
            }
        }
    }

    fn extract_from_object_stream(&self, data: &[u8], index: u32) -> Result<Vec<u8>, String> {
        // Parse object stream format
        // First parse the offset table
        let n = self
            .dict
            .get("N")
            .and_then(|v| v.as_integer())
            .ok_or("Missing N in object stream")?;

        let first = self
            .dict
            .get("First")
            .and_then(|v| v.as_integer())
            .ok_or("Missing First in object stream")?;

        if index >= n as u32 {
            return Err(format!(
                "Index {} out of range for object stream with {} objects",
                index, n
            ));
        }

        // Parse offset table (simplified - would need proper parsing)
        let _offset_entry_size = 16; // Approximate size of "objnum offset" entry
        let offset_table_end = first as usize;

        if offset_table_end > data.len() {
            return Err("Invalid First offset in object stream".to_string());
        }

        // Find object offset in stream
        let offset_table = &data[..offset_table_end];
        let entries: Vec<&str> = std::str::from_utf8(offset_table)
            .map_err(|e| format!("Invalid offset table: {}", e))?
            .split_whitespace()
            .collect();

        if entries.len() < (index * 2 + 2) as usize {
            return Err("Insufficient entries in offset table".to_string());
        }

        let obj_offset = entries[(index * 2 + 1) as usize]
            .parse::<usize>()
            .map_err(|e| format!("Invalid offset: {}", e))?;

        let absolute_offset = first as usize + obj_offset;

        // Find next object offset to determine length
        let next_offset = if index + 1 < n as u32 {
            let next_obj_offset = entries[((index + 1) * 2 + 1) as usize]
                .parse::<usize>()
                .map_err(|e| format!("Invalid next offset: {}", e))?;
            first as usize + next_obj_offset
        } else {
            data.len()
        };

        if absolute_offset >= data.len() || next_offset > data.len() {
            return Err("Object offset out of bounds".to_string());
        }

        Ok(data[absolute_offset..next_offset].to_vec())
    }

    /// Get dictionary without loading stream data
    pub fn get_dict(&self) -> &PdfDictionary {
        &self.dict
    }

    /// Check if stream data is currently cached
    pub fn is_cached(&self) -> bool {
        self.cache
            .lock()
            .map(|cache| cache.is_some())
            .unwrap_or(false)
    }

    /// Clear cached data to free memory
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = None;
        }
    }

    /// Get estimated memory usage
    pub fn memory_usage(&self) -> usize {
        let dict_size = std::mem::size_of_val(&self.dict) + self.dict.len() * 50; // Rough estimate

        let cached_size = self
            .cache
            .lock()
            .ok()
            .and_then(|cache| cache.as_ref().map(|data| data.len()))
            .unwrap_or(0);

        dict_size + cached_size
    }

    /// Convert to regular PdfStream by loading data
    pub fn to_stream(&self) -> Result<PdfStream, String> {
        let data = self.load()?;
        Ok(PdfStream {
            dict: self.dict.clone(),
            data: crate::types::stream::StreamData::Decoded(data),
        })
    }
}

/// File-based stream source implementation
pub struct FileStreamSource<R: Read + Seek> {
    reader: Arc<Mutex<R>>,
}

impl<R: Read + Seek + Send + 'static> FileStreamSource<R> {
    pub fn new(reader: R) -> Self {
        FileStreamSource {
            reader: Arc::new(Mutex::new(reader)),
        }
    }
}

impl<R: Read + Seek + Send + 'static> StreamSource for FileStreamSource<R> {
    fn read_at(&mut self, offset: u64, length: usize) -> std::io::Result<Vec<u8>> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|_| std::io::Error::other("Failed to acquire lock"))?;
        reader.seek(SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    fn clone_source(&self) -> Box<dyn StreamSource> {
        Box::new(FileStreamSource {
            reader: self.reader.clone(),
        })
    }
}

/// Memory-based stream source for testing
pub struct MemoryStreamSource {
    data: Vec<u8>,
}

impl MemoryStreamSource {
    pub fn new(data: Vec<u8>) -> Self {
        MemoryStreamSource { data }
    }
}

impl StreamSource for MemoryStreamSource {
    fn read_at(&mut self, offset: u64, length: usize) -> std::io::Result<Vec<u8>> {
        let offset = offset as usize;
        if offset + length > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Read beyond end of stream",
            ));
        }

        Ok(self.data[offset..offset + length].to_vec())
    }

    fn clone_source(&self) -> Box<dyn StreamSource> {
        Box::new(MemoryStreamSource {
            data: self.data.clone(),
        })
    }
}

/// Stream cache manager for memory management
pub struct StreamCacheManager {
    max_memory: usize,
    current_usage: Arc<Mutex<usize>>,
    streams: Arc<Mutex<Vec<Arc<LazyStream>>>>,
}

impl StreamCacheManager {
    pub fn new(max_memory: usize) -> Self {
        StreamCacheManager {
            max_memory,
            current_usage: Arc::new(Mutex::new(0)),
            streams: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn register_stream(&self, stream: Arc<LazyStream>) {
        if let Ok(mut streams) = self.streams.lock() {
            streams.push(stream);
        }
    }

    pub fn update_usage(&self, delta: isize) {
        if let Ok(mut usage) = self.current_usage.lock() {
            if delta > 0 {
                *usage = usage.saturating_add(delta as usize);
            } else {
                *usage = usage.saturating_sub((-delta) as usize);
            }

            // Trigger cleanup if over limit
            if *usage > self.max_memory {
                self.cleanup_caches(*usage - self.max_memory);
            }
        }
    }

    fn cleanup_caches(&self, bytes_needed: usize) {
        if let Ok(streams) = self.streams.lock() {
            let mut freed = 0;

            for stream in streams.iter() {
                if freed >= bytes_needed {
                    break;
                }

                if stream.is_cached() {
                    let usage = stream.memory_usage();
                    stream.clear_cache();
                    freed += usage;
                }
            }
        }
    }

    pub fn clear_all_caches(&self) {
        if let Ok(streams) = self.streams.lock() {
            for stream in streams.iter() {
                stream.clear_cache();
            }
        }

        if let Ok(mut usage) = self.current_usage.lock() {
            *usage = 0;
        }
    }

    pub fn get_current_usage(&self) -> usize {
        self.current_usage.lock().map(|usage| *usage).unwrap_or(0)
    }
}
