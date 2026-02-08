//! Project-wide constants for PDF-AST
//!
//! This module centralizes magic numbers and configuration values
//! to improve code readability and maintainability.

/// Buffer size constants
pub mod buffer_sizes {
    /// Small buffer for header reading (1KB)
    pub const SMALL: usize = 1024;
    /// Medium buffer for general operations (4KB)
    pub const MEDIUM: usize = 4096;
    /// Large buffer for stream processing (64KB)
    pub const LARGE: usize = 65536;
    /// First object buffer size for linearization check
    pub const FIRST_OBJECT: usize = 1024;
    /// Header search buffer size
    pub const HEADER_SEARCH: usize = 1024;
}

/// Parsing limits
pub mod limits {
    /// Maximum nesting depth for PDF structures
    pub const MAX_NESTING_DEPTH: usize = 256;
    /// Maximum objects to scan during recovery
    pub const MAX_OBJECT_SCAN: usize = 50_000;
    /// Default decode limit in megabytes
    pub const DEFAULT_DECODE_LIMIT_MB: usize = 10;
    /// Maximum decode ratio for decompression bombs
    pub const MAX_DECODE_RATIO: usize = 100;
}

/// Size unit constants
pub mod units {
    pub const KB: usize = 1024;
    pub const MB: usize = 1024 * KB;
    pub const GB: usize = 1024 * MB;

    /// Convert bytes to megabytes
    pub fn bytes_to_mb(bytes: usize) -> f64 {
        bytes as f64 / MB as f64
    }
}

/// PDF version defaults
pub mod pdf_version {
    /// Default major version when parsing fails in tolerant mode
    pub const DEFAULT_MAJOR: u8 = 1;
    /// Default minor version when parsing fails in tolerant mode
    pub const DEFAULT_MINOR: u8 = 7;
}

/// Minimum sizes for validation
pub mod min_sizes {
    /// Minimum size for a valid PDF header
    pub const PDF_HEADER: usize = 8;
}
