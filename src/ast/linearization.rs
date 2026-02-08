/// Information about a linearized PDF
///
/// Linearized PDFs (also called "Fast Web View" or "Optimized" PDFs) are structured
/// to enable efficient byte-range requests over HTTP, allowing page-at-a-time downloading.
///
/// This structure belongs in the AST (domain) layer as it represents fundamental
/// document metadata, not parsing logic.
#[derive(Debug, Clone)]
pub struct LinearizationInfo {
    pub version: f64,
    pub file_length: u64,
    pub hint_stream_offset: u64,
    pub hint_stream_length: Option<u64>,
    pub object_count: u32,
    pub first_page_object_number: u32,
    pub first_page_end_offset: u64,
    pub main_xref_table_entries: u32,
}

impl LinearizationInfo {
    /// Validate the linearization information
    pub fn validate(&self) -> Result<(), String> {
        if self.version < 1.0 {
            return Err("Invalid linearization version".to_string());
        }

        if self.file_length == 0 {
            return Err("Invalid file length in linearization dict".to_string());
        }

        if self.hint_stream_offset >= self.file_length {
            return Err("Hint stream offset beyond file length".to_string());
        }

        if self.object_count == 0 {
            return Err("Invalid object count in linearization dict".to_string());
        }

        if self.first_page_end_offset >= self.file_length {
            return Err("First page end offset beyond file length".to_string());
        }

        Ok(())
    }
}
