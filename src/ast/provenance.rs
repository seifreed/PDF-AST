use crate::ast::NodeId;
use crate::types::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    /// Source information
    pub source: SourceInfo,

    /// Parsing metadata
    pub parsing: ParsingInfo,

    /// Decryption/decompression applied
    pub transformations: Vec<TransformationInfo>,

    /// OCG visibility state when parsed
    pub visibility_state: Option<VisibilityState>,

    /// Cross-reference chain
    pub xref_chain: Vec<ProvenanceXRefEntry>,

    /// Revision chain from Prev entries
    pub revision_chain: Vec<RevisionInfo>,

    /// Validation results
    pub validation: ValidationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// File offset where this object starts
    pub file_offset: u64,

    /// Size in bytes of the object
    pub object_size: u64,

    /// PDF version when this object was created
    pub pdf_version: Option<String>,

    /// Linearization hint table reference
    pub linearized_hint: Option<u64>,

    /// Object stream container if compressed
    pub container_stream: Option<ObjectId>,

    /// Index within object stream
    pub stream_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingInfo {
    /// Timestamp when parsed
    pub parse_timestamp: u64,

    /// Parser version used
    pub parser_version: String,

    /// Parse mode (strict, tolerant, etc.)
    pub parse_mode: String,

    /// Recovery operations applied
    pub recovery_operations: Vec<RecoveryOperation>,

    /// Parse warnings/issues
    pub parse_issues: Vec<ParseIssue>,

    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationInfo {
    /// Type of transformation
    pub transformation_type: TransformationType,

    /// Parameters used
    pub parameters: HashMap<String, String>,

    /// Success/failure status
    pub status: TransformationStatus,

    /// Original size before transformation
    pub original_size: Option<u64>,

    /// Final size after transformation
    pub final_size: Option<u64>,

    /// Transformation timestamp
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationType {
    /// Decryption applied
    Decryption {
        algorithm: String,
        key_length: u32,
        crypt_filter: Option<String>,
    },

    /// Decompression applied
    Decompression {
        filter: String,
        predictor: Option<u32>,
        columns: Option<u32>,
    },

    /// ASCII decoding
    AsciiDecoding { encoding: String },

    /// Image processing
    ImageProcessing {
        color_space: String,
        bits_per_component: u32,
        width: u32,
        height: u32,
    },

    /// Content stream processing
    ContentProcessing {
        operators_parsed: u32,
        graphics_state_depth: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationStatus {
    Success,
    Partial { reason: String },
    Failed { error: String },
    Skipped { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityState {
    /// OCG states active during parsing
    pub active_ocgs: Vec<String>,

    /// OCG configuration used
    pub ocg_config: Option<String>,

    /// Print/view context
    pub context: VisibilityContext,

    /// Zoom level if applicable
    pub zoom_level: Option<f64>,

    /// Page rotation applied
    pub rotation: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisibilityContext {
    View,
    Print,
    Export,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceXRefEntry {
    /// Object ID
    pub object_id: ObjectId,

    /// File offset
    pub offset: u64,

    /// Entry type (free, in-use, compressed)
    pub entry_type: XRefEntryType,

    /// Generation number
    pub generation: u16,

    /// Next free object (for free entries)
    pub next_free: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XRefEntryType {
    Free,
    InUse,
    Compressed { stream_id: u32, index: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionInfo {
    /// Revision number (0 is original)
    pub revision_number: u32,

    /// Byte offset of xref table for this revision
    pub xref_offset: u64,

    /// Trailer dictionary for this revision
    pub trailer_size: u64,

    /// Previous revision offset
    pub prev_offset: Option<u64>,

    /// Objects changed in this revision
    pub changed_objects: Vec<ObjectId>,

    /// Incremental update timestamp
    pub update_timestamp: Option<String>,

    /// Digital signature covering this revision
    pub signature_coverage: Option<SignatureCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureCoverage {
    /// Signature field name
    pub field_name: String,

    /// Byte range covered by signature
    pub byte_range: Vec<u64>,

    /// Signature validation status
    pub is_valid: bool,

    /// Signer certificate info
    pub signer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOperation {
    /// Type of recovery performed
    pub operation_type: RecoveryType,

    /// Description of what was recovered
    pub description: String,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,

    /// Alternative interpretations considered
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryType {
    XRefRepair,
    TrailerReconstruction,
    ObjectRepair,
    StreamLengthFix,
    EncodingFallback,
    StructureReconstruction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseIssue {
    /// Issue severity
    pub severity: IssueSeverity,

    /// Issue category
    pub category: IssueCategory,

    /// Description
    pub message: String,

    /// File location
    pub location: Option<FileLocation>,

    /// Suggested fix
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueCategory {
    Syntax,
    Structure,
    Security,
    Compatibility,
    Performance,
    Accessibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLocation {
    pub offset: u64,
    pub length: u64,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Time spent parsing this object (microseconds)
    pub parse_time_us: u64,

    /// Memory allocated for this object
    pub memory_allocated: u64,

    /// Number of child objects processed
    pub children_processed: u32,

    /// Recursion depth reached
    pub max_recursion_depth: u32,

    /// Cache hit ratio if applicable
    pub cache_hit_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetadata {
    /// PDF/A compliance level
    pub pdfa_compliance: Option<String>,

    /// PDF/X compliance level
    pub pdfx_compliance: Option<String>,

    /// Accessibility compliance
    pub accessibility_score: Option<f64>,

    /// Security assessment
    pub security_assessment: SecurityAssessment,

    /// Quality metrics
    pub quality_metrics: QualityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAssessment {
    /// Risk level (Low, Medium, High, Critical)
    pub risk_level: RiskLevel,

    /// Threats detected
    pub threats: Vec<ThreatInfo>,

    /// Encryption strength if applicable
    pub encryption_strength: Option<EncryptionStrength>,

    /// Digital signature validity
    pub signature_validity: Vec<SignatureValidation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    pub threat_type: String,
    pub description: String,
    pub mitigated: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStrength {
    pub algorithm: String,
    pub key_length: u32,
    pub is_strong: bool,
    pub vulnerabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureValidation {
    pub field_name: String,
    pub is_valid: bool,
    pub certificate_chain_valid: bool,
    pub timestamp_valid: Option<bool>,
    pub revocation_status: Option<RevocationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevocationStatus {
    Valid,
    Revoked,
    Unknown,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Text extraction quality (0.0 to 1.0)
    pub text_quality: Option<f64>,

    /// Font embedding completeness
    pub font_completeness: Option<f64>,

    /// Color consistency
    pub color_consistency: Option<f64>,

    /// Structure completeness for tagged PDFs
    pub structure_completeness: Option<f64>,

    /// Image quality assessment
    pub image_quality: Option<f64>,
}

/// Provenance tracker that collects metadata during parsing
pub struct ProvenanceTracker {
    /// Provenance data per node
    node_provenance: HashMap<NodeId, ProvenanceInfo>,

    /// Current parsing context
    current_context: ParsingContext,

    /// Global document metadata
    document_metadata: DocumentProvenance,
}

#[derive(Debug, Clone)]
pub struct ParsingContext {
    pub current_offset: u64,
    pub current_revision: u32,
    pub active_ocgs: Vec<String>,
    pub decryption_state: Option<DecryptionState>,
    pub performance_tracker: PerformanceTracker,
}

#[derive(Debug, Clone)]
pub struct DecryptionState {
    pub algorithm: String,
    pub key: Vec<u8>,
    pub crypt_filter: Option<String>,
    pub objects_decrypted: u32,
}

#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    pub start_time: SystemTime,
    pub objects_parsed: u32,
    pub bytes_processed: u64,
    pub memory_peak: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProvenance {
    pub file_size: u64,
    pub file_hash_sha256: String,
    pub parse_start_time: u64,
    pub parse_duration_ms: u64,
    pub parser_version: String,
    pub total_revisions: u32,
    pub linearized: bool,
    pub encrypted: bool,
    pub signed: bool,
    pub pdf_version: String,
}

impl ProvenanceTracker {
    pub fn new() -> Self {
        let start_time = SystemTime::now();

        Self {
            node_provenance: HashMap::new(),
            current_context: ParsingContext {
                current_offset: 0,
                current_revision: 0,
                active_ocgs: Vec::new(),
                decryption_state: None,
                performance_tracker: PerformanceTracker {
                    start_time,
                    objects_parsed: 0,
                    bytes_processed: 0,
                    memory_peak: 0,
                },
            },
            document_metadata: DocumentProvenance {
                file_size: 0,
                file_hash_sha256: String::new(),
                parse_start_time: start_time.duration_since(UNIX_EPOCH).unwrap().as_secs(),
                parse_duration_ms: 0,
                parser_version: env!("CARGO_PKG_VERSION").to_string(),
                total_revisions: 0,
                linearized: false,
                encrypted: false,
                signed: false,
                pdf_version: String::new(),
            },
        }
    }

    pub fn record_object_parsed(
        &mut self,
        node_id: NodeId,
        _object_id: ObjectId,
        offset: u64,
        size: u64,
    ) {
        let parse_time = self
            .current_context
            .performance_tracker
            .start_time
            .elapsed()
            .unwrap_or_default()
            .as_micros() as u64;

        let source_info = SourceInfo {
            file_offset: offset,
            object_size: size,
            pdf_version: Some(self.document_metadata.pdf_version.clone()),
            linearized_hint: None,
            container_stream: None,
            stream_index: None,
        };

        let parsing_info = ParsingInfo {
            parse_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            parser_version: self.document_metadata.parser_version.clone(),
            parse_mode: "tolerant".to_string(),
            recovery_operations: Vec::new(),
            parse_issues: Vec::new(),
            performance_metrics: PerformanceMetrics {
                parse_time_us: parse_time,
                memory_allocated: size,
                children_processed: 0,
                max_recursion_depth: 0,
                cache_hit_ratio: None,
            },
        };

        let provenance = ProvenanceInfo {
            source: source_info,
            parsing: parsing_info,
            transformations: Vec::new(),
            visibility_state: None,
            xref_chain: Vec::new(),
            revision_chain: Vec::new(),
            validation: ValidationMetadata {
                pdfa_compliance: None,
                pdfx_compliance: None,
                accessibility_score: None,
                security_assessment: SecurityAssessment {
                    risk_level: RiskLevel::Low,
                    threats: Vec::new(),
                    encryption_strength: None,
                    signature_validity: Vec::new(),
                },
                quality_metrics: QualityMetrics {
                    text_quality: None,
                    font_completeness: None,
                    color_consistency: None,
                    structure_completeness: None,
                    image_quality: None,
                },
            },
        };

        self.node_provenance.insert(node_id, provenance);
        self.current_context.performance_tracker.objects_parsed += 1;
        self.current_context.performance_tracker.bytes_processed += size;
    }

    pub fn record_transformation(&mut self, node_id: NodeId, transformation: TransformationInfo) {
        if let Some(provenance) = self.node_provenance.get_mut(&node_id) {
            provenance.transformations.push(transformation);
        }
    }

    pub fn record_decryption(&mut self, node_id: NodeId, algorithm: String, key_length: u32) {
        let transformation = TransformationInfo {
            transformation_type: TransformationType::Decryption {
                algorithm: algorithm.clone(),
                key_length,
                crypt_filter: self
                    .current_context
                    .decryption_state
                    .as_ref()
                    .and_then(|s| s.crypt_filter.clone()),
            },
            parameters: HashMap::new(),
            status: TransformationStatus::Success,
            original_size: None,
            final_size: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.record_transformation(node_id, transformation);
    }

    pub fn record_decompression(
        &mut self,
        node_id: NodeId,
        filter: String,
        original_size: u64,
        final_size: u64,
    ) {
        let transformation = TransformationInfo {
            transformation_type: TransformationType::Decompression {
                filter,
                predictor: None,
                columns: None,
            },
            parameters: HashMap::new(),
            status: TransformationStatus::Success,
            original_size: Some(original_size),
            final_size: Some(final_size),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.record_transformation(node_id, transformation);
    }

    pub fn set_visibility_context(
        &mut self,
        active_ocgs: Vec<String>,
        _context: VisibilityContext,
    ) {
        self.current_context.active_ocgs = active_ocgs;
    }

    pub fn record_parse_issue(&mut self, node_id: NodeId, issue: ParseIssue) {
        if let Some(provenance) = self.node_provenance.get_mut(&node_id) {
            provenance.parsing.parse_issues.push(issue);
        }
    }

    pub fn record_recovery_operation(&mut self, node_id: NodeId, operation: RecoveryOperation) {
        if let Some(provenance) = self.node_provenance.get_mut(&node_id) {
            provenance.parsing.recovery_operations.push(operation);
        }
    }

    pub fn get_provenance(&self, node_id: NodeId) -> Option<&ProvenanceInfo> {
        self.node_provenance.get(&node_id)
    }

    pub fn get_all_provenance(&self) -> &HashMap<NodeId, ProvenanceInfo> {
        &self.node_provenance
    }

    pub fn get_document_metadata(&self) -> &DocumentProvenance {
        &self.document_metadata
    }

    pub fn set_document_metadata(&mut self, metadata: DocumentProvenance) {
        self.document_metadata = metadata;
    }

    pub fn finalize(&mut self) {
        let duration = self
            .current_context
            .performance_tracker
            .start_time
            .elapsed()
            .unwrap_or_default();
        self.document_metadata.parse_duration_ms = duration.as_millis() as u64;
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}
