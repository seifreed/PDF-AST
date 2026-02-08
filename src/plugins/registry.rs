use super::*;
use crate::ast::NodeType;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Plugin registry for managing registered plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn AstPlugin>>>>,
    plugin_metadata: Arc<RwLock<HashMap<String, PluginMetadata>>>,
    type_mappings: Arc<RwLock<HashMap<NodeType, Vec<String>>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_metadata: Arc::new(RwLock::new(HashMap::new())),
            type_mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin
    pub fn register(&self, plugin: Arc<dyn AstPlugin>) -> PluginResult {
        let metadata = plugin.metadata().clone();
        let name = metadata.name.clone();

        // Check for duplicate names
        {
            let plugins = self.plugins.read().unwrap();
            if plugins.contains_key(&name) {
                return PluginResult::Error(format!("Plugin '{}' already registered", name));
            }
        }

        // Register plugin
        {
            let mut plugins = self.plugins.write().unwrap();
            plugins.insert(name.clone(), plugin.clone());
        }

        // Store metadata
        {
            let mut plugin_metadata = self.plugin_metadata.write().unwrap();
            plugin_metadata.insert(name.clone(), metadata.clone());
        }

        // Update type mappings
        {
            let mut type_mappings = self.type_mappings.write().unwrap();
            for node_type_str in &metadata.supported_node_types {
                if let Ok(node_type) = self.parse_node_type(node_type_str) {
                    type_mappings
                        .entry(node_type)
                        .or_default()
                        .push(name.clone());
                }
            }
        }

        PluginResult::Success
    }

    /// Unregister a plugin
    pub fn unregister(&self, name: &str) -> PluginResult {
        // Remove from main registry
        let plugin_existed = {
            let mut plugins = self.plugins.write().unwrap();
            plugins.remove(name).is_some()
        };

        if !plugin_existed {
            return PluginResult::Error(format!("Plugin '{}' not found", name));
        }

        // Remove metadata
        let metadata = {
            let mut plugin_metadata = self.plugin_metadata.write().unwrap();
            plugin_metadata.remove(name)
        };

        // Update type mappings
        if let Some(metadata) = metadata {
            let mut type_mappings = self.type_mappings.write().unwrap();
            for node_type_str in &metadata.supported_node_types {
                if let Ok(node_type) = self.parse_node_type(node_type_str) {
                    if let Some(plugin_names) = type_mappings.get_mut(&node_type) {
                        plugin_names.retain(|n| n != name);
                        if plugin_names.is_empty() {
                            type_mappings.remove(&node_type);
                        }
                    }
                }
            }
        }

        PluginResult::Success
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn AstPlugin>> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(name).cloned()
    }

    /// Get all registered plugin names
    pub fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().unwrap();
        plugins.keys().cloned().collect()
    }

    /// Get plugins that can process a specific node type
    pub fn get_plugins_for_type(&self, node_type: &NodeType) -> Vec<Arc<dyn AstPlugin>> {
        let type_mappings = self.type_mappings.read().unwrap();
        let plugins = self.plugins.read().unwrap();

        if let Some(plugin_names) = type_mappings.get(node_type) {
            plugin_names
                .iter()
                .filter_map(|name| plugins.get(name).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        let plugin_metadata = self.plugin_metadata.read().unwrap();
        plugin_metadata.get(name).cloned()
    }

    /// Get all plugin metadata
    pub fn list_metadata(&self) -> Vec<PluginMetadata> {
        let plugin_metadata = self.plugin_metadata.read().unwrap();
        plugin_metadata.values().cloned().collect()
    }

    /// Find plugins by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<String> {
        let plugin_metadata = self.plugin_metadata.read().unwrap();
        plugin_metadata
            .iter()
            .filter(|(_, metadata)| metadata.tags.contains(&tag.to_string()))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Find plugins by author
    pub fn find_by_author(&self, author: &str) -> Vec<String> {
        let plugin_metadata = self.plugin_metadata.read().unwrap();
        plugin_metadata
            .iter()
            .filter(|(_, metadata)| metadata.author == author)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Check plugin dependencies
    pub fn check_dependencies(&self, name: &str) -> PluginResult {
        let plugin_metadata = self.plugin_metadata.read().unwrap();

        if let Some(metadata) = plugin_metadata.get(name) {
            for dependency in &metadata.dependencies {
                if !plugin_metadata.contains_key(dependency) {
                    return PluginResult::Error(format!(
                        "Plugin '{}' depends on '{}' which is not registered",
                        name, dependency
                    ));
                }
            }
            PluginResult::Success
        } else {
            PluginResult::Error(format!("Plugin '{}' not found", name))
        }
    }

    /// Get dependency graph for a plugin
    pub fn get_dependency_order(&self, plugin_names: &[String]) -> Result<Vec<String>, String> {
        let plugin_metadata = self.plugin_metadata.read().unwrap();
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        fn visit(
            name: &str,
            plugin_metadata: &HashMap<String, PluginMetadata>,
            result: &mut Vec<String>,
            visited: &mut std::collections::HashSet<String>,
            visiting: &mut std::collections::HashSet<String>,
        ) -> Result<(), String> {
            if visiting.contains(name) {
                return Err(format!("Circular dependency detected involving '{}'", name));
            }

            if visited.contains(name) {
                return Ok(());
            }

            visiting.insert(name.to_string());

            if let Some(metadata) = plugin_metadata.get(name) {
                for dependency in &metadata.dependencies {
                    visit(dependency, plugin_metadata, result, visited, visiting)?;
                }
            }

            visiting.remove(name);
            visited.insert(name.to_string());
            result.push(name.to_string());

            Ok(())
        }

        for name in plugin_names {
            visit(
                name,
                &plugin_metadata,
                &mut result,
                &mut visited,
                &mut visiting,
            )?;
        }

        Ok(result)
    }

    /// Clear all plugins
    pub fn clear(&self) {
        let mut plugins = self.plugins.write().unwrap();
        plugins.clear();

        let mut plugin_metadata = self.plugin_metadata.write().unwrap();
        plugin_metadata.clear();

        let mut type_mappings = self.type_mappings.write().unwrap();
        type_mappings.clear();
    }

    /// Get plugin count
    pub fn count(&self) -> usize {
        let plugins = self.plugins.read().unwrap();
        plugins.len()
    }

    /// Helper function to parse node type from string
    fn parse_node_type(&self, type_str: &str) -> Result<NodeType, String> {
        match type_str {
            "Catalog" => Ok(NodeType::Catalog),
            "Pages" => Ok(NodeType::Pages),
            "Page" => Ok(NodeType::Page),
            "ContentStream" => Ok(NodeType::ContentStream),
            "Font" => Ok(NodeType::Font),
            "Type1Font" => Ok(NodeType::Type1Font),
            "TrueTypeFont" => Ok(NodeType::TrueTypeFont),
            "Type3Font" => Ok(NodeType::Type3Font),
            "Image" => Ok(NodeType::Image),
            "Annotation" => Ok(NodeType::Annotation),
            "Outline" => Ok(NodeType::Outline),
            "Action" => Ok(NodeType::Action),
            "Encryption" => Ok(NodeType::Encryption),
            "Metadata" => Ok(NodeType::Metadata),
            "Structure" => Ok(NodeType::Structure),
            "Form" => Ok(NodeType::Form),
            "JavaScript" => Ok(NodeType::JavaScript),
            "Multimedia" => Ok(NodeType::Multimedia),
            "ColorSpace" => Ok(NodeType::ColorSpace),
            "Pattern" => Ok(NodeType::Pattern),
            "Shading" => Ok(NodeType::Shading),
            "XObject" => Ok(NodeType::XObject),
            "EmbeddedFile" => Ok(NodeType::EmbeddedFile),
            "Other" => Ok(NodeType::Other),
            _ => Err(format!("Unknown node type: {}", type_str)),
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PluginRegistry {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            plugin_metadata: Arc::clone(&self.plugin_metadata),
            type_mappings: Arc::clone(&self.type_mappings),
        }
    }
}
