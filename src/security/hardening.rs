use crate::performance::limits::{PerformanceGuard, PerformanceLimits};
use crate::types::{ObjectId, PdfArray, PdfDictionary, PdfName, PdfString, PdfValue};
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct SecurityLimits {
    /// Maximum string length to prevent memory exhaustion
    pub max_string_length: usize,

    /// Maximum array size
    pub max_array_size: usize,

    /// Maximum dictionary size
    pub max_dictionary_size: usize,

    /// Maximum stream size in bytes
    pub max_stream_size: usize,

    /// Maximum nesting depth for objects
    pub max_nesting_depth: usize,

    /// Maximum number of references to prevent infinite loops
    pub max_reference_count: usize,

    /// Blacklisted object types that are considered dangerous
    pub forbidden_types: HashSet<String>,

    /// Blacklisted dictionary keys
    pub forbidden_keys: HashSet<String>,

    /// Patterns that should not appear in string values
    pub forbidden_patterns: Vec<Regex>,

    /// Enable JavaScript validation
    pub validate_javascript: bool,

    /// Enable form field validation
    pub validate_forms: bool,

    /// Enable annotation validation
    pub validate_annotations: bool,

    /// Maximum number of pages
    pub max_pages: usize,

    /// Maximum file size in bytes
    pub max_file_size: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        let mut forbidden_types = HashSet::new();
        forbidden_types.insert("Launch".to_string());
        forbidden_types.insert("ImportData".to_string());
        forbidden_types.insert("JavaScript".to_string());
        forbidden_types.insert("ResetForm".to_string());
        forbidden_types.insert("SubmitForm".to_string());

        let mut forbidden_keys = HashSet::new();
        forbidden_keys.insert("JS".to_string());
        forbidden_keys.insert("JavaScript".to_string());
        forbidden_keys.insert("Launch".to_string());
        forbidden_keys.insert("URI".to_string());

        let mut forbidden_patterns = Vec::new();
        // JavaScript patterns
        if let Ok(js_pattern) = Regex::new(r"(?i)(javascript|eval|function|var|let|const)") {
            forbidden_patterns.push(js_pattern);
        }
        // File system patterns
        if let Ok(fs_pattern) = Regex::new(r"(?i)(\.\.[\\/]|file://|[a-z]:\\)") {
            forbidden_patterns.push(fs_pattern);
        }
        // Network patterns
        if let Ok(net_pattern) = Regex::new(r"(?i)(https?://|ftp://|ldap://)") {
            forbidden_patterns.push(net_pattern);
        }

        Self {
            max_string_length: 1_000_000,
            max_array_size: 100_000,
            max_dictionary_size: 10_000,
            max_stream_size: 50_000_000, // 50MB
            max_nesting_depth: 50,
            max_reference_count: 1_000_000,
            forbidden_types,
            forbidden_keys,
            forbidden_patterns,
            validate_javascript: true,
            validate_forms: true,
            validate_annotations: true,
            max_pages: 10_000,
            max_file_size: 100_000_000, // 100MB
        }
    }
}

impl SecurityLimits {
    pub fn permissive() -> Self {
        Self {
            max_string_length: 10_000_000,
            max_array_size: 1_000_000,
            max_dictionary_size: 100_000,
            max_stream_size: 500_000_000, // 500MB
            max_nesting_depth: 200,
            max_reference_count: 10_000_000,
            validate_javascript: false,
            validate_forms: false,
            validate_annotations: false,
            max_pages: 100_000,
            max_file_size: 1_000_000_000, // 1GB
            ..Default::default()
        }
    }

    pub fn strict() -> Self {
        let mut limits = Self::default();

        // Add more forbidden types
        limits.forbidden_types.insert("GoTo".to_string());
        limits.forbidden_types.insert("GoToR".to_string());
        limits.forbidden_types.insert("Movie".to_string());
        limits.forbidden_types.insert("Sound".to_string());
        limits.forbidden_types.insert("Rendition".to_string());

        // Stricter limits
        limits.max_string_length = 100_000;
        limits.max_array_size = 10_000;
        limits.max_dictionary_size = 1_000;
        limits.max_stream_size = 10_000_000; // 10MB
        limits.max_nesting_depth = 20;
        limits.max_pages = 1_000;
        limits.max_file_size = 10_000_000; // 10MB

        limits
    }
}

#[derive(Debug, Clone)]
pub enum SecurityViolation {
    StringTooLong(usize, usize),
    ArrayTooLarge(usize, usize),
    DictionaryTooLarge(usize, usize),
    StreamTooLarge(usize, usize),
    NestingTooDeep(usize, usize),
    TooManyReferences(usize, usize),
    ForbiddenObjectType(String),
    ForbiddenDictionaryKey(String),
    SuspiciousPattern(String, String),
    MaliciousJavaScript(String),
    DangerousForm(String),
    SuspiciousAnnotation(String),
    TooManyPages(usize, usize),
    FileTooLarge(usize, usize),
}

impl std::fmt::Display for SecurityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityViolation::StringTooLong(len, max) => {
                write!(f, "String too long: {} > {} characters", len, max)
            }
            SecurityViolation::ArrayTooLarge(size, max) => {
                write!(f, "Array too large: {} > {} elements", size, max)
            }
            SecurityViolation::DictionaryTooLarge(size, max) => {
                write!(f, "Dictionary too large: {} > {} entries", size, max)
            }
            SecurityViolation::StreamTooLarge(size, max) => {
                write!(f, "Stream too large: {} > {} bytes", size, max)
            }
            SecurityViolation::NestingTooDeep(depth, max) => {
                write!(f, "Nesting too deep: {} > {} levels", depth, max)
            }
            SecurityViolation::TooManyReferences(count, max) => {
                write!(f, "Too many references: {} > {}", count, max)
            }
            SecurityViolation::ForbiddenObjectType(obj_type) => {
                write!(f, "Forbidden object type: {}", obj_type)
            }
            SecurityViolation::ForbiddenDictionaryKey(key) => {
                write!(f, "Forbidden dictionary key: {}", key)
            }
            SecurityViolation::SuspiciousPattern(pattern, content) => write!(
                f,
                "Suspicious pattern '{}' in content: {}",
                pattern,
                &content[..content.len().min(100)]
            ),
            SecurityViolation::MaliciousJavaScript(script) => write!(
                f,
                "Malicious JavaScript detected: {}",
                &script[..script.len().min(100)]
            ),
            SecurityViolation::DangerousForm(form_desc) => {
                write!(f, "Dangerous form field: {}", form_desc)
            }
            SecurityViolation::SuspiciousAnnotation(annot_desc) => {
                write!(f, "Suspicious annotation: {}", annot_desc)
            }
            SecurityViolation::TooManyPages(count, max) => {
                write!(f, "Too many pages: {} > {}", count, max)
            }
            SecurityViolation::FileTooLarge(size, max) => {
                write!(f, "File too large: {} > {} bytes", size, max)
            }
        }
    }
}

impl std::error::Error for SecurityViolation {}

pub struct SecurityValidator {
    limits: SecurityLimits,
    performance_guard: PerformanceGuard,
    reference_counts: HashMap<ObjectId, usize>,
    current_depth: usize,
    page_count: usize,
}

impl SecurityValidator {
    pub fn new(limits: SecurityLimits, performance_limits: PerformanceLimits) -> Self {
        Self {
            limits,
            performance_guard: PerformanceGuard::new(performance_limits, "security_validation"),
            reference_counts: HashMap::new(),
            current_depth: 0,
            page_count: 0,
        }
    }

    pub fn validate_file_size(&self, size: usize) -> Result<(), SecurityViolation> {
        if size > self.limits.max_file_size {
            return Err(SecurityViolation::FileTooLarge(
                size,
                self.limits.max_file_size,
            ));
        }
        Ok(())
    }

    pub fn validate_value(&mut self, value: &PdfValue) -> Result<(), SecurityViolation> {
        self.validate_value_recursive(value, 0)
    }

    fn validate_value_recursive(
        &mut self,
        value: &PdfValue,
        depth: usize,
    ) -> Result<(), SecurityViolation> {
        // Check nesting depth
        if depth > self.limits.max_nesting_depth {
            return Err(SecurityViolation::NestingTooDeep(
                depth,
                self.limits.max_nesting_depth,
            ));
        }

        match value {
            PdfValue::String(s) => self.validate_string(s),
            PdfValue::Array(arr) => self.validate_array(arr, depth),
            PdfValue::Dictionary(dict) => self.validate_dictionary(dict, depth),
            PdfValue::Stream(stream) => self.validate_stream(stream, depth),
            PdfValue::Reference(reference) => self.validate_reference(&reference.id()),
            PdfValue::Name(name) => self.validate_name(name),
            _ => Ok(()),
        }
    }

    fn validate_string(&self, string: &PdfString) -> Result<(), SecurityViolation> {
        let content = string.to_string_lossy();

        // Check length
        if content.len() > self.limits.max_string_length {
            return Err(SecurityViolation::StringTooLong(
                content.len(),
                self.limits.max_string_length,
            ));
        }

        // Check for forbidden patterns
        for pattern in &self.limits.forbidden_patterns {
            if let Some(_matched) = pattern.find(&content) {
                return Err(SecurityViolation::SuspiciousPattern(
                    pattern.as_str().to_string(),
                    content,
                ));
            }
        }

        Ok(())
    }

    fn validate_array(&mut self, array: &PdfArray, depth: usize) -> Result<(), SecurityViolation> {
        // Check size
        if array.len() > self.limits.max_array_size {
            return Err(SecurityViolation::ArrayTooLarge(
                array.len(),
                self.limits.max_array_size,
            ));
        }

        // Validate each element
        for element in array.iter() {
            self.validate_value_recursive(element, depth + 1)?;
        }

        Ok(())
    }

    fn validate_dictionary(
        &mut self,
        dict: &PdfDictionary,
        depth: usize,
    ) -> Result<(), SecurityViolation> {
        self.validate_dictionary_size(dict)?;

        for (key, value) in dict.iter() {
            self.validate_forbidden_key(key)?;
            self.validate_forbidden_object_type(key, value)?;
            self.validate_content_security(key, value)?;
            self.validate_value_recursive(value, depth + 1)?;
        }

        self.update_page_count_if_page(dict)
    }

    fn validate_dictionary_size(&self, dict: &PdfDictionary) -> Result<(), SecurityViolation> {
        if dict.len() > self.limits.max_dictionary_size {
            return Err(SecurityViolation::DictionaryTooLarge(
                dict.len(),
                self.limits.max_dictionary_size,
            ));
        }
        Ok(())
    }

    fn validate_forbidden_key(&self, key: &PdfName) -> Result<(), SecurityViolation> {
        if self.limits.forbidden_keys.contains(&key.to_string()) {
            return Err(SecurityViolation::ForbiddenDictionaryKey(key.to_string()));
        }
        Ok(())
    }

    fn validate_forbidden_object_type(
        &self,
        key: &PdfName,
        value: &PdfValue,
    ) -> Result<(), SecurityViolation> {
        if key != "Type" && key != "S" {
            return Ok(());
        }

        if let PdfValue::Name(type_name) = value {
            let type_str = type_name.without_slash();
            if self.limits.forbidden_types.contains(type_str) {
                return Err(SecurityViolation::ForbiddenObjectType(type_str.to_string()));
            }
        }
        Ok(())
    }

    fn validate_content_security(
        &self,
        key: &PdfName,
        value: &PdfValue,
    ) -> Result<(), SecurityViolation> {
        if self.limits.validate_javascript {
            self.validate_javascript_content(key.as_str(), value)?;
        }
        if self.limits.validate_forms {
            self.validate_form_content(key.as_str(), value)?;
        }
        if self.limits.validate_annotations {
            self.validate_annotation_content(key.as_str(), value)?;
        }
        Ok(())
    }

    fn update_page_count_if_page(&mut self, dict: &PdfDictionary) -> Result<(), SecurityViolation> {
        let is_page = dict
            .get("Type")
            .and_then(|v| v.as_name())
            .map(|n| n.without_slash())
            == Some("Page");

        if !is_page {
            return Ok(());
        }

        self.page_count += 1;
        if self.page_count > self.limits.max_pages {
            return Err(SecurityViolation::TooManyPages(
                self.page_count,
                self.limits.max_pages,
            ));
        }
        Ok(())
    }

    fn validate_stream(
        &mut self,
        stream: &crate::types::PdfStream,
        depth: usize,
    ) -> Result<(), SecurityViolation> {
        // Check stream size
        if stream.data.len() > self.limits.max_stream_size {
            return Err(SecurityViolation::StreamTooLarge(
                stream.data.len(),
                self.limits.max_stream_size,
            ));
        }

        // Validate stream dictionary
        self.validate_dictionary(&stream.dict, depth)
    }

    fn validate_reference(
        &mut self,
        reference: &crate::types::ObjectId,
    ) -> Result<(), SecurityViolation> {
        // Count reference usage to detect potential DoS
        let count = self.reference_counts.entry(*reference).or_insert(0);
        *count += 1;

        if *count > self.limits.max_reference_count {
            return Err(SecurityViolation::TooManyReferences(
                *count,
                self.limits.max_reference_count,
            ));
        }

        Ok(())
    }

    fn validate_name(&self, name: &PdfName) -> Result<(), SecurityViolation> {
        let name_str = name.without_slash();

        // Check for forbidden object types
        if self.limits.forbidden_types.contains(name_str) {
            return Err(SecurityViolation::ForbiddenObjectType(name_str.to_string()));
        }

        Ok(())
    }

    fn validate_javascript_content(
        &self,
        key: &str,
        value: &PdfValue,
    ) -> Result<(), SecurityViolation> {
        if key == "JS" || key == "JavaScript" {
            if let Some(content) = value.as_string() {
                let script = content.to_string_lossy();

                // Basic JavaScript security checks
                let dangerous_patterns = [
                    r"eval\s*\(",
                    r"Function\s*\(",
                    r"document\.",
                    r"window\.",
                    r"XMLHttpRequest",
                    r"fetch\s*\(",
                    r"\.innerHTML",
                    r"\.outerHTML",
                    r"createElement",
                ];

                for pattern in &dangerous_patterns {
                    if let Ok(regex) = Regex::new(pattern) {
                        if regex.is_match(&script) {
                            return Err(SecurityViolation::MaliciousJavaScript(script));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_form_content(&self, key: &str, value: &PdfValue) -> Result<(), SecurityViolation> {
        if key == "FT" || key == "Ff" {
            // Check for dangerous form field types
            if let Some(field_type) = value.as_name() {
                let ft = field_type.without_slash();
                if ft == "Sig" && key == "FT" {
                    // Signature fields can be dangerous
                    return Err(SecurityViolation::DangerousForm(
                        "Signature field detected".to_string(),
                    ));
                }
            }
        }

        if key == "A" || key == "AA" {
            // Action dictionaries in forms can execute code
            return Err(SecurityViolation::DangerousForm(
                "Form action detected".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_annotation_content(
        &self,
        key: &str,
        value: &PdfValue,
    ) -> Result<(), SecurityViolation> {
        if key == "Subtype" && value.as_name().map(|n| n.without_slash()) == Some("Widget") {
            // Widget annotations can be interactive
            return Err(SecurityViolation::SuspiciousAnnotation(
                "Interactive widget annotation".to_string(),
            ));
        }

        if key == "A" || key == "AA" {
            // Action dictionaries in annotations
            return Err(SecurityViolation::SuspiciousAnnotation(
                "Annotation with actions".to_string(),
            ));
        }

        if key == "Movie" || key == "Sound" {
            // Multimedia annotations
            return Err(SecurityViolation::SuspiciousAnnotation(
                "Multimedia annotation".to_string(),
            ));
        }

        Ok(())
    }

    pub fn get_statistics(&self) -> SecurityStatistics {
        SecurityStatistics {
            reference_counts: self.reference_counts.clone(),
            page_count: self.page_count,
            max_depth_reached: self.current_depth,
            performance_stats: self.performance_guard.get_stats(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub reference_counts: HashMap<ObjectId, usize>,
    pub page_count: usize,
    pub max_depth_reached: usize,
    pub performance_stats: crate::performance::limits::PerformanceStats,
}

/// Sanitize PDF content by removing dangerous elements
pub struct PdfSanitizer {
    limits: SecurityLimits,
}

impl PdfSanitizer {
    pub fn new(limits: SecurityLimits) -> Self {
        Self { limits }
    }

    pub fn sanitize_value(&self, value: &mut PdfValue) -> bool {
        match value {
            PdfValue::String(s) => self.sanitize_string(s),
            PdfValue::Array(arr) => self.sanitize_array(arr),
            PdfValue::Dictionary(dict) => self.sanitize_dictionary(dict),
            PdfValue::Stream(stream) => self.sanitize_stream(stream),
            _ => true,
        }
    }

    fn sanitize_string(&self, string: &mut PdfString) -> bool {
        let mut content = string.to_string_lossy();
        let _original_len = content.len();

        // Remove content matching forbidden patterns
        for pattern in &self.limits.forbidden_patterns {
            content = pattern.replace_all(&content, "[SANITIZED]").to_string();
        }

        // Truncate if too long
        if content.len() > self.limits.max_string_length {
            content.truncate(self.limits.max_string_length);
            content.push_str("[TRUNCATED]");
        }

        if content != string.to_string_lossy() {
            *string = PdfString::new_literal(content.as_bytes());
            return false; // Content was modified
        }

        true
    }

    fn sanitize_array(&self, array: &mut PdfArray) -> bool {
        let mut all_clean = true;

        // Truncate if too large
        if array.len() > self.limits.max_array_size {
            array.truncate(self.limits.max_array_size);
            all_clean = false;
        }

        // Sanitize each element
        for element in array.iter_mut() {
            if !self.sanitize_value(element) {
                all_clean = false;
            }
        }

        all_clean
    }

    fn sanitize_dictionary(&self, dict: &mut PdfDictionary) -> bool {
        let keys_removed = self.remove_forbidden_keys(dict);
        let type_removed = self.remove_forbidden_type(dict);
        let values_modified = self.sanitize_dictionary_values(dict);

        !keys_removed && !type_removed && !values_modified
    }

    fn remove_forbidden_keys(&self, dict: &mut PdfDictionary) -> bool {
        let keys_to_remove: Vec<_> = dict
            .keys()
            .filter(|key| self.limits.forbidden_keys.contains(key.without_slash()))
            .cloned()
            .collect();

        let removed_any = !keys_to_remove.is_empty();
        for key in keys_to_remove {
            dict.remove(key.as_str());
        }
        removed_any
    }

    fn remove_forbidden_type(&self, dict: &mut PdfDictionary) -> bool {
        let should_remove = dict
            .get("Type")
            .and_then(|v| v.as_name())
            .map(|type_name| {
                self.limits
                    .forbidden_types
                    .contains(type_name.without_slash())
            })
            .unwrap_or(false);

        if should_remove {
            dict.remove("Type");
        }
        should_remove
    }

    fn sanitize_dictionary_values(&self, dict: &mut PdfDictionary) -> bool {
        let mut any_modified = false;
        let keys: Vec<_> = dict.keys().cloned().collect();

        for key in keys {
            if let Some(mut value) = dict.remove(key.as_str()) {
                if !self.sanitize_value(&mut value) {
                    any_modified = true;
                }
                dict.insert(key, value);
            }
        }
        any_modified
    }

    fn sanitize_stream(&self, stream: &mut crate::types::PdfStream) -> bool {
        let mut all_clean = true;

        // Truncate stream data if too large
        if stream.data.len() > self.limits.max_stream_size {
            stream.data.truncate(self.limits.max_stream_size);
            all_clean = false;
        }

        // Sanitize stream dictionary
        if !self.sanitize_dictionary(&mut stream.dict) {
            all_clean = false;
        }

        all_clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_security_limits() {
        let limits = SecurityLimits::strict();
        assert!(limits.forbidden_types.contains("JavaScript"));
        assert!(limits.forbidden_keys.contains("JS"));
        assert!(limits.max_string_length < SecurityLimits::default().max_string_length);
    }

    #[test]
    fn test_string_validation() {
        let limits = SecurityLimits::default();
        let perf_limits = PerformanceLimits::default();
        let validator = SecurityValidator::new(limits, perf_limits);

        // Test long string
        let long_string = PdfString::new_literal(vec![b'a'; 2_000_000]);
        assert!(validator.validate_string(&long_string).is_err());

        // Test JavaScript pattern
        let js_string = PdfString::new_literal(b"function evil() { eval('bad'); }");
        assert!(validator.validate_string(&js_string).is_err());
    }

    #[test]
    fn test_sanitizer() {
        let limits = SecurityLimits::default();
        let sanitizer = PdfSanitizer::new(limits);

        let mut dict = PdfDictionary::new();
        dict.insert(
            "Type".to_string(),
            PdfValue::Name(PdfName::new("JavaScript")),
        );
        dict.insert(
            "JS".to_string(),
            PdfValue::String(PdfString::new_literal(b"alert('xss')")),
        );

        let clean = sanitizer.sanitize_dictionary(&mut dict);
        assert!(!clean); // Should not be clean
        assert!(!dict.contains_key("Type")); // JavaScript type should be removed
        assert!(!dict.contains_key("JS")); // JS key should be removed
    }

    #[test]
    fn test_reference_counting() {
        let limits = SecurityLimits::default();
        let perf_limits = PerformanceLimits::default();
        let mut validator = SecurityValidator::new(limits, perf_limits);

        let obj_id = ObjectId {
            number: 1,
            generation: 0,
        };

        // Validate same reference multiple times
        for _ in 0..10 {
            assert!(validator.validate_reference(&obj_id).is_ok());
        }

        assert_eq!(validator.reference_counts[&obj_id], 10);
    }
}
