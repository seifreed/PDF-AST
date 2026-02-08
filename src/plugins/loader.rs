use super::*;
use crate::plugins::registry::PluginRegistry;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Plugin loader for loading plugins from various sources
pub struct PluginLoader {
    registry: Arc<PluginRegistry>,
    search_paths: Vec<PathBuf>,
    loaded_plugins: std::collections::HashMap<String, LoadedPlugin>,
}

/// Information about a loaded plugin
#[derive(Clone)]
pub struct LoadedPlugin {
    pub name: String,
    pub path: Option<PathBuf>,
    pub metadata: PluginMetadata,
    pub load_time: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    library: Option<std::sync::Arc<LibraryHandle>>,
}

/// Plugin loading configuration
#[derive(Debug, Clone)]
pub struct LoadConfig {
    pub auto_resolve_dependencies: bool,
    pub allow_version_conflicts: bool,
    pub validate_signatures: bool,
    pub sandbox_mode: bool,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            auto_resolve_dependencies: true,
            allow_version_conflicts: false,
            validate_signatures: false,
            sandbox_mode: true,
        }
    }
}

impl PluginLoader {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self {
            registry,
            search_paths: Vec::new(),
            loaded_plugins: std::collections::HashMap::new(),
        }
    }

    /// Add a search path for plugins
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// Load plugin from memory (for built-in plugins)
    pub fn load_plugin(&mut self, plugin: Arc<dyn AstPlugin>) -> PluginResult {
        let metadata = plugin.metadata().clone();
        let name = metadata.name.clone();

        // Register with registry
        match self.registry.register(plugin) {
            PluginResult::Success => {
                // Track loaded plugin
                let loaded_plugin = LoadedPlugin {
                    name: name.clone(),
                    path: None,
                    metadata,
                    load_time: chrono::Utc::now(),
                    library: None,
                };
                self.loaded_plugins.insert(name, loaded_plugin);
                PluginResult::Success
            }
            result => result,
        }
    }

    /// Load plugin from configuration
    pub fn load_from_config(
        &mut self,
        config: &serde_json::Value,
        load_config: &LoadConfig,
    ) -> PluginResult {
        // Parse plugin configuration
        let plugin_name = match config.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return PluginResult::Error("Missing plugin name in config".to_string()),
        };

        let plugin_type = config
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("dynamic");

        match plugin_type {
            "builtin" => self.load_builtin_plugin(plugin_name, config, load_config),
            "dynamic" => self.load_dynamic_plugin(plugin_name, config, load_config),
            _ => PluginResult::Error(format!("Unknown plugin type: {}", plugin_type)),
        }
    }

    /// Load multiple plugins with dependency resolution
    pub fn load_plugins(
        &mut self,
        configs: &[serde_json::Value],
        load_config: &LoadConfig,
    ) -> Vec<PluginResult> {
        let mut results = Vec::new();

        if load_config.auto_resolve_dependencies {
            // Extract plugin names and resolve dependencies
            let plugin_names: Vec<String> = configs
                .iter()
                .filter_map(|config| {
                    config
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            match self.registry.get_dependency_order(&plugin_names) {
                Ok(ordered_names) => {
                    // Load plugins in dependency order
                    for name in ordered_names {
                        if let Some(config) = configs.iter().find(|c| {
                            c.get("name")
                                .and_then(|v| v.as_str())
                                .map(|s| s == name)
                                .unwrap_or(false)
                        }) {
                            let result = self.load_from_config(config, load_config);
                            results.push(result);
                        }
                    }
                }
                Err(err) => {
                    results.push(PluginResult::Error(err));
                }
            }
        } else {
            // Load plugins in order provided
            for config in configs {
                let result = self.load_from_config(config, load_config);
                results.push(result);
            }
        }

        results
    }

    /// Discover plugins in search paths
    pub fn discover_plugins(&self) -> Vec<PathBuf> {
        let mut discovered = Vec::new();

        for search_path in &self.search_paths {
            if let Ok(entries) = fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Look for plugin manifests or libraries
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        // Plugin manifest
                        discovered.push(path);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("so")
                        || path.extension().and_then(|s| s.to_str()) == Some("dll")
                        || path.extension().and_then(|s| s.to_str()) == Some("dylib")
                    {
                        // Dynamic library
                        discovered.push(path);
                    }
                }
            }
        }

        discovered
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> PluginResult {
        // Remove from registry
        let registry_result = self.registry.unregister(name);

        // Remove from loaded plugins tracking
        self.loaded_plugins.remove(name);

        registry_result
    }

    /// Get information about loaded plugins
    pub fn get_loaded_plugins(&self) -> Vec<LoadedPlugin> {
        self.loaded_plugins.values().cloned().collect()
    }

    /// Check if a plugin is loaded
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded_plugins.contains_key(name)
    }

    /// Reload a plugin
    pub fn reload_plugin(&mut self, name: &str, load_config: &LoadConfig) -> PluginResult {
        // Get current plugin info
        let loaded_plugin = self.loaded_plugins.get(name).cloned();

        if let Some(plugin_info) = loaded_plugin {
            // Unload current plugin
            let unload_result = self.unload_plugin(name);
            if let PluginResult::Error(msg) = unload_result {
                return PluginResult::Error(msg);
            }

            // Try to reload from original path or config
            if let Some(path) = plugin_info.path {
                // Reload from file
                self.load_from_path(&path, load_config)
            } else {
                PluginResult::Error("Cannot reload in-memory plugin without source".to_string())
            }
        } else {
            PluginResult::Error(format!("Plugin '{}' is not loaded", name))
        }
    }

    /// Load plugin from file path
    pub fn load_from_path(&mut self, path: &Path, load_config: &LoadConfig) -> PluginResult {
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => self.load_from_manifest(path, load_config),
            Some("so") | Some("dll") | Some("dylib") => {
                self.load_dynamic_library(path, load_config)
            }
            _ => PluginResult::Error(format!("Unsupported plugin file: {:?}", path)),
        }
    }

    /// Load plugin from manifest file
    fn load_from_manifest(&mut self, path: &Path, load_config: &LoadConfig) -> PluginResult {
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(config) => self.load_from_config(&config, load_config),
                Err(err) => PluginResult::Error(format!("Invalid plugin manifest: {}", err)),
            },
            Err(err) => PluginResult::Error(format!("Failed to read manifest: {}", err)),
        }
    }

    /// Load built-in plugin
    fn load_builtin_plugin(
        &mut self,
        name: &str,
        config: &serde_json::Value,
        _load_config: &LoadConfig,
    ) -> PluginResult {
        // Create built-in plugin instance based on name
        let plugin: Arc<dyn AstPlugin> = match name {
            "basic_validator" => Arc::new(BasicValidatorPlugin::new()),
            "basic_transformer" => Arc::new(BasicTransformerPlugin::new()),
            "structure_analyzer" => Arc::new(StructureAnalyzerPlugin::new()),
            "security_scanner" => Arc::new(SecurityScannerPlugin::new()),
            "metadata_extractor" => Arc::new(MetadataExtractorPlugin::new()),
            _ => return PluginResult::Error(format!("Unknown built-in plugin: {}", name)),
        };

        // Apply configuration if provided
        if let Some(params) = config.get("parameters") {
            // Store parameters in plugin context for runtime use
            let _ = params;
        }

        self.load_plugin(plugin)
    }

    /// Load dynamic plugin from library
    fn load_dynamic_plugin(
        &mut self,
        name: &str,
        config: &serde_json::Value,
        load_config: &LoadConfig,
    ) -> PluginResult {
        // Get library path from config
        let lib_path = match config.get("path").and_then(|v| v.as_str()) {
            Some(path) => PathBuf::from(path),
            None => {
                // Search for library in search paths
                if let Some(path) = self.find_plugin_library(name) {
                    path
                } else {
                    return PluginResult::Error(format!(
                        "Cannot find library for plugin: {}",
                        name
                    ));
                }
            }
        };

        self.load_dynamic_library(&lib_path, load_config)
    }

    /// Load dynamic library
    fn load_dynamic_library(&mut self, path: &Path, load_config: &LoadConfig) -> PluginResult {
        // Verify library exists
        if !path.exists() {
            return PluginResult::Error(format!("Library not found: {:?}", path));
        }

        // Validate library signature if required
        if load_config.validate_signatures && !self.validate_library_signature(path) {
            return PluginResult::Error(format!("Invalid library signature: {:?}", path));
        }

        // Load library using platform-specific loader
        match DynamicLibraryLoader::load(path, load_config.sandbox_mode) {
            Ok(library) => {
                let library_handle = library.handle();
                // Get plugin factory function
                match library.get_plugin_factory() {
                    Ok(plugin) => {
                        // Use plugin instance
                        let metadata = plugin.metadata().clone();
                        let name = metadata.name.clone();

                        // Store library handle
                        let loaded_plugin = LoadedPlugin {
                            name: name.clone(),
                            path: Some(path.to_path_buf()),
                            metadata,
                            load_time: chrono::Utc::now(),
                            library: Some(library_handle),
                        };

                        // Register plugin
                        self.loaded_plugins.insert(name, loaded_plugin);
                        self.load_plugin(plugin)
                    }
                    Err(err) => {
                        PluginResult::Error(format!("Failed to get plugin factory: {}", err))
                    }
                }
            }
            Err(err) => PluginResult::Error(format!("Failed to load library: {}", err)),
        }
    }

    /// Find plugin library in search paths
    fn find_plugin_library(&self, name: &str) -> Option<PathBuf> {
        let lib_extensions = if cfg!(target_os = "windows") {
            vec!["dll"]
        } else if cfg!(target_os = "macos") {
            vec!["dylib"]
        } else {
            vec!["so"]
        };

        for search_path in &self.search_paths {
            for ext in &lib_extensions {
                let lib_name = format!("lib{}.{}", name, ext);
                let lib_path = search_path.join(&lib_name);
                if lib_path.exists() {
                    return Some(lib_path);
                }

                // Also try without "lib" prefix
                let lib_name = format!("{}.{}", name, ext);
                let lib_path = search_path.join(&lib_name);
                if lib_path.exists() {
                    return Some(lib_path);
                }
            }
        }

        None
    }

    /// Validate library signature
    fn validate_library_signature(&self, path: &Path) -> bool {
        // In production, would verify cryptographic signature
        // For now, just check if file is readable
        path.exists() && path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_dynamic_library() {
        let registry = Arc::new(PluginRegistry::new());
        let mut loader = PluginLoader::new(registry);
        let load_config = LoadConfig::default();
        let path = Path::new("nonexistent_plugin.so");
        let result = loader.load_from_path(path, &load_config);
        assert!(matches!(result, PluginResult::Error(_)));
    }
}

/// Built-in plugin factories
pub struct BuiltinPlugins;

impl BuiltinPlugins {
    /// Create a simple validation plugin
    pub fn create_basic_validator() -> Box<dyn AstPlugin> {
        Box::new(BasicValidatorPlugin::new())
    }

    /// Create a simple transformer plugin
    pub fn create_basic_transformer() -> Box<dyn AstPlugin> {
        Box::new(BasicTransformerPlugin::new())
    }

    /// Get list of available built-in plugins
    pub fn list_available() -> Vec<String> {
        vec![
            "basic_validator".to_string(),
            "basic_transformer".to_string(),
        ]
    }
}

/// Example basic validator plugin
pub struct BasicValidatorPlugin {
    metadata: PluginMetadata,
}

impl Default for BasicValidatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicValidatorPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "basic_validator",
                "1.0.0",
                "Basic document validation plugin",
                "PDF-AST",
            )
            .with_tags(vec!["validation", "builtin"])
            .with_supported_types(vec![NodeType::Catalog, NodeType::Pages]),
        }
    }
}

impl AstPlugin for BasicValidatorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn process_document(
        &self,
        document: &mut crate::ast::PdfDocument,
        _context: &mut PluginContext,
    ) -> PluginResult {
        // Basic validation: check if document has catalog
        if document.ast.get_root().is_none() {
            PluginResult::Error("Document missing root catalog".to_string())
        } else {
            PluginResult::Success
        }
    }

    fn clone_plugin(&self) -> Box<dyn AstPlugin> {
        Box::new(Self::new())
    }
}

/// Example basic transformer plugin
pub struct BasicTransformerPlugin {
    metadata: PluginMetadata,
}

impl Default for BasicTransformerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicTransformerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "basic_transformer",
                "1.0.0",
                "Basic document transformation plugin",
                "PDF-AST",
            )
            .with_tags(vec!["transformation", "builtin"]),
        }
    }
}

impl AstPlugin for BasicTransformerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            can_modify_nodes: true,
            can_add_nodes: false,
            can_remove_nodes: false,
            can_validate: false,
            can_transform: true,
            requires_document_context: false,
            thread_safe: true,
        }
    }

    fn clone_plugin(&self) -> Box<dyn AstPlugin> {
        Box::new(Self::new())
    }
}

/// Structure analyzer plugin
pub struct StructureAnalyzerPlugin {
    metadata: PluginMetadata,
}

impl Default for StructureAnalyzerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureAnalyzerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "structure_analyzer",
                "1.0.0",
                "Analyzes document structure and complexity",
                "PDF-AST",
            )
            .with_tags(vec!["analysis", "builtin"]),
        }
    }
}

impl AstPlugin for StructureAnalyzerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn process_document(
        &self,
        document: &mut crate::ast::PdfDocument,
        context: &mut PluginContext,
    ) -> PluginResult {
        // Analyze document structure
        let node_count = document.ast.get_all_nodes().len();
        let depth = document.ast.get_max_depth();

        // Store analysis results in context
        context.set_data("node_count", node_count.to_string());
        context.set_data("max_depth", depth.to_string());

        // Check for complexity issues
        if node_count > 10000 {
            context.add_warning("Document has high complexity (>10000 nodes)");
        }

        if depth > 50 {
            context.add_warning("Document has deep nesting (>50 levels)");
        }

        PluginResult::Success
    }

    fn clone_plugin(&self) -> Box<dyn AstPlugin> {
        Box::new(Self::new())
    }
}

/// Security scanner plugin
pub struct SecurityScannerPlugin {
    metadata: PluginMetadata,
}

impl Default for SecurityScannerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityScannerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "security_scanner",
                "1.0.0",
                "Scans for security issues and suspicious patterns",
                "PDF-AST",
            )
            .with_tags(vec!["security", "builtin"])
            .with_supported_types(vec![
                NodeType::JavaScriptAction,
                NodeType::LaunchAction,
                NodeType::URIAction,
                NodeType::EmbeddedFile,
            ]),
        }
    }
}

impl AstPlugin for SecurityScannerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn process_node(
        &self,
        node: &mut crate::ast::AstNode,
        context: &mut PluginContext,
    ) -> PluginResult {
        match node.node_type {
            NodeType::JavaScriptAction | NodeType::EmbeddedJS => {
                context.add_warning(&format!("JavaScript detected at node {:?}", node.id));
            }
            NodeType::LaunchAction => {
                context.add_warning(&format!("Launch action detected at node {:?}", node.id));
            }
            NodeType::URIAction => {
                if let Some(uri) = node.metadata.get_property("URI") {
                    if uri.starts_with("http://") {
                        context.add_warning(&format!("Insecure HTTP URI at node {:?}", node.id));
                    }
                }
            }
            NodeType::EmbeddedFile => {
                context.add_info(format!("Embedded file detected at node {:?}", node.id));
            }
            _ => {}
        }

        PluginResult::Success
    }

    fn clone_plugin(&self) -> Box<dyn AstPlugin> {
        Box::new(Self::new())
    }
}

/// Metadata extractor plugin
pub struct MetadataExtractorPlugin {
    metadata: PluginMetadata,
}

impl Default for MetadataExtractorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataExtractorPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "metadata_extractor",
                "1.0.0",
                "Extracts and consolidates document metadata",
                "PDF-AST",
            )
            .with_tags(vec!["metadata", "builtin"])
            .with_supported_types(vec![NodeType::Metadata, NodeType::Catalog]),
        }
    }
}

impl AstPlugin for MetadataExtractorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn process_document(
        &self,
        document: &mut crate::ast::PdfDocument,
        context: &mut PluginContext,
    ) -> PluginResult {
        // Extract document metadata
        context.set_data("title", document.metadata.title.clone().unwrap_or_default());
        context.set_data(
            "author",
            document.metadata.author.clone().unwrap_or_default(),
        );
        context.set_data(
            "subject",
            document.metadata.subject.clone().unwrap_or_default(),
        );
        context.set_data(
            "creator",
            document.metadata.creator.clone().unwrap_or_default(),
        );
        context.set_data(
            "producer",
            document.metadata.producer.clone().unwrap_or_default(),
        );

        if let Some(created) = &document.metadata.creation_date {
            context.set_data("creation_date", created.to_string());
        }

        if let Some(modified) = &document.metadata.modification_date {
            context.set_data("modification_date", modified.to_string());
        }

        context.set_data("page_count", document.metadata.page_count.to_string());
        context.set_data("encrypted", document.metadata.encrypted.to_string());

        PluginResult::Success
    }

    fn clone_plugin(&self) -> Box<dyn AstPlugin> {
        Box::new(Self::new())
    }
}

/// Dynamic library loader
struct DynamicLibraryLoader {
    handle: std::sync::Arc<LibraryHandle>,
}

impl DynamicLibraryLoader {
    /// Load a dynamic library
    fn load(path: &Path, sandbox: bool) -> Result<Self, String> {
        // Platform-specific library loading
        let handle = LibraryHandle::load(path, sandbox)?;
        Ok(Self {
            handle: std::sync::Arc::new(handle),
        })
    }

    /// Get plugin factory function from library
    fn get_plugin_factory(&self) -> Result<Arc<dyn AstPlugin>, String> {
        self.handle.get_plugin_factory()
    }

    fn handle(&self) -> std::sync::Arc<LibraryHandle> {
        std::sync::Arc::clone(&self.handle)
    }
}

/// Platform-specific library handle
#[allow(dead_code)]
enum LibraryHandle {
    #[cfg(unix)]
    Unix(UnixLibrary),
    #[cfg(windows)]
    Windows(WindowsLibrary),
    Stub,
}

impl LibraryHandle {
    fn load(path: &Path, sandbox: bool) -> Result<Self, String> {
        // Check if file exists
        if !path.exists() {
            return Err(format!("Library file not found: {:?}", path));
        }

        #[cfg(unix)]
        {
            UnixLibrary::load(path, sandbox).map(LibraryHandle::Unix)
        }

        #[cfg(windows)]
        {
            WindowsLibrary::load(path, sandbox).map(LibraryHandle::Windows)
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = (path, sandbox);
            Ok(LibraryHandle::Stub)
        }
    }

    fn get_plugin_factory(&self) -> Result<Arc<dyn AstPlugin>, String> {
        match self {
            #[cfg(unix)]
            LibraryHandle::Unix(lib) => lib.get_plugin_factory(),
            #[cfg(windows)]
            LibraryHandle::Windows(lib) => lib.get_plugin_factory(),
            LibraryHandle::Stub => {
                Err("Dynamic loading not supported on this platform".to_string())
            }
        }
    }
}

#[cfg(unix)]
struct UnixLibrary {
    _handle: *mut std::ffi::c_void,
}

#[cfg(unix)]
impl UnixLibrary {
    fn load(path: &Path, sandbox: bool) -> Result<Self, String> {
        use std::ffi::CString;

        // Convert path to C string
        let path_str = path.to_str().ok_or("Invalid path")?;
        let c_path = CString::new(path_str).map_err(|e| e.to_string())?;

        // Load library with dlopen
        let flags = if sandbox {
            0x0002 | 0x0100 // RTLD_NOW | RTLD_LOCAL
        } else {
            0x0002 // RTLD_NOW
        };

        unsafe {
            let handle = libc::dlopen(c_path.as_ptr(), flags);
            if handle.is_null() {
                let error = std::ffi::CStr::from_ptr(libc::dlerror());
                return Err(format!("Failed to load library: {:?}", error));
            }

            Ok(Self { _handle: handle })
        }
    }

    fn get_plugin_factory(&self) -> Result<Arc<dyn AstPlugin>, String> {
        use std::ffi::{CStr, CString};

        type PluginFactory = unsafe extern "C" fn() -> *mut std::ffi::c_void;

        let symbol_name = CString::new("pdf_ast_plugin_factory")
            .map_err(|e| format!("Invalid symbol name: {}", e))?;

        unsafe {
            let symbol = libc::dlsym(self._handle, symbol_name.as_ptr());
            if symbol.is_null() {
                let error = libc::dlerror();
                let message = if error.is_null() {
                    "Failed to resolve symbol".to_string()
                } else {
                    CStr::from_ptr(error).to_string_lossy().into_owned()
                };
                return Err(format!("Failed to resolve plugin factory: {}", message));
            }

            let factory: PluginFactory = std::mem::transmute(symbol);
            let plugin_ptr = factory();
            if plugin_ptr.is_null() {
                return Err("Plugin factory returned null".to_string());
            }
            let boxed_plugin = Box::from_raw(plugin_ptr as *mut Box<dyn AstPlugin>);
            Ok(Arc::from(*boxed_plugin))
        }
    }
}

#[cfg(unix)]
impl Drop for UnixLibrary {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self._handle);
        }
    }
}

#[cfg(windows)]
struct WindowsLibrary {
    _handle: *mut std::ffi::c_void,
}

#[cfg(windows)]
impl WindowsLibrary {
    fn load(path: &Path, _sandbox: bool) -> Result<Self, String> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        // Convert path to wide string
        let wide_path: Vec<u16> = OsStr::new(path.as_os_str())
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            let handle = LoadLibraryW(wide_path.as_ptr());
            if handle.is_null() {
                return Err("Failed to load library".to_string());
            }

            Ok(Self { _handle: handle })
        }
    }

    fn get_plugin_factory(&self) -> Result<Arc<dyn AstPlugin>, String> {
        use std::ffi::CString;

        type PluginFactory = unsafe extern "C" fn() -> *mut std::ffi::c_void;

        let symbol_name = CString::new("pdf_ast_plugin_factory")
            .map_err(|e| format!("Invalid symbol name: {}", e))?;

        unsafe {
            let symbol = GetProcAddress(self._handle, symbol_name.as_ptr());
            if symbol.is_null() {
                return Err("Failed to resolve plugin factory".to_string());
            }
            let factory: PluginFactory = std::mem::transmute(symbol);
            let plugin_ptr = factory();
            if plugin_ptr.is_null() {
                return Err("Plugin factory returned null".to_string());
            }
            let boxed_plugin = Box::from_raw(plugin_ptr as *mut Box<dyn AstPlugin>);
            Ok(Arc::from(*boxed_plugin))
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsLibrary {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self._handle);
        }
    }
}

// FFI declarations for Unix
#[cfg(unix)]
#[allow(dead_code)]
extern "C" {
    fn dlopen(filename: *const std::ffi::c_char, flag: std::ffi::c_int) -> *mut std::ffi::c_void;
    fn dlerror() -> *const std::ffi::c_char;
}

// FFI declarations for Windows
#[cfg(windows)]
extern "system" {
    fn LoadLibraryW(lpFileName: *const u16) -> *mut std::ffi::c_void;
    fn GetProcAddress(
        hModule: *mut std::ffi::c_void,
        lpProcName: *const i8,
    ) -> *mut std::ffi::c_void;
    fn FreeLibrary(hModule: *mut std::ffi::c_void) -> i32;
}
