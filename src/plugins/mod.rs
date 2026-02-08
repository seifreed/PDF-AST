use crate::ast::{AstNode, NodeId, NodeType, PdfAstGraph, PdfDocument};
use crate::validation::{SchemaConstraint, ValidationReport};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;

pub mod api;
pub mod loader;
pub mod registry;

pub use api::*;
pub use loader::*;
pub use registry::*;

/// Plugin execution result
#[derive(Debug, Clone)]
pub enum PluginResult {
    Success,
    Modified(Vec<NodeId>),
    Error(String),
    Warning(String),
}

impl PluginResult {
    pub fn is_success(&self) -> bool {
        matches!(self, PluginResult::Success | PluginResult::Modified(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, PluginResult::Error(_))
    }

    pub fn get_error(&self) -> Option<&String> {
        match self {
            PluginResult::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub supported_node_types: Vec<String>,
    pub dependencies: Vec<String>,
    pub api_version: String,
}

impl PluginMetadata {
    pub fn new(name: &str, version: &str, description: &str, author: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            author: author.to_string(),
            license: None,
            homepage: None,
            repository: None,
            tags: Vec::new(),
            supported_node_types: Vec::new(),
            dependencies: Vec::new(),
            api_version: "0.1.0".to_string(),
        }
    }

    pub fn with_license(mut self, license: &str) -> Self {
        self.license = Some(license.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_supported_types(mut self, types: Vec<NodeType>) -> Self {
        self.supported_node_types = types.into_iter().map(|t| format!("{:?}", t)).collect();
        self
    }
}

/// Base trait for all plugins
pub trait AstPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    fn initialize(&mut self, context: &mut PluginContext) -> PluginResult {
        let _ = context;
        PluginResult::Success
    }

    /// Process a single node
    fn process_node(&self, node: &mut AstNode, context: &mut PluginContext) -> PluginResult {
        let _ = (node, context);
        PluginResult::Success
    }

    /// Process the entire document
    fn process_document(
        &self,
        document: &mut PdfDocument,
        context: &mut PluginContext,
    ) -> PluginResult {
        let _ = (document, context);
        PluginResult::Success
    }

    /// Finalize processing
    fn finalize(&self, context: &mut PluginContext) -> PluginResult {
        let _ = context;
        PluginResult::Success
    }

    /// Get plugin configuration schema
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Set plugin configuration
    fn set_config(&mut self, config: serde_json::Value) -> PluginResult {
        let _ = config;
        PluginResult::Success
    }

    /// Check if plugin can process specific node type
    fn can_process_node_type(&self, node_type: &NodeType) -> bool {
        let node_type_str = format!("{:?}", node_type);
        self.metadata().supported_node_types.is_empty()
            || self
                .metadata()
                .supported_node_types
                .contains(&node_type_str)
    }

    /// Get plugin capabilities
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    /// Clone the plugin (for plugin instances)
    fn clone_plugin(&self) -> Box<dyn AstPlugin>;
}

/// Plugin capabilities
#[derive(Debug, Clone, Default)]
pub struct PluginCapabilities {
    pub can_modify_nodes: bool,
    pub can_add_nodes: bool,
    pub can_remove_nodes: bool,
    pub can_validate: bool,
    pub can_transform: bool,
    pub requires_document_context: bool,
    pub thread_safe: bool,
}

/// Plugin execution context
pub struct PluginContext {
    pub document: Option<*const PdfDocument>,
    pub current_node: Option<NodeId>,
    pub graph: Option<*mut PdfAstGraph>,
    pub config: HashMap<String, serde_json::Value>,
    pub shared_data: HashMap<String, Box<dyn Any + Send + Sync>>,
    pub statistics: PluginStatistics,
}

impl PluginContext {
    pub fn new() -> Self {
        Self {
            document: None,
            current_node: None,
            graph: None,
            config: HashMap::new(),
            shared_data: HashMap::new(),
            statistics: PluginStatistics::default(),
        }
    }

    pub fn with_document(mut self, document: &PdfDocument) -> Self {
        self.document = Some(document as *const PdfDocument);
        self
    }

    pub fn with_graph(mut self, graph: &mut PdfAstGraph) -> Self {
        self.graph = Some(graph as *mut PdfAstGraph);
        self
    }

    pub fn set_config(&mut self, key: String, value: serde_json::Value) {
        self.config.insert(key, value);
    }

    pub fn get_config(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.get(key)
    }

    pub fn set_shared_data<T: Any + Send + Sync>(&mut self, key: String, data: T) {
        self.shared_data.insert(key, Box::new(data));
    }

    pub fn get_shared_data<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.shared_data.get(key)?.downcast_ref::<T>()
    }

    pub fn get_document(&self) -> Option<&PdfDocument> {
        self.document.map(|ptr| unsafe { &*ptr })
    }

    pub fn get_graph_mut(&mut self) -> Option<&mut PdfAstGraph> {
        self.graph.map(|ptr| unsafe { &mut *ptr })
    }

    pub fn set_data(&mut self, key: &str, value: String) {
        self.set_config(key.to_string(), serde_json::Value::String(value));
    }

    pub fn add_warning(&mut self, message: &str) {
        self.statistics.warnings.push(message.to_string());
    }

    pub fn add_error(&mut self, message: &str) {
        self.statistics.errors.push(message.to_string());
    }

    pub fn add_info(&mut self, message: String) {
        self.statistics.info_messages.push(message);
    }
}

unsafe impl Send for PluginContext {}
unsafe impl Sync for PluginContext {}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin execution statistics
#[derive(Debug, Clone, Default)]
pub struct PluginStatistics {
    pub nodes_processed: usize,
    pub nodes_modified: usize,
    pub nodes_added: usize,
    pub nodes_removed: usize,
    pub execution_time_ms: u64,
    pub memory_used_bytes: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info_messages: Vec<String>,
}

/// Specialized plugin types
/// Node transformer plugin
pub trait NodeTransformerPlugin: AstPlugin {
    fn transform_node(&self, node: &mut AstNode, context: &mut PluginContext) -> PluginResult;
}

/// Validator plugin
pub trait ValidatorPlugin: AstPlugin {
    fn validate_document(&self, document: &PdfDocument) -> ValidationReport;
    fn get_constraints(&self) -> Vec<Box<dyn SchemaConstraint>>;
}

/// Filter plugin for content processing
pub trait FilterPlugin: AstPlugin {
    fn filter_content(
        &self,
        content: &[u8],
        filter_params: &HashMap<String, String>,
    ) -> Result<Vec<u8>, String>;
    fn get_supported_filters(&self) -> Vec<String>;
}

/// Analyzer plugin for extracting information
pub trait AnalyzerPlugin: AstPlugin {
    fn analyze_document(&self, document: &PdfDocument) -> AnalysisResult;
    fn get_analysis_types(&self) -> Vec<String>;
}

/// Analysis result from analyzer plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub analyzer_name: String,
    pub analysis_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

impl AnalysisResult {
    pub fn new(analyzer_name: &str, analysis_type: &str, data: serde_json::Value) -> Self {
        Self {
            analyzer_name: analyzer_name.to_string(),
            analysis_type: analysis_type.to_string(),
            timestamp: chrono::Utc::now(),
            data,
            metadata: HashMap::new(),
        }
    }
}

/// Plugin execution pipeline
pub struct PluginPipeline {
    plugins: Vec<Box<dyn AstPlugin>>,
    context: PluginContext,
    parallel_execution: bool,
}

impl PluginPipeline {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            context: PluginContext::new(),
            parallel_execution: false,
        }
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn AstPlugin>) {
        self.plugins.push(plugin);
    }

    pub fn with_parallel_execution(mut self, parallel: bool) -> Self {
        self.parallel_execution = parallel;
        self
    }

    pub fn execute(&mut self, document: &mut PdfDocument) -> Vec<PluginResult> {
        let mut results = Vec::new();

        // Initialize context
        self.context = PluginContext::new()
            .with_document(document)
            .with_graph(&mut document.ast);

        // Initialize all plugins
        for plugin in &mut self.plugins {
            let result = plugin.initialize(&mut self.context);
            results.push(result);
        }

        // Process document with each plugin
        if self.parallel_execution {
            // Parallel execution would require Arc<Mutex<>> or similar
            // For now, execute sequentially
            self.execute_sequential(document, &mut results);
        } else {
            self.execute_sequential(document, &mut results);
        }

        // Finalize all plugins
        for plugin in &mut self.plugins {
            let result = plugin.finalize(&mut self.context);
            results.push(result);
        }

        results
    }

    fn execute_sequential(&mut self, document: &mut PdfDocument, results: &mut Vec<PluginResult>) {
        for plugin in &self.plugins {
            // Process entire document
            let doc_result = plugin.process_document(document, &mut self.context);
            results.push(doc_result);

            // Process individual nodes if plugin supports it
            if plugin.capabilities().can_modify_nodes {
                let node_ids: Vec<NodeId> =
                    document.ast.get_all_nodes().iter().map(|n| n.id).collect();

                for node_id in node_ids {
                    if let Some(mut node) = document.ast.get_node(node_id).cloned() {
                        if plugin.can_process_node_type(&node.node_type) {
                            let node_result = plugin.process_node(&mut node, &mut self.context);
                            results.push(node_result);

                            // Update node in graph if modified
                            if let Some(graph_node) = document.ast.get_node_mut(node_id) {
                                *graph_node = node;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_statistics(&self) -> &PluginStatistics {
        &self.context.statistics
    }
}

impl Default for PluginPipeline {
    fn default() -> Self {
        Self::new()
    }
}
