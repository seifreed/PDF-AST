use super::*;
use crate::ast::PdfDocument;
use crate::plugins::{loader::PluginLoader, registry::PluginRegistry};
use std::collections::HashMap;
use std::sync::Arc;

/// High-level plugin API for easy integration
pub struct PluginManager {
    registry: Arc<PluginRegistry>,
    loader: PluginLoader,
    execution_config: ExecutionConfig,
}

/// Plugin execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub parallel_execution: bool,
    pub max_execution_time_ms: Option<u64>,
    pub max_memory_usage_mb: Option<usize>,
    pub abort_on_error: bool,
    pub collect_statistics: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            parallel_execution: false,
            max_execution_time_ms: Some(30000), // 30 seconds
            max_memory_usage_mb: Some(512),     // 512 MB
            abort_on_error: true,
            collect_statistics: true,
        }
    }
}

/// Plugin execution summary
#[derive(Debug, Clone)]
pub struct ExecutionSummary {
    pub total_plugins: usize,
    pub successful_plugins: usize,
    pub failed_plugins: usize,
    pub total_execution_time_ms: u64,
    pub plugin_results: HashMap<String, PluginResult>,
    pub statistics: HashMap<String, PluginStatistics>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        let registry = Arc::new(PluginRegistry::new());
        let loader = PluginLoader::new(Arc::clone(&registry));

        Self {
            registry,
            loader,
            execution_config: ExecutionConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ExecutionConfig) -> Self {
        let mut manager = Self::new();
        manager.execution_config = config;
        manager
    }

    /// Register a plugin
    pub fn register_plugin(&mut self, plugin: Arc<dyn AstPlugin>) -> PluginResult {
        self.loader.load_plugin(plugin)
    }

    /// Load plugins from configuration file
    pub fn load_plugins_from_file<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<Vec<PluginResult>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;

        let plugins_config = config
            .get("plugins")
            .and_then(|v| v.as_array())
            .ok_or("Invalid plugin configuration")?;

        let load_config = crate::plugins::loader::LoadConfig::default();
        let configs: Vec<serde_json::Value> = plugins_config.to_vec();

        Ok(self.loader.load_plugins(&configs, &load_config))
    }

    /// Add search path for plugin discovery
    pub fn add_plugin_path<P: AsRef<std::path::Path>>(&mut self, path: P) {
        self.loader.add_search_path(path);
    }

    /// Discover and load plugins from search paths
    pub fn discover_and_load_plugins(&mut self) -> Vec<PluginResult> {
        let discovered = self.loader.discover_plugins();
        let mut results = Vec::new();
        let load_config = crate::plugins::loader::LoadConfig::default();

        for path in discovered {
            let result = self.loader.load_from_path(&path, &load_config);
            results.push(result);
        }

        results
    }

    /// Execute all applicable plugins on a document
    pub fn execute_plugins(&self, document: &mut PdfDocument) -> ExecutionSummary {
        let start_time = std::time::Instant::now();
        let plugin_names = self.registry.list_plugins();
        let mut summary = ExecutionSummary {
            total_plugins: plugin_names.len(),
            successful_plugins: 0,
            failed_plugins: 0,
            total_execution_time_ms: 0,
            plugin_results: HashMap::new(),
            statistics: HashMap::new(),
        };

        let mut pipeline =
            PluginPipeline::new().with_parallel_execution(self.execution_config.parallel_execution);

        // Add all plugins to pipeline
        for name in &plugin_names {
            if let Some(plugin) = self.registry.get_plugin(name) {
                pipeline.add_plugin(plugin.clone_plugin());
            }
        }

        // Execute pipeline
        let results = pipeline.execute(document);

        // Process results
        for (i, result) in results.into_iter().enumerate() {
            if let Some(plugin_name) = plugin_names.get(i) {
                match result {
                    PluginResult::Success | PluginResult::Modified(_) => {
                        summary.successful_plugins += 1;
                    }
                    _ => {
                        summary.failed_plugins += 1;
                    }
                }
                summary.plugin_results.insert(plugin_name.clone(), result);
            }
        }

        // Collect statistics
        if self.execution_config.collect_statistics {
            summary
                .statistics
                .insert("pipeline".to_string(), pipeline.get_statistics().clone());
        }

        summary.total_execution_time_ms = start_time.elapsed().as_millis() as u64;
        summary
    }

    /// Execute specific plugins by name
    pub fn execute_plugins_by_name(
        &self,
        document: &mut PdfDocument,
        plugin_names: &[String],
    ) -> ExecutionSummary {
        let start_time = std::time::Instant::now();
        let mut summary = ExecutionSummary {
            total_plugins: plugin_names.len(),
            successful_plugins: 0,
            failed_plugins: 0,
            total_execution_time_ms: 0,
            plugin_results: HashMap::new(),
            statistics: HashMap::new(),
        };

        let mut pipeline =
            PluginPipeline::new().with_parallel_execution(self.execution_config.parallel_execution);

        // Add specified plugins to pipeline
        for name in plugin_names {
            if let Some(plugin) = self.registry.get_plugin(name) {
                pipeline.add_plugin(plugin.clone_plugin());
            }
        }

        // Execute pipeline
        let results = pipeline.execute(document);

        // Process results
        for (i, result) in results.into_iter().enumerate() {
            if let Some(plugin_name) = plugin_names.get(i) {
                match result {
                    PluginResult::Success | PluginResult::Modified(_) => {
                        summary.successful_plugins += 1;
                    }
                    _ => {
                        summary.failed_plugins += 1;
                    }
                }
                summary.plugin_results.insert(plugin_name.clone(), result);
            }
        }

        summary.total_execution_time_ms = start_time.elapsed().as_millis() as u64;
        summary
    }

    /// Execute plugins for specific node type
    pub fn execute_plugins_for_type(
        &self,
        document: &mut PdfDocument,
        node_type: &crate::ast::NodeType,
    ) -> ExecutionSummary {
        let start_time = std::time::Instant::now();
        let plugins = self.registry.get_plugins_for_type(node_type);
        let mut summary = ExecutionSummary {
            total_plugins: plugins.len(),
            successful_plugins: 0,
            failed_plugins: 0,
            total_execution_time_ms: 0,
            plugin_results: HashMap::new(),
            statistics: HashMap::new(),
        };

        let mut pipeline =
            PluginPipeline::new().with_parallel_execution(self.execution_config.parallel_execution);

        // Add type-specific plugins to pipeline
        for plugin in &plugins {
            pipeline.add_plugin(plugin.clone_plugin());
        }

        // Execute pipeline
        let results = pipeline.execute(document);

        // Process results
        for (i, result) in results.into_iter().enumerate() {
            if let Some(plugin) = plugins.get(i) {
                let plugin_name = plugin.metadata().name.clone();
                match result {
                    PluginResult::Success | PluginResult::Modified(_) => {
                        summary.successful_plugins += 1;
                    }
                    _ => {
                        summary.failed_plugins += 1;
                    }
                }
                summary.plugin_results.insert(plugin_name, result);
            }
        }

        summary.total_execution_time_ms = start_time.elapsed().as_millis() as u64;
        summary
    }

    /// Get plugin information
    pub fn get_plugin_info(&self, name: &str) -> Option<PluginMetadata> {
        self.registry.get_metadata(name)
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.registry.list_metadata()
    }

    /// List plugins by tag
    pub fn list_plugins_by_tag(&self, tag: &str) -> Vec<PluginMetadata> {
        let plugin_names = self.registry.find_by_tag(tag);
        plugin_names
            .into_iter()
            .filter_map(|name| self.registry.get_metadata(&name))
            .collect()
    }

    /// Check plugin dependencies
    pub fn validate_dependencies(&self) -> HashMap<String, PluginResult> {
        let mut results = HashMap::new();
        let plugin_names = self.registry.list_plugins();

        for name in plugin_names {
            let result = self.registry.check_dependencies(&name);
            results.insert(name, result);
        }

        results
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> PluginResult {
        self.loader.unload_plugin(name)
    }

    /// Reload a plugin
    pub fn reload_plugin(&mut self, name: &str) -> PluginResult {
        let load_config = crate::plugins::loader::LoadConfig::default();
        self.loader.reload_plugin(name, &load_config)
    }

    /// Set execution configuration
    pub fn set_execution_config(&mut self, config: ExecutionConfig) {
        self.execution_config = config;
    }

    /// Get execution configuration
    pub fn get_execution_config(&self) -> &ExecutionConfig {
        &self.execution_config
    }

    /// Get plugin registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Get plugin loader
    pub fn loader(&self) -> &PluginLoader {
        &self.loader
    }

    /// Get mutable plugin loader
    pub fn loader_mut(&mut self) -> &mut PluginLoader {
        &mut self.loader
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for plugin execution
pub mod convenience {
    use super::*;

    /// Execute a single plugin on a document
    pub fn execute_plugin(plugin: Arc<dyn AstPlugin>, document: &mut PdfDocument) -> PluginResult {
        let mut context = PluginContext::new()
            .with_document(document)
            .with_graph(&mut document.ast);

        plugin.process_document(document, &mut context)
    }

    /// Execute multiple plugins in sequence
    pub fn execute_plugins_sequence(
        plugins: Vec<Arc<dyn AstPlugin>>,
        document: &mut PdfDocument,
    ) -> Vec<PluginResult> {
        let mut results = Vec::new();
        let mut context = PluginContext::new()
            .with_document(document)
            .with_graph(&mut document.ast);

        for plugin in plugins {
            let result = plugin.process_document(document, &mut context);
            results.push(result);
        }

        results
    }

    /// Create a simple plugin manager with built-in plugins
    pub fn create_default_manager() -> PluginManager {
        let mut manager = PluginManager::new();

        // Load built-in plugins
        let basic_validator = Arc::new(super::loader::BasicValidatorPlugin::new());
        let basic_transformer = Arc::new(super::loader::BasicTransformerPlugin::new());

        let _ = manager.register_plugin(basic_validator);
        let _ = manager.register_plugin(basic_transformer);

        manager
    }
}

/// Plugin execution errors
#[derive(Debug, Clone)]
pub enum PluginExecutionError {
    PluginNotFound(String),
    DependencyError(String),
    ExecutionTimeout,
    MemoryLimitExceeded,
    PluginError(String),
}

impl std::fmt::Display for PluginExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginExecutionError::PluginNotFound(name) => {
                write!(f, "Plugin not found: {}", name)
            }
            PluginExecutionError::DependencyError(msg) => {
                write!(f, "Dependency error: {}", msg)
            }
            PluginExecutionError::ExecutionTimeout => {
                write!(f, "Plugin execution timeout")
            }
            PluginExecutionError::MemoryLimitExceeded => {
                write!(f, "Plugin memory limit exceeded")
            }
            PluginExecutionError::PluginError(msg) => {
                write!(f, "Plugin error: {}", msg)
            }
        }
    }
}

impl std::error::Error for PluginExecutionError {}
