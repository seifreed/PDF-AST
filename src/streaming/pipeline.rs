use crate::ast::{AstNode, AstResult, NodeId, NodeType};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(feature = "async")]
use tokio::sync::mpsc as async_mpsc;

/// Trait for pipeline processing stages
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait PipelineStage: Send + Sync {
    /// Get stage name
    fn name(&self) -> &str;

    /// Process a pipeline item
    async fn process(&self, item: PipelineItem) -> AstResult<PipelineItem>;

    /// Initialize the stage
    async fn initialize(&mut self) -> AstResult<()> {
        Ok(())
    }

    /// Cleanup the stage
    async fn cleanup(&mut self) -> AstResult<()> {
        Ok(())
    }

    /// Check if stage can handle the item type
    fn can_process(&self, item_type: &PipelineItemType) -> bool;

    /// Get stage configuration
    fn get_config(&self) -> StageConfig {
        StageConfig::default()
    }
}

/// Synchronous trait for pipeline processing stages (when async feature is disabled)
#[cfg(not(feature = "async"))]
pub trait PipelineStage: Send + Sync {
    fn process(&self, item: PipelineItem) -> AstResult<Vec<PipelineItem>>;
    fn name(&self) -> &str;

    // Default implementations for optional methods
    fn initialize(&mut self) -> AstResult<()> {
        Ok(())
    }

    fn cleanup(&mut self) -> AstResult<()> {
        Ok(())
    }

    fn can_process(&self, _item_type: &PipelineItemType) -> bool {
        // By default, can process all items
        true
    }
}

/// Asynchronous processing pipeline for streaming PDF analysis
#[allow(dead_code)]
pub struct ProcessingPipeline {
    stages: Vec<Box<dyn PipelineStage>>,
    config: PipelineConfig,
    statistics: Arc<Mutex<PipelineStatistics>>,
}

/// Configuration for the processing pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub buffer_size: usize,
    pub max_concurrent_tasks: usize,
    pub enable_backpressure: bool,
    pub timeout_ms: u64,
    pub retry_attempts: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            max_concurrent_tasks: 4,
            enable_backpressure: true,
            timeout_ms: 30000, // 30 seconds
            retry_attempts: 3,
        }
    }
}

/// Statistics for pipeline processing
#[derive(Debug, Clone, Default)]
pub struct PipelineStatistics {
    pub items_processed: usize,
    pub items_failed: usize,
    pub total_processing_time_ms: u64,
    pub stage_statistics: Vec<StageStatistics>,
    pub throughput_items_per_sec: f64,
    pub memory_usage_mb: f64,
}

/// Statistics for individual pipeline stage
#[derive(Debug, Clone, Default)]
pub struct StageStatistics {
    pub stage_name: String,
    pub items_processed: usize,
    pub processing_time_ms: u64,
    pub error_count: usize,
    pub queue_depth: usize,
}

/// Configuration for individual pipeline stages
#[derive(Debug, Clone)]
pub struct StageConfig {
    pub max_queue_size: usize,
    pub processing_timeout_ms: u64,
    pub enable_parallel_processing: bool,
    pub batch_size: usize,
}

impl Default for StageConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 100,
            processing_timeout_ms: 5000,
            enable_parallel_processing: false,
            batch_size: 1,
        }
    }
}

/// Item flowing through the pipeline
#[derive(Debug, Clone)]
pub struct PipelineItem {
    pub id: String,
    pub item_type: PipelineItemType,
    pub data: PipelineData,
    pub metadata: PipelineMetadata,
    pub stage_history: Vec<String>,
}

/// Type of pipeline item
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineItemType {
    RawData,
    ParsedObject,
    AstNode,
    Document,
    ValidationResult,
    IndexEntry,
}

/// Data carried by pipeline items
#[derive(Debug, Clone)]
pub enum PipelineData {
    Bytes(Vec<u8>),
    Node(AstNode),
    Document(crate::ast::PdfDocument),
    ValidationReport(crate::validation::ValidationReport),
    IndexData(IndexData),
    Custom(serde_json::Value),
}

/// Metadata for pipeline items
#[derive(Debug, Clone)]
pub struct PipelineMetadata {
    pub created_at: std::time::SystemTime,
    pub source_offset: u64,
    pub priority: u8,
    pub retry_count: usize,
    pub processing_hints: std::collections::HashMap<String, String>,
}

/// Index data for search functionality
#[derive(Debug, Clone)]
pub struct IndexData {
    pub content: String,
    pub node_id: Option<NodeId>,
    pub keywords: Vec<String>,
    pub weight: f32,
}

impl ProcessingPipeline {
    /// Create a new processing pipeline
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            stages: Vec::new(),
            config,
            statistics: Arc::new(Mutex::new(PipelineStatistics::default())),
        }
    }

    /// Add a stage to the pipeline
    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    /// Process items through the pipeline (async version)
    #[cfg(feature = "async")]
    pub async fn process_stream(
        &mut self,
        mut input: async_mpsc::Receiver<PipelineItem>,
        output: async_mpsc::Sender<PipelineItem>,
    ) -> AstResult<()> {
        // Initialize all stages
        for stage in &mut self.stages {
            stage.initialize().await?;
        }

        // Start processing items
        while let Some(item) = input.recv().await {
            let result = self.process_item(item).await;

            match result {
                Ok(processed_item) => {
                    if output.send(processed_item).await.is_err() {
                        break; // Output channel closed
                    }
                }
                Err(e) => {
                    eprintln!("Pipeline processing error: {:?}", e);
                    // Optionally send error items to a dead letter queue
                }
            }

            // Update statistics
            self.update_statistics().await;
        }

        // Cleanup all stages
        for stage in &mut self.stages {
            stage.cleanup().await?;
        }

        Ok(())
    }

    /// Process a single item through all stages (async version)
    #[cfg(feature = "async")]
    async fn process_item(&self, mut item: PipelineItem) -> AstResult<PipelineItem> {
        for stage in &self.stages {
            if stage.can_process(&item.item_type) {
                let start_time = std::time::Instant::now();

                // Process with timeout
                let result = tokio::time::timeout(
                    std::time::Duration::from_millis(self.config.timeout_ms),
                    stage.process(item.clone()),
                )
                .await;

                match result {
                    Ok(Ok(processed_item)) => {
                        item = processed_item;
                        item.stage_history.push(stage.name().to_string());

                        // Update stage statistics
                        let processing_time = start_time.elapsed().as_millis() as u64;
                        self.update_stage_statistics(stage.name(), processing_time, false)
                            .await;
                    }
                    Ok(Err(e)) => {
                        // Stage processing error
                        self.update_stage_statistics(stage.name(), 0, true).await;

                        // Retry logic
                        if item.metadata.retry_count < self.config.retry_attempts {
                            item.metadata.retry_count += 1;
                            return self.process_item(item).await;
                        } else {
                            return Err(e);
                        }
                    }
                    Err(_) => {
                        // Timeout
                        self.update_stage_statistics(stage.name(), 0, true).await;
                        return Err(crate::ast::AstError::Parse(
                            "Pipeline stage timeout".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(item)
    }

    /// Update pipeline statistics
    #[cfg(feature = "async")]
    async fn update_statistics(&self) {
        let mut stats = self.statistics.lock().unwrap();
        stats.items_processed += 1;

        // Calculate throughput
        if stats.total_processing_time_ms > 0 {
            let seconds = stats.total_processing_time_ms as f64 / 1000.0;
            stats.throughput_items_per_sec = stats.items_processed as f64 / seconds;
        }
    }

    /// Update stage-specific statistics
    #[cfg(feature = "async")]
    async fn update_stage_statistics(
        &self,
        stage_name: &str,
        processing_time: u64,
        is_error: bool,
    ) {
        let mut stats = self.statistics.lock().unwrap();

        // Find or create stage statistics
        let stage_stat = stats
            .stage_statistics
            .iter_mut()
            .find(|s| s.stage_name == stage_name);

        if let Some(stat) = stage_stat {
            stat.items_processed += 1;
            stat.processing_time_ms += processing_time;
            if is_error {
                stat.error_count += 1;
            }
        } else {
            stats.stage_statistics.push(StageStatistics {
                stage_name: stage_name.to_string(),
                items_processed: 1,
                processing_time_ms: processing_time,
                error_count: if is_error { 1 } else { 0 },
                queue_depth: 0,
            });
        }
    }

    /// Get current pipeline statistics
    pub fn get_statistics(&self) -> PipelineStatistics {
        self.statistics.lock().unwrap().clone()
    }
}

/// Built-in pipeline stages
/// Parser stage that converts raw data to PDF objects
pub struct ParserStage {
    parser: crate::parser::PdfParser,
    node_counter: AtomicUsize,
}

impl ParserStage {
    pub fn new() -> Self {
        Self {
            parser: crate::parser::PdfParser::new(),
            node_counter: AtomicUsize::new(0),
        }
    }
}

/// Configuration for indexing
#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub extract_text: bool,
    pub extract_metadata: bool,
    pub min_word_length: usize,
    pub max_index_size: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            extract_text: true,
            extract_metadata: true,
            min_word_length: 3,
            max_index_size: 1000000, // 1M entries
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl PipelineStage for ParserStage {
    fn name(&self) -> &str {
        "Parser"
    }

    async fn process(&self, mut item: PipelineItem) -> AstResult<PipelineItem> {
        if let PipelineData::Bytes(data) = item.data {
            match self.parser.parse_object(&data) {
                Ok(pdf_value) => {
                    let counter = self.node_counter.fetch_add(1, Ordering::SeqCst);
                    let node_id = NodeId::new(counter);
                    let node_type = NodeType::from_dict(
                        pdf_value
                            .as_dict()
                            .unwrap_or(&crate::types::PdfDictionary::new()),
                    );
                    let mut node = AstNode::new(node_id, node_type, pdf_value);
                    node.metadata.offset = Some(item.metadata.source_offset);
                    node.metadata.size = Some(data.len());
                    item.data = PipelineData::Node(node);
                    item.item_type = PipelineItemType::AstNode;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(item)
    }

    fn can_process(&self, item_type: &PipelineItemType) -> bool {
        matches!(item_type, PipelineItemType::RawData)
    }
}

#[cfg(not(feature = "async"))]
impl PipelineStage for ParserStage {
    fn name(&self) -> &str {
        "Parser"
    }

    fn process(&self, mut item: PipelineItem) -> AstResult<Vec<PipelineItem>> {
        if let PipelineData::Bytes(data) = item.data {
            match self.parser.parse_object(&data) {
                Ok(pdf_value) => {
                    let counter = self.node_counter.fetch_add(1, Ordering::SeqCst);
                    let node_id = NodeId::new(counter);
                    let node_type = NodeType::from_dict(
                        pdf_value
                            .as_dict()
                            .unwrap_or(&crate::types::PdfDictionary::new()),
                    );
                    let mut node = AstNode::new(node_id, node_type, pdf_value);
                    node.metadata.offset = Some(item.metadata.source_offset);
                    node.metadata.size = Some(data.len());
                    item.data = PipelineData::Node(node);
                    item.item_type = PipelineItemType::AstNode;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(vec![item])
    }

    fn can_process(&self, item_type: &PipelineItemType) -> bool {
        matches!(item_type, PipelineItemType::RawData)
    }
}

impl Default for ParserStage {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation stage that validates nodes against schemas
#[allow(dead_code)]
pub struct ValidationStage {
    schema_registry: crate::validation::SchemaRegistry,
    schema_name: String,
}

impl ValidationStage {
    pub fn new(schema_name: String) -> Self {
        Self {
            schema_registry: crate::validation::SchemaRegistry::new(),
            schema_name,
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl PipelineStage for ValidationStage {
    fn name(&self) -> &str {
        "Validation"
    }

    async fn process(&self, mut item: PipelineItem) -> AstResult<PipelineItem> {
        match &item.data {
            PipelineData::Document(document) => {
                if let Some(report) = self.schema_registry.validate(document, &self.schema_name) {
                    item.data = PipelineData::ValidationReport(report);
                    item.item_type = PipelineItemType::ValidationResult;
                }
            }
            PipelineData::Node(node) => {
                // Create a temporary document for validation
                let mut doc =
                    crate::ast::PdfDocument::new(crate::ast::PdfVersion { major: 1, minor: 7 });
                let node_id = doc
                    .ast
                    .create_node(node.node_type.clone(), node.value.clone());
                doc.ast.set_root(node_id);
                if let Some(report) = self.schema_registry.validate(&doc, &self.schema_name) {
                    item.data = PipelineData::ValidationReport(report);
                    item.item_type = PipelineItemType::ValidationResult;
                }
            }
            _ => {}
        }
        Ok(item)
    }

    fn can_process(&self, item_type: &PipelineItemType) -> bool {
        matches!(
            item_type,
            PipelineItemType::AstNode | PipelineItemType::Document
        )
    }
}

#[cfg(not(feature = "async"))]
impl PipelineStage for ValidationStage {
    fn name(&self) -> &str {
        "Validation"
    }

    fn process(&self, item: PipelineItem) -> AstResult<Vec<PipelineItem>> {
        let mut item = item;
        match &item.data {
            PipelineData::Document(document) => {
                if let Some(report) = self.schema_registry.validate(document, &self.schema_name) {
                    item.data = PipelineData::ValidationReport(report);
                    item.item_type = PipelineItemType::ValidationResult;
                }
            }
            PipelineData::Node(node) => {
                // Create a temporary document for validation
                let mut doc =
                    crate::ast::PdfDocument::new(crate::ast::PdfVersion { major: 1, minor: 7 });
                let node_id = doc
                    .ast
                    .create_node(node.node_type.clone(), node.value.clone());
                doc.ast.set_root(node_id);
                if let Some(report) = self.schema_registry.validate(&doc, &self.schema_name) {
                    item.data = PipelineData::ValidationReport(report);
                    item.item_type = PipelineItemType::ValidationResult;
                }
            }
            _ => {}
        }
        Ok(vec![item])
    }

    fn can_process(&self, item_type: &PipelineItemType) -> bool {
        matches!(
            item_type,
            PipelineItemType::AstNode | PipelineItemType::Document
        )
    }
}

/// Indexing stage that creates search indices
pub struct IndexingStage {
    index_config: IndexConfig,
}

impl IndexingStage {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            index_config: config,
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl PipelineStage for IndexingStage {
    fn name(&self) -> &str {
        "Indexing"
    }

    async fn process(&self, mut item: PipelineItem) -> AstResult<PipelineItem> {
        if let PipelineData::Node(ref node) = &item.data {
            let index_data = self.extract_index_data(node);
            item.data = PipelineData::IndexData(index_data);
            item.item_type = PipelineItemType::IndexEntry;
        }
        Ok(item)
    }

    fn can_process(&self, item_type: &PipelineItemType) -> bool {
        matches!(item_type, PipelineItemType::AstNode)
    }
}

#[cfg(not(feature = "async"))]
impl PipelineStage for IndexingStage {
    fn name(&self) -> &str {
        "Indexing"
    }

    fn process(&self, mut item: PipelineItem) -> AstResult<Vec<PipelineItem>> {
        if let PipelineData::Node(ref node) = &item.data {
            let index_data = self.extract_index_data(node);
            item.data = PipelineData::IndexData(index_data);
            item.item_type = PipelineItemType::IndexEntry;
        }
        Ok(vec![item])
    }

    fn can_process(&self, item_type: &PipelineItemType) -> bool {
        matches!(item_type, PipelineItemType::AstNode)
    }
}

impl IndexingStage {
    fn extract_index_data(&self, node: &AstNode) -> IndexData {
        let mut content = String::new();
        let mut keywords = Vec::new();

        // Extract content based on node type
        match &node.value {
            crate::types::PdfValue::String(s) => {
                content = s.to_string_lossy();
            }
            crate::types::PdfValue::Stream(stream) => {
                if let crate::types::StreamData::Raw(data) = &stream.data {
                    if let Ok(text) = std::str::from_utf8(data) {
                        content = text.to_string();
                    }
                }
            }
            _ => {}
        }

        // Extract keywords
        if self.index_config.extract_text && !content.is_empty() {
            keywords = content
                .split_whitespace()
                .filter(|word| word.len() >= self.index_config.min_word_length)
                .take(100) // Limit keywords
                .map(|s| s.to_lowercase())
                .collect();
        }

        IndexData {
            content,
            node_id: Some(node.id),
            keywords,
            weight: self.calculate_weight(node),
        }
    }

    fn calculate_weight(&self, node: &AstNode) -> f32 {
        // Calculate relevance weight based on node type
        match node.node_type {
            NodeType::Catalog => 1.0,
            NodeType::Page => 0.8,
            NodeType::ContentStream => 0.6,
            NodeType::Metadata => 0.9,
            _ => 0.5,
        }
    }
}

/// Helper function to create a standard processing pipeline
pub fn create_standard_pipeline() -> ProcessingPipeline {
    let mut pipeline = ProcessingPipeline::new(PipelineConfig::default());

    pipeline.add_stage(Box::new(ParserStage::new()));
    pipeline.add_stage(Box::new(ValidationStage::new("PDF-2.0".to_string())));
    pipeline.add_stage(Box::new(IndexingStage::new(IndexConfig::default())));

    pipeline
}

/// Create pipeline item from raw data
pub fn create_pipeline_item(id: String, data: Vec<u8>, offset: u64) -> PipelineItem {
    PipelineItem {
        id,
        item_type: PipelineItemType::RawData,
        data: PipelineData::Bytes(data),
        metadata: PipelineMetadata {
            created_at: std::time::SystemTime::now(),
            source_offset: offset,
            priority: 5, // Normal priority
            retry_count: 0,
            processing_hints: std::collections::HashMap::new(),
        },
        stage_history: Vec::new(),
    }
}
